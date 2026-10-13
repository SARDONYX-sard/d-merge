//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
mod priority_ids;
pub(crate) mod tasks;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rayon::prelude::*;
pub use tasks::templates::gen_bin::create_bin_templates;
pub(crate) use tasks::{
    adsf::path_parser::ParseError as AsdfPathParseError,
    asdsf::path_parser::ParseError as AsdsfPathParseError,
};

pub use crate::behaviors::priority_ids::types::{PatchMaps, PriorityMap};
use crate::{
    behaviors::tasks::{
        adsf::apply_adsf_patches,
        asdsf::apply_asdsf_patches,
        fnis::{self, collect::owned::OwnedFnisInjection},
        hkx::generate::generate_hkx_files,
        patches::{
            apply::apply_patches,
            collect::{collect_borrowed_patches, collect_owned_patches, template_keys_from_paths},
            types::{OwnedPatchMap, OwnedPatches, PatchCollection},
        },
        templates::collect::{borrowed, owned},
    },
    config::{Config, Status},
    errors::{BehaviorGenerationError, Error, Result, writer::write_errors},
};

/// - `resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
///
/// # Cancellation
/// The CPU heavy part runs on a blocking thread(`spawn_blocking`).
/// If this future is dropped(e.g. the task is aborted by the GUI), the blocking work stops at the next check point.
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub async fn behavior_gen(patches: PatchMaps, config: Config) -> Result<()> {
    #[cfg(feature = "tracing")]
    {
        let PatchMaps { nemesis_entries, fnis_entries } = &patches;
        tracing::trace!("nemesis_entries = {:#?}", {
            let mut sorted: Vec<_> = nemesis_entries.par_iter().collect();
            sorted.par_sort_by_key(|&(_, v)| *v);
            sorted
        });
        tracing::trace!("fnis_entries = {:#?}", {
            let mut sorted: Vec<_> = fnis_entries.par_iter().collect();
            sorted.par_sort_by_key(|&(_, v)| *v);
            sorted
        });
    }

    let cancel = CancelOnDrop::default();
    let patches = Arc::new(patches);
    let config = Arc::new(config);

    // Collect FNIS(async I/O) and Nemesis(blocking I/O) patches concurrently.
    let nemesis_task = tokio::task::spawn_blocking({
        let patches = Arc::clone(&patches);
        let config = Arc::clone(&config);
        move || collect_owned_patches(&patches.nemesis_entries, &config)
    });
    let fnis_future = async {
        if patches.fnis_entries.is_empty() {
            return Ok::<_, Error>((vec![], vec![]));
        }
        let skyrim_data_dir_glob =
            config.skyrim_data_dir_glob.as_ref().ok_or(Error::MissingSkyrimDataDirGlob)?;
        Ok(fnis::collect::collect_all_fnis_injections(skyrim_data_dir_glob, &patches.fnis_entries)
            .await)
    };
    // NOTE: `spawn_blocking` starts immediately, so the nemesis collection runs while awaiting FNIS.
    let fnis_result = fnis_future.await;
    let owned_patches = nemesis_task.await?;
    let (owned_fnis_patches, fnis_errors) = fnis_result?;

    let result = tokio::task::spawn_blocking({
        let patches = Arc::clone(&patches);
        let config = Arc::clone(&config);
        let cancel = Arc::clone(&cancel.0);
        move || {
            gen_blocking(
                &patches,
                &config,
                &owned_fnis_patches,
                fnis_errors,
                owned_patches,
                &cancel,
            )
        }
    })
    .await?;

    // Error process
    if let Err((err, all_errors)) = result {
        config.on_report_status(Status::Error(err.to_string()));

        write_errors(&config, &all_errors).await?;
        return Err(Error::FailedToGenerateBehaviors { source: err });
    }

    config.on_report_status(Status::Done);
    Ok(())
}

/// Sets the cancel flag when dropped.
///
/// The blocking work cannot be aborted by `JoinHandle::abort`, so it checks this flag instead.
#[derive(Debug, Default)]
struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// Is cancellation requested?
#[inline]
pub(crate) fn is_cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Relaxed)
}

/// The blocking(CPU heavy) part of [`behavior_gen`].
///
/// # Note
/// FNIS patch generation uses I/O internally, so this must not be called inside `rayon::par_iter`.
fn gen_blocking(
    patches: &PatchMaps,
    config: &Config,
    owned_fnis_patches: &[OwnedFnisInjection],
    mut fnis_errors: Vec<Error>,
    owned_patches: OwnedPatches,
    cancel: &AtomicBool,
) -> core::result::Result<(), (BehaviorGenerationError, Vec<Error>)> {
    let (fnis_hkx_patches, fnis_adsf_patches, fnis_asdsf_patches, io_job_runner) = {
        let (fnis_hkx_patches, fnis_adsf_patches, fnis_asdsf_patches, io_job_runner, errors) =
            fnis::patch_gen::collect_borrowed_patches(owned_fnis_patches, config);
        fnis_errors.par_extend(errors);

        (fnis_hkx_patches, fnis_adsf_patches, fnis_asdsf_patches, io_job_runner)
    };

    let OwnedPatches {
        owned_patches,
        adsf_patches: owned_adsf_patches,
        asdsf_patches: owned_asdsf_patches,
        errors: owned_file_errors,
    } = owned_patches;

    if is_cancelled(cancel) {
        return Ok(());
    }

    let mut adsf_errors = vec![];
    let mut asdsf_errors = vec![];
    let mut patched_hkx_errors = None;
    let mut fnis_convert_errors = vec![];

    rayon::scope(|s| {
        s.spawn(|_| fnis_convert_errors = io_job_runner.convert());
        s.spawn(|_| {
            adsf_errors =
                apply_adsf_patches(owned_adsf_patches, patches, config, fnis_adsf_patches);
        });
        s.spawn(|_| {
            asdsf_errors =
                apply_asdsf_patches(owned_asdsf_patches, patches, config, fnis_asdsf_patches);
        });
        s.spawn(|_| {
            patched_hkx_errors =
                Some(apply_and_gen_patched_hkx(&owned_patches, config, fnis_hkx_patches, cancel));
        });
    });

    // Error process
    let Errors { patch_errors_len, apply_errors_len, hkx_errors_len, hkx_errors } =
        patched_hkx_errors.unwrap_or_default();
    let fnis_errors_errors_len = fnis_errors.len() + fnis_convert_errors.len();

    let owned_file_errors_len = owned_file_errors.len();
    let adsf_errors_len = adsf_errors.len();
    let asdsf_errors_len = asdsf_errors.len();

    let all_errors = {
        let mut all_errors = vec![];

        all_errors.par_extend(fnis_errors);
        all_errors.par_extend(fnis_convert_errors);

        all_errors.par_extend(owned_file_errors);
        all_errors.par_extend(adsf_errors);
        all_errors.par_extend(asdsf_errors);
        all_errors.par_extend(hkx_errors);
        all_errors
    };

    if all_errors.is_empty() {
        return Ok(());
    }

    let err = BehaviorGenerationError {
        fnis_errors_errors_len,
        owned_file_errors_len,
        adsf_errors_len,
        asdsf_errors_len,
        patch_errors_len,
        apply_errors_len,
        hkx_errors_len,
    };
    Err((err, all_errors))
}

#[derive(Default)]
struct Errors {
    patch_errors_len: usize,
    apply_errors_len: usize,
    hkx_errors_len: usize,
    hkx_errors: Vec<Error>,
}

fn apply_and_gen_patched_hkx<'a>(
    owned_patches: &'a OwnedPatchMap,
    config: &Config,
    fnis_patches: PatchCollection<'a>,
    cancel: &AtomicBool,
) -> Errors {
    let mut all_errors = vec![];

    // 1/3: Read templates, then decode them while parsing nemesis patches.
    //
    // NOTE: The needed templates are known from the paths alone(no XML parse needed),
    //       so the heavy msgpack decoding of templates can overlap with the patch parsing.
    let mut template_error_len;
    let owned_templates = {
        let mut needed_template_names = template_keys_from_paths(owned_patches);
        needed_template_names
            .extend(fnis_patches.borrowed_patches.0.iter().map(|entry| entry.key().clone()));

        let (owned_templates, errors) =
            owned::collect_templates(&config.resource_dir, needed_template_names);
        template_error_len = errors.len();
        all_errors.par_extend(errors);
        owned_templates
    };

    let ((templates, template_errors), (patch_collection, patch_errors)) = rayon::join(
        || borrowed::collect_templates(&owned_templates),
        || collect_borrowed_patches(owned_patches, config, fnis_patches),
    );
    let PatchCollection { borrowed_patches, behavior_graph_data_map: variable_class_map } =
        patch_collection;

    let patch_errors_len = patch_errors.len();
    all_errors.par_extend(patch_errors);

    let mut templates = templates;
    template_error_len += template_errors.len();
    all_errors.par_extend(template_errors);

    // Templates whose patches all failed to parse are not output(same as before).
    templates.retain(|key, _| borrowed_patches.0.contains_key(key));

    #[cfg(feature = "tracing")]
    tracing::debug!(
        "owned_templates_keys(Things that actually exist) = {:#?}",
        owned_templates.keys()
    );

    if is_cancelled(cancel) {
        return Errors::default();
    }

    // 2/3: Apply patches & Replace variables to indexes
    let mut apply_errors_len = template_error_len;
    if let Err(errors) = apply_patches(&mut templates, borrowed_patches, config) {
        apply_errors_len = errors.len();
        all_errors.par_extend(errors);
    };

    if is_cancelled(cancel) {
        return Errors::default();
    }

    // 3/3: Generate hkx files.
    let hkx_errors_len = {
        if let Err(hkx_errors) = generate_hkx_files(config, templates, variable_class_map, cancel) {
            let errors_len = hkx_errors.len();
            all_errors.par_extend(hkx_errors);
            errors_len
        } else {
            0
        }
    };

    Errors { patch_errors_len, apply_errors_len, hkx_errors_len, hkx_errors: all_errors }
}

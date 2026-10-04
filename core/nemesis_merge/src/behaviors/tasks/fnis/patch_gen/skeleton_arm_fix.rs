use std::{borrow::Cow, fs, path::Path};

use havok_classes::Classes;
use json_patch::{Action, JsonPatch, ValueWithPriority, json_path};
use serde_hkx_features::{ClassMap, convert::process_serde_with};
use simd_json::json_typed;

use crate::behaviors::tasks::{
    fnis::patch_gen::JsonPatchPairs,
    patches::types::BehaviorPatchesMap,
    templates::key::{THREAD_PERSON_DEFAULTFEMALE_KEY, THREAD_PERSON_DEFAULTMALE_KEY},
};

/// Generate skeleton arm-fix patches from the skeletons distributed by mods.
///
/// The skeleton path is resolved from `skyrim_data_dir_glob`. When multiple
/// files match, the first match is used and a warning is logged.
///
/// `<Skyrim Data directory>/.../skeleton.xml`
/// -> bone count
/// -> character HKX patch
///
/// # Errors
///
/// Returns an error if a matching `skeleton.xml` cannot be found, read, or
/// parsed, or if its HKX structure is invalid.
pub(crate) fn apply<'a>(
    config: &crate::Config,
    patches: &BehaviorPatchesMap<'a>,
) -> Result<(), serde_hkx_features::error::Error> {
    let skyrim_data_dir_glob = config.skyrim_data_dir_glob.as_deref().unwrap_or(".");

    for (template_key, skeleton_suffix) in [
        (THREAD_PERSON_DEFAULTMALE_KEY, "meshes/actors/character/character assets/skeleton.xml"),
        (
            THREAD_PERSON_DEFAULTFEMALE_KEY,
            "meshes/actors/character/character assets female/skeleton_female.xml",
        ),
    ] {
        let skeleton_glob = format!("{skyrim_data_dir_glob}/{skeleton_suffix}");
        let skeleton_paths = jwalk_glob::glob_files(&skeleton_glob);

        #[cfg(feature = "tracing")]
        if skeleton_paths.len() > 1 {
            tracing::warn!(
                skeleton_glob, matched_skeleton_paths = ?skeleton_paths,
                "[Skeleton Arm Fix] Multiple skeleton.xml files matched; using the first match."
            );
        }

        let Some(skeleton_path) = skeleton_paths.first() else {
            if matches!(config.parser_mode, crate::ParserMode::Lenient) {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    template_key = ?template_key,
                    skeleton_glob = %skeleton_glob,
                    "[Skeleton Arm Fix] No skeleton.xml matched in Lenient mode; skipped."
                );

                continue;
            }

            return Err(serde_hkx_features::error::Error::IoError {
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("[Skeleton Arm Fix] No skeleton.xml matched: {skeleton_glob}"),
                ),
            });
        };

        #[cfg(feature = "tracing")]
        tracing::debug!(
            template_key = ?template_key,
            skeleton_glob = %skeleton_glob,
            skeleton_path = %skeleton_path.display(),
            "[Skeleton Arm Fix] Read skeleton.xml."
        );

        let bone_count = read_skeleton_bone_len(skeleton_path)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(skeleton_path = %skeleton_path.display(), bone_count, "[Skeleton Arm Fix] Generated patches.");

        let seq = build_skeleton_arm_fix_patches(bone_count);
        let entry = patches.0.entry(template_key).or_default();
        for (path, patch) in seq {
            entry.seq.insert(path, patch);
        }
    }

    Ok(())
}

/// Read the bone count of `NPC Root [Root]` from a skeleton XML.
///
/// The skeleton is resolved through the HKX object graph:
///
/// `hkRootLevelContainer`
/// -> `namedVariants`
/// -> `hkaAnimationContainer`
/// -> `hkaSkeleton`
fn read_skeleton_bone_len<P>(path: P) -> Result<usize, serde_hkx_features::error::Error>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let bytes = fs::read(path).map_err(|e| serde_hkx_features::error::Error::FailedReadFile {
        path: path.to_path_buf(),
        source: e,
    })?;

    fn f(class_map: ClassMap<'_>) -> Result<usize, serde_hkx_features::error::Error> {
        read_character_skeleton(&class_map).map_err(|e| serde_hkx_features::error::Error::IoError {
            source: std::io::Error::other(e),
        })
    }

    process_serde_with(&bytes, path, f, f)
}

/// Resolve the character skeleton through the HKX object graph.
///
/// # Object graph
///
/// ```text
/// hkRootLevelContainer
///     └── namedVariants
///          └── Merged Animation Container
///               └── variant
///                    └── hkaAnimationContainer
///                         └── skeletons
///                              └── hkaSkeleton
///                                   └── name = NPC Root [Root]
/// ```
///
/// # Errors
///
/// Returns [`SkeletonArmFixPatchGenerationError`] if any required object or
/// reference is missing or malformed.
fn read_character_skeleton(
    class_map: &ClassMap<'_>,
) -> Result<usize, SkeletonArmFixPatchGenerationError> {
    const ANIMATION_CONTAINER_NAME: &str = "Merged Animation Container";
    const NPC_ROOT_BONE: &str = "NPC Root [Root]";

    let root_containers = class_map
        .iter()
        .filter_map(|(_, class)| {
            let Classes::hkRootLevelContainer(root) = class else {
                return None;
            };
            Some(root)
        })
        .collect::<Vec<_>>();

    let root = match root_containers.len() {
        1 => root_containers[0],
        count => return RootLevelContainerCountSnafu { count }.fail(),
    };

    let Some(named_variant) = root
        .m_namedVariants
        .iter()
        .find(|variant| *variant.m_name.get_ref() == Some(Cow::Borrowed(ANIMATION_CONTAINER_NAME)))
    else {
        return MissingAnimationContainerVariantSnafu { variant_name: ANIMATION_CONTAINER_NAME }
            .fail();
    };

    if *named_variant.m_className.get_ref() != Some(Cow::Borrowed("hkaAnimationContainer")) {
        return UnexpectedAnimationContainerClassSnafu {
            variant_name: ANIMATION_CONTAINER_NAME,
            class_name: named_variant.m_className.to_string(),
        }
        .fail();
    }

    let animation_container_index = named_variant.m_variant.get();
    let Some(animation_container) = class_map.iter().find_map(|(index, class)| {
        if index == animation_container_index {
            match class {
                Classes::hkaAnimationContainer(container) => Some(container),
                _ => None,
            }
        } else {
            None
        }
    }) else {
        let exists = class_map.iter().any(|(index, _)| index == animation_container_index);

        if exists {
            return UnexpectedAnimationContainerClassSnafu {
                variant_name: ANIMATION_CONTAINER_NAME,
                class_name: animation_container_index.to_string(),
            }
            .fail();
        }

        return MissingAnimationContainerSnafu {
            variant_name: ANIMATION_CONTAINER_NAME,
            index: animation_container_index.to_string(),
        }
        .fail();
    };

    if animation_container.m_skeletons.is_empty() {
        return EmptySkeletonsSnafu { index: named_variant.m_variant.to_string() }.fail();
    }

    let mut character_skeleton = None;
    for skeleton_reference in &animation_container.m_skeletons {
        let skeleton_index = skeleton_reference.get();

        let Some(class) =
            class_map.iter().find_map(|(index, class)| (index == skeleton_index).then_some(class))
        else {
            return MissingSkeletonSnafu { index: skeleton_index.to_string() }.fail();
        };

        let Classes::hkaSkeleton(skeleton) = class else {
            return InvalidSkeletonReferenceSnafu { index: skeleton_reference.to_string() }.fail();
        };

        if *skeleton.m_name.get_ref() == Some(Cow::Borrowed(NPC_ROOT_BONE)) {
            character_skeleton = Some(skeleton);
            break;
        }
    }

    let skeleton = character_skeleton
        .ok_or_else(|| MissingCharacterSkeletonSnafu { name: NPC_ROOT_BONE }.build())?;

    let bone_count = skeleton.m_bones.len();
    if bone_count == 0 {
        return EmptyCharacterSkeletonSnafu { name: NPC_ROOT_BONE }.fail();
    }

    if bone_count - 1 > i16::MAX as usize {
        return BoneIndexOverflowSnafu { bone_count }.fail();
    }

    Ok(bone_count)
}

/// Generate the skeleton arm-fix patch.
///
/// `skeleton.xml` -> `hkaAnimationContainer` -> `NPC Root [Root]`
/// -> bone count -> append missing indices (`99..bone_count`) to `bonePairMap`
/// -> append `0.0` / `1.0` to the corresponding `hkbBoneWeightArray`s.
fn build_skeleton_arm_fix_patches(bone_len: usize) -> JsonPatchPairs<'static> {
    /// Number of bone-pair entries already present in the character template.
    ///
    /// The source skeleton contains 126 bones, while the target character
    /// template contains only the first 99 entries (`0..=98`).
    /// The arm fix therefore appends bone indices `99..=125`.
    const VANILLA_BONE_LEN: usize = 99;

    /// Bone-weight arrays that receive `0.0` for every newly appended bone.
    ///
    /// # Notes
    /// - Same Index defaultmale/defaultfemale
    /// - These additional class specifications can be obtained from the FNIS templates.
    const ZERO_WEIGHT_ARRAYS: &[&str] = &[
        "#0031", "#0032", "#0035", "#0037", "#0038", "#0040", "#0044", "#0045", "#0046", "#0047",
        "#0048", "#0049", "#0050", "#0054", "#0055", "#0056", "#0057",
    ];

    /// Bone-weight arrays that receive `1.0` for every newly appended bone.
    ///
    /// # Notes
    /// - Same Index defaultmale/defaultfemale
    /// - These additional class specifications can be obtained from the FNIS templates.
    const ONE_WEIGHT_ARRAYS: &[&str] =
        &["#0033", "#0034", "#0036", "#0039", "#0041", "#0051", "#0052", "#0053"];

    let missing_bone_len = bone_len.saturating_sub(VANILLA_BONE_LEN);
    if missing_bone_len == 0 || bone_len <= VANILLA_BONE_LEN {
        return Vec::new();
    }

    let mut seq_patches =
        Vec::with_capacity(1 + ZERO_WEIGHT_ARRAYS.len() + ONE_WEIGHT_ARRAYS.len());

    // Append all missing bone-pair indices in one SeqPush.
    let missing_bone_indices: Vec<_> = (VANILLA_BONE_LEN..bone_len).collect();
    seq_patches.push((
        json_path!["#0028", "hkbMirroredSkeletonInfo", "bonePairMap"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::SeqPush,
                value: json_typed!(borrowed, missing_bone_indices),
            },
            priority: 0,
        },
    ));

    // Append all missing weights in one SeqPush per weight array.
    let zero_weights = vec![0.0_f32; missing_bone_len];
    for &class_index in ZERO_WEIGHT_ARRAYS {
        seq_patches.push((
            json_path![class_index, "hkbBoneWeightArray", "boneWeights"],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: json_typed!(borrowed, zero_weights),
                },
                priority: 0,
            },
        ));
    }

    let one_weights = vec![1.0_f32; missing_bone_len];
    for &class_index in ONE_WEIGHT_ARRAYS {
        seq_patches.push((
            json_path![class_index, "hkbBoneWeightArray", "boneWeights"],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: json_typed!(borrowed, one_weights),
                },
                priority: 0,
            },
        ));
    }

    seq_patches
}

#[derive(Debug, snafu::Snafu)]
pub(crate) enum SkeletonArmFixPatchGenerationError {
    /// The HKX file does not contain exactly one root-level container.
    #[snafu(display("Expected exactly one hkRootLevelContainer, but found {count}"))]
    RootLevelContainerCount { count: usize },

    /// The root-level container does not contain the expected animation
    /// container named variant.
    #[snafu(display(
        "The hkRootLevelContainer does not contain the `{variant_name}` named variant"
    ))]
    MissingAnimationContainerVariant { variant_name: &'static str },

    /// The expected animation container named variant references an
    /// unexpected Havok class.
    #[snafu(display(
        "The `{variant_name}` named variant has an unexpected class name `{class_name}`"
    ))]
    UnexpectedAnimationContainerClass { variant_name: &'static str, class_name: String },

    /// The animation container named variant references an object that does
    /// not exist in the HKX class map.
    #[snafu(display("The `{variant_name}` named variant references missing object `{index}`"))]
    MissingAnimationContainer { variant_name: &'static str, index: String },

    /// The resolved animation container does not reference any skeletons.
    #[snafu(display("The animation container `{index}` contains no skeletons"))]
    EmptySkeletons { index: String },

    /// The animation container references a skeleton object that does not
    /// exist in the HKX class map.
    #[snafu(display("The animation container references missing skeleton `{index}`"))]
    MissingSkeleton { index: String },

    /// The animation container references an object that is not an
    /// `hkaSkeleton`.
    #[snafu(display("The animation container reference `{index}` is not an hkaSkeleton"))]
    InvalidSkeletonReference { index: String },

    /// The animation container does not contain the character skeleton
    /// identified by its expected name.
    #[snafu(display("The animation container does not contain an hkaSkeleton named `{name}`"))]
    MissingCharacterSkeleton { name: &'static str },

    /// The expected character skeleton exists but contains no bones.
    #[snafu(display("The character skeleton `{name}` contains no bones"))]
    EmptyCharacterSkeleton { name: &'static str },

    /// The source skeleton contains more bones than can be represented by
    /// the `i16` indices used by `hkbMirroredSkeletonInfo.bonePairMap`.
    #[snafu(display(
        "The skeleton contains {bone_count} bones, but the bone index cannot be represented by i16"
    ))]
    BoneIndexOverflow { bone_count: usize },
}

//! Writing the real mod: only what the work read or made is replaced.

use std::{fmt::Write as _, path::Path};

use gui_shared::settings::pages::patch::behavior::ParserMode;
use stdx::text::push_joined;

use super::{
    Workspace,
    locations::{ENGINES, joined},
    write,
};

/// e.g. `Nemesis_Engine\\mod\\para\\0_master\\#0106.txt` -> `Nemesis_Engine/mod/para/0_master/#0106.txt`
pub(super) fn owned_key(relative: &Path) -> String {
    relative.to_string_lossy().replace('\\', "/")
}

/// Removes a file of [`Workspace::owned`], and the directories this leaves empty.
pub(crate) fn remove_owned(output: &Path, file: &str) -> Result<(), String> {
    let path = output.join(file);
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("{}: {err}", path.display())),
    }
    let mut dir = path.parent();
    while let Some(parent) = dir.filter(|parent| *parent != output) {
        if std::fs::remove_dir(parent).is_err() {
            break; // Not empty, or not there.
        }
        dir = parent.parent();
    }
    Ok(())
}

impl Workspace {
    /// The templates with patches that Nemesis itself can't read when the work is written as
    /// [`super::ExportFormat::Nemesis`]: Nemesis does not know them, so [`Self::export`] puts them in
    /// `Nemesis_EngineExt`, which only d-merge reads. (Animation data always goes to Nemesis.)
    ///
    /// e.g. `meshes/actors/troll/behaviors/trollbehavior.bin`
    pub fn outside_nemesis(&self) -> impl Iterator<Item = &str> {
        let patched = |behavior: &&super::Behavior| !(behavior.edits.is_empty() && behavior.added.is_empty());
        let unknown = |behavior: &&super::Behavior| !behavior.is_anim_data() && behavior.nemesis.is_none();
        self.behaviors.iter().filter(patched).filter(unknown).map(|behavior| behavior.meshes_path.as_str())
    }

    /// The mod codes this mod depends on: what it said when it was read, and the mods it refers to.
    ///
    /// e.g. a reference `D:/mods/a/Nemesis_Engine/mod/amco` -> `amco`
    pub(super) fn deps(&self, own_code: &str) -> Vec<String> {
        let refs = self.ref_mods.iter().filter_map(|dir| Path::new(dir).file_name()?.to_str());
        let all: std::collections::BTreeSet<&str> =
            self.info.deps.iter().map(String::as_str).chain(refs).filter(|code| *code != own_code).collect();
        all.into_iter().map(str::to_owned).collect()
    }

    /// `info.ini` of `Nemesis_EngineExt`: `info`, with what only that one has. What is not known is left
    /// out, so that a mod without any of it reads like Nemesis'.
    pub fn ext_info(&self, info: &str, own_code: &str) -> String {
        let mut out = info.to_owned();
        if !self.info.version.trim().is_empty() {
            let _ = write!(out, "\nversion={}", self.info.version.trim());
        }
        let deps = self.deps(own_code);
        if !deps.is_empty() {
            out.push_str("\ndeps=");
            push_joined(&mut out, deps.iter().map(String::as_str), ",");
        }
        out
    }

    /// Writes the mod under `<output>`: each behavior to its [`Behavior::patch_dir`], `info.ini` to
    /// each mod root used. Returns the files written and the patches d-merge's Nemesis parser rejects
    /// (`behavior/#id: error`).
    ///
    /// By default `<output>` is the mod it was read from, so only [`Self::owned`] is replaced: what
    /// this work did not read stays (a readme, the patches of a template that is not installed).
    ///
    /// # Errors
    /// A file could not be written.
    pub fn export(&mut self, output: &Path, code: &str, mode: ParserMode) -> Result<(usize, Vec<String>), String> {
        for file in &self.owned {
            remove_owned(output, file)?;
        }
        self.owned.clear();
        let info = self.info.exported();

        let (mut written, mut invalid) = (0, Vec::new());
        let ext = self.export_format.is_ext();
        for behavior in &self.behaviors {
            let relative = behavior.patch_dir(code, ext);
            for id in behavior.edits.keys().chain(behavior.added.keys()) {
                let Some(patch) = behavior.patch(code, id) else { continue };
                if let Err(err) = behavior.validate(id, &patch, mode) {
                    invalid.push(format!("{}/{id}: {err}", behavior.name));
                }
                let file = Path::new(&relative).join(behavior.patch_file(id));
                write(&output.join(&file), &patch)?;
                self.owned.insert(owned_key(&file));
                written += 1;
            }
        }
        let roots = ENGINES.map(|engine| joined(output, &[engine, "mod", code]));
        let [nemesis, ext] = roots;
        if nemesis.is_dir() {
            write(&nemesis.join("info.ini"), &info)?;
        }
        if ext.is_dir() {
            write(&ext.join("info.ini"), &self.ext_info(&info, code))?;
        }
        Ok((written, invalid))
    }
}

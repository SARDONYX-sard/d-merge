//! A mod's metadata, `info.ini`.

/// Mod metadata (`info.ini`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModInfo {
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) site: String,
    /// The mod's version now, written for `Nemesis_EngineExt` only. e.g. `1.2.0`
    pub(crate) version: String,
    /// Mod codes it says it depends on, as it was read. See [`Workspace::deps`].
    pub(crate) deps: Vec<String>,
    /// The lines the editor has no field for, as they were read: the mod's, so they stay.
    ///
    /// e.g. `auto=true`
    pub(crate) other: Vec<String>,
}

pub(super) fn info_ini(info: &ModInfo) -> String {
    let deps = info.deps.join(",");
    let mut out = format!(
        "name={}\nauthor={}\nsite={}\nversion={}\ndeps={deps}\n",
        info.name, info.author, info.site, info.version
    );
    for line in &info.other {
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub(super) fn parse_info(ini: &str) -> ModInfo {
    let mut info = ModInfo::default();
    for line in ini.lines().filter(|line| !line.trim().is_empty()) {
        let Some((key, value)) = line.split_once('=') else {
            info.other.push(line.to_owned());
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "name" => info.name = value,
            "author" => info.author = value,
            "site" => info.site = value,
            "version" | "deps" if value.eq_ignore_ascii_case("null") => {}
            "version" => info.version = value,
            "deps" => {
                let codes = value.split(',').map(str::trim).filter(|code| !code.is_empty());
                info.deps = codes.map(str::to_owned).collect();
            }
            _ => info.other.push(line.to_owned()),
        }
    }
    info
}

impl ModInfo {
    /// `info.ini` of the mod, without a line end at the end. Nemesis wants `auto`, so a mod has it.
    pub(super) fn exported(&self) -> String {
        let mut out = format!("name={}\nauthor={}\nsite={}", self.name, self.author, self.site);
        for line in &self.other {
            out.push('\n');
            out.push_str(line);
        }
        let is_auto = |line: &String| line.split_once('=').is_some_and(|(key, _)| key.trim() == "auto");
        if !self.other.iter().any(is_auto) {
            out.push_str("\nauto=null");
        }
        out
    }
}

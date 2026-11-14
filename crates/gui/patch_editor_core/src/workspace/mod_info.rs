//! A mod's metadata, `info.ini`.

/// Mod metadata (`info.ini`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModInfo {
    pub name: String,
    pub author: String,
    pub site: String,
    /// The mod's version now, written for `Nemesis_EngineExt` only. e.g. `1.2.0`
    pub version: String,
    /// Mod codes it says it depends on, as it was read. See [`Workspace::deps`].
    pub(super) deps: Vec<String>,
    /// The lines the editor has no field for, as they were read: the mod's, so they stay.
    ///
    /// e.g. `auto=true`
    pub(super) other: Vec<String>,
}

pub(crate) fn info_ini(info: &ModInfo) -> String {
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

/// What the `info.xml` of the Pandora mod at `dir` says. `null` is what Pandora writes for what is not
/// known. `None`: it has none, or it can't be read.
///
/// e.g. `<mod code="amco"><name>Attack MCO</name><author>a</author><site>null</site></mod>`
pub(crate) fn pandora_info(dir: &std::path::Path) -> Option<ModInfo> {
    let text = std::fs::read(dir.join("info.xml")).ok()?;
    let text = String::from_utf8_lossy(&text);
    let root = pandora_patch::tree::parse(text.trim_start_matches('\u{feff}'))?;
    let field = |name: &str| {
        let words = root.children.iter().find(|child| child.tag == name).map(|child| child.words.join(" "));
        words.filter(|words| !words.eq_ignore_ascii_case("null")).unwrap_or_default()
    };
    Some(ModInfo { name: field("name"), author: field("author"), site: field("site"), ..ModInfo::default() })
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

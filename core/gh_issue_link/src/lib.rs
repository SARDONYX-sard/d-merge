pub mod version;

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use urlencoding::encode;

#[derive(Debug, Clone, Copy)]
pub enum SkyrimRuntime {
    /// `Special/Anniversary Edition(64-bit)`
    Se,
    /// `VR(64bit)`
    Vr,
    /// `Legendary Edition(32bit)`
    Le,
}

impl SkyrimRuntime {
    const fn as_str(&self) -> &'static str {
        // NOTE: This must be identical to the string in `.github/ISSUE_TEMPLATE/bug-report.yaml`.
        match self {
            Self::Se => "Special/Anniversary Edition(64-bit)",
            Self::Vr => "VR(64-bit)",
            Self::Le => "Legendary Edition(32-bit)",
        }
    }
}

/// Creates a `System` instance with CPU and RAM information collected.
///
/// This function should be called by the user once, and the returned `System`
/// can be shared across multiple `EnvInfo` instances to avoid redundant refreshes.
///
/// # Example
///
/// ```
/// let sys = gh_issue_link::create_system();
/// ```
pub fn create_system() -> System {
    System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    )
}

/// Holds environment information for generating GitHub issues for d-merge.
///
/// The required CPU, RAM, and OS information is collected at creation time
/// from a `System` reference. EnvInfo is independent once created and does
/// not retain the System reference for further refreshes.
///
/// # Example
///
/// ```
/// use gh_issue_link::{create_system, EnvInfo};
///
/// let sys = create_system();
/// let env = EnvInfo::new(&sys);
///
/// println!("CPU: {:?}", env.cpu);
/// println!("RAM: {}", env.dram);
/// println!("OS: {}", env.os);
/// ```
#[derive(Debug)]
pub struct EnvInfo<'a> {
    /// CPU brand string (may be None)
    pub cpu: Option<&'a str>,
    /// Total RAM in GB as a formatted string
    pub dram: String,
    /// OS version string
    pub os: String,
}

impl<'a> EnvInfo<'a> {
    /// Constructs a new `EnvInfo` by collecting environment details from a `System` reference.
    ///
    /// - `sys` - Reference to a `System` created by `create_system()`
    /// - `d_merge_version` - Version of d-merge
    /// - `skyrim_version` - Version of Skyrim
    pub fn new(sys: &'a System) -> Self {
        let cpu = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim())
            .filter(|c| !c.is_empty());
        let dram = format!(
            "{:.1} GB",
            sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0
        );

        let os = System::long_os_version();
        let ver = System::kernel_version();
        let os = format!(
            "{} {}",
            os.as_deref().unwrap_or("Unknown OS"),
            ver.as_deref().unwrap_or("")
        );

        Self { cpu, dram, os }
    }
}

/// Generates a GitHub issue link pre-filled with environment information.
///
/// The user only needs to fill in the title, unexpected behavior, reproduction steps,
/// and debug output on GitHub after opening the link.
///
/// # Example
///
/// ```
/// use gh_issue_link::{SkyrimRuntime, new_gh_issue_link};
///
/// println!("{}", new_gh_issue_link("0.1.0", SkyrimRuntime::Se, "1.6.1170.0"));
/// ```
pub fn new_gh_issue_link(
    d_merge_version: &str,
    skyrim_runtime: SkyrimRuntime,
    skyrim_version: Option<&str>,
) -> String {
    let base_url = "https://github.com/SARDONYX-sard/d-merge/issues/new";
    let template = "bug-report.yaml";
    let labels = "bug";

    let mut url = format!(
        "{base_url}?assignees=&labels={}%2CNeeds+Triage&template={}",
        encode(labels),
        encode(template)
    );

    let mut append_param = |key: &str, v: &str| {
        url.push('&');
        url.push_str(key);
        url.push('=');
        url.push_str(&encode(v));
    };

    let sys = create_system();
    let env = EnvInfo::new(&sys);

    append_param("version", d_merge_version);
    if let Some(cpu) = env.cpu {
        append_param("cpu", cpu);
    }
    append_param("dram", &env.dram);
    append_param("os", &env.os);
    append_param("skyrim-version", skyrim_runtime.as_str());
    if let Some(skyrim_version) = skyrim_version {
        append_param("skyrim-version", skyrim_version);
    }

    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_info_link() {
        let sys = create_system();
        let env = EnvInfo::new(&sys);
        println!("env: {env:#?}");

        println!(
            "GitHub issue link:\n{}",
            new_gh_issue_link("0.1.0", SkyrimRuntime::Se, Some("1.6.1170.0"))
        );
    }
}

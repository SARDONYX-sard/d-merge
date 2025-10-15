use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use urlencoding::encode;

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
/// let env = EnvInfo::new(&sys, "0.1.0", "1.6.1170.0");
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
    /// d-merge version
    pub d_merge_version: &'a str,
    /// Skyrim version
    pub skyrim_version: &'a str,
}

impl<'a> EnvInfo<'a> {
    /// Constructs a new `EnvInfo` by collecting environment details from a `System` reference.
    ///
    /// - `sys` - Reference to a `System` created by `create_system()`
    /// - `d_merge_version` - Version of d-merge
    /// - `skyrim_version` - Version of Skyrim
    pub fn new(sys: &'a System, d_merge_version: &'a str, skyrim_version: &'a str) -> Self {
        let cpu = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim())
            .filter(|c| !c.is_empty());
        let dram = format!(
            "{:.1} GB",
            sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0
        );
        let os = System::kernel_long_version();

        Self {
            cpu,
            dram,
            os,
            d_merge_version,
            skyrim_version,
        }
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
/// println!("{}", gh_issue_link::new_gh_issue_link());
/// ```
pub fn new_gh_issue_link() -> String {
    let base_url = "https://github.com/SARDONYX-sard/d-merge/issues/new";
    let template = "bug-report.yaml";
    let labels = "bug";

    let mut url = format!(
        "{base_url}?assignees=&labels={}%2CNeeds+Triage&template={}",
        encode(labels),
        encode(template)
    );

    let mut append_param = |key: &str, value: Option<&str>| {
        if let Some(v) = value {
            url.push('&');
            url.push_str(key);
            url.push('=');
            url.push_str(&encode(v));
        }
    };

    let sys = create_system();
    let env = EnvInfo::new(&sys, "0.1.0", "1.6.1170.0");
    append_param("version", Some(env.d_merge_version));
    append_param("cpu", env.cpu);
    append_param("dram", Some(&env.dram));
    append_param("gpu", None);
    append_param("os", Some(&env.os));
    append_param("skyrim-version", Some(env.skyrim_version));

    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_info_link() {
        let sys = create_system();
        let env = EnvInfo::new(&sys, "0.1.0", "1.6.1170.0");
        println!("env: {env:#?}");

        println!("GitHub issue link:\n{}", new_gh_issue_link());
    }
}

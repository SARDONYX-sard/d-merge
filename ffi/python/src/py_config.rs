use nemesis_merge::{Config, DebugOptions, HackOptions, OutPutTarget};
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone, Copy, Debug)]
pub enum PyOutPutTarget {
    SkyrimSe,
    SkyrimLe,
}

impl From<PyOutPutTarget> for OutPutTarget {
    #[inline]
    fn from(value: PyOutPutTarget) -> Self {
        match value {
            PyOutPutTarget::SkyrimSe => OutPutTarget::SkyrimSe,
            PyOutPutTarget::SkyrimLe => OutPutTarget::SkyrimLe,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyConfig {
    #[pyo3(get, set)]
    resource_dir: String,
    #[pyo3(get, set)]
    output_dir: String,
    #[pyo3(get, set)]
    output_target: PyOutPutTarget,
    #[pyo3(get, set)]
    cast_ragdoll_event: bool,
    #[pyo3(get, set)]
    output_patch_json: bool,
    #[pyo3(get, set)]
    output_merged_json: bool,
    #[pyo3(get, set)]
    output_merged_xml: bool,
}

impl From<PyConfig> for Config {
    fn from(value: PyConfig) -> Self {
        let output_target = OutPutTarget::from(value.output_target);

        Config {
            resource_dir: value.resource_dir.into(),
            output_dir: value.output_dir.into(),
            output_target,
            status_report: None,
            hack_options: if value.cast_ragdoll_event {
                Some(HackOptions::enable_all())
            } else {
                None
            },
            debug: DebugOptions {
                output_patch_json: value.output_patch_json,
                output_merged_json: value.output_merged_json,
                output_merged_xml: value.output_merged_xml,
            },
        }
    }
}

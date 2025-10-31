pub mod log;
pub mod patch;
pub mod serde_hkx_;

use pyo3::prelude::*;

// To call `d_merge_python::stub_info()?;`
pyo3_stub_gen::define_stub_info_gatherer!(stub_info);

/// A Python module implemented in Rust.
#[pyo3::pymodule(name = "d_merge_python")] // https://github.com/PyO3/maturin/issues/2455
fn d_merge_python_patch(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(log::logger_init, m)?)?;
    m.add_function(wrap_pyfunction!(log::change_log_level, m)?)?;
    m.add_function(wrap_pyfunction!(log::log_trace, m)?)?;
    m.add_function(wrap_pyfunction!(log::log_debug, m)?)?;
    m.add_function(wrap_pyfunction!(log::log_info, m)?)?;
    m.add_function(wrap_pyfunction!(log::log_warn, m)?)?;
    m.add_function(wrap_pyfunction!(log::log_error, m)?)?;

    m.add_class::<patch::Config>()?;
    m.add_class::<patch::DebugOptions>()?;
    m.add_class::<patch::HackOptions>()?;
    m.add_class::<patch::ModInfo>()?;
    m.add_class::<patch::ModType>()?;
    m.add_class::<patch::OutPutTarget>()?;
    m.add_class::<patch::PatchMaps>()?;
    m.add_class::<patch::PatchStatus>()?;
    m.add_function(wrap_pyfunction!(patch::behavior_gen, m)?)?;
    m.add_function(wrap_pyfunction!(patch::get_skyrim_data_dir, m)?)?;
    m.add_function(wrap_pyfunction!(patch::load_mods_info, m)?)?;

    m.add_class::<serde_hkx_::DirEntry>()?;
    m.add_class::<serde_hkx_::Payload>()?;
    m.add_class::<serde_hkx_::SerdeHkxStatus>()?;
    m.add_function(wrap_pyfunction!(serde_hkx_::convert, m)?)?;
    m.add_function(wrap_pyfunction!(serde_hkx_::load_dir_node, m)?)?;

    Ok(())
}

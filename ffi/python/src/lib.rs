mod behavior;
mod py_config;

use pyo3::prelude::*;

use crate::{
    behavior::behavior_gen_py,
    py_config::{PyConfig, PyOutPutTarget},
};

/// A Python module implemented in Rust.
#[pymodule(name = "d_merge_ffi")] // https://github.com/PyO3/maturin/issues/2455
fn ffi_d_merge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyOutPutTarget>()?;
    m.add_class::<PyConfig>()?;
    m.add_function(wrap_pyfunction!(behavior_gen_py, m)?)?;
    Ok(())
}

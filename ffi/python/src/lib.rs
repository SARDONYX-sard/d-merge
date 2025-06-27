mod behavior;
mod py_config;

use pyo3::prelude::*;

use crate::{
    behavior::behavior_gen_py,
    py_config::{PyConfig, PyOutPutTarget},
};

/// A Python module implemented in Rust.
#[pymodule]
fn ffi_d_merge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyOutPutTarget>()?;
    m.add_class::<PyConfig>()?;
    m.add_function(wrap_pyfunction!(behavior_gen_py, m)?)?;
    Ok(())
}

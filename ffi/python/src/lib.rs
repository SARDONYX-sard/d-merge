mod behavior;
mod py_config;
mod status;

use pyo3::prelude::*;

use crate::{
    behavior::behavior_gen,
    py_config::{Config, LogLevel, OutPutTarget},
    status::Status,
};

/// A Python module implemented in Rust.
#[pymodule(name = "d_merge_python")] // https://github.com/PyO3/maturin/issues/2455
fn d_merge_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<OutPutTarget>()?;
    m.add_class::<LogLevel>()?;
    m.add_class::<Status>()?;
    m.add_class::<Config>()?;
    m.add_function(wrap_pyfunction!(behavior_gen, m)?)?;
    Ok(())
}

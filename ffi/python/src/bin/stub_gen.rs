#![expect(unused_crate_dependencies)] // Wrong lint. use deps ffi lib
use pyo3_stub_gen::Result;

// update pyi
// cargo run -p d_merge_python --bin stub_gen
fn main() -> Result<()> {
    // `stub_info` is a function defined by `define_stub_info_gatherer!` macro.
    let stub = d_merge_python::stub_info()?;
    stub.generate()?;
    Ok(())
}

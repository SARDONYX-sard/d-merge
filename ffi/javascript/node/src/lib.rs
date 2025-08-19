pub mod log;
mod patch;
mod serde_hkx_;

use neon::prelude::*;
use tokio::runtime::Runtime;

/// Return a global tokio runtime or create one if it doesn't exist.
/// Throws a JavaScript exception if the `Runtime` fails to create.
pub fn get_tokio_rt<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    use once_cell::sync::OnceCell;

    static RUNTIME: OnceCell<Runtime> = OnceCell::new();
    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    log::register_api(&mut cx)?;
    patch::register_api(&mut cx)?;
    serde_hkx_::register_api(&mut cx)?;
    Ok(())
}

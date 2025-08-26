use std::env;

fn main() {
    // only run if target os is windows
    let is_non_windows = env::var("CARGO_CFG_TARGET_OS")
        .map(|os| os != "windows")
        .unwrap_or(true);

    // Since the PROFILE environment variable becomes empty for builds other than "debug" or "release", use OPT_LEVEL instead.
    let release_opt_level = env::var("OPT_LEVEL").unwrap_or_default() == "3";

    if is_non_windows || !release_opt_level {
        return;
    }

    if let Err(e) = embed_resources() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

// ref: https://github.com/mxre/winres/blob/master/example/build.rs
fn embed_resources() -> Result<(), std::io::Error> {
    const ICO_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../backend/tauri/icons/icon.ico"
    );
    let path = std::path::Path::new(ICO_PATH);
    let mut res = winres::WindowsResource::new();

    #[cfg(unix)]
    {
        res.set_toolkit_path("/usr/x86_64-w64-mingw32/bin");
        res.set_ar_path("ar");
        res.set_windres_path("/usr/bin/x86_64-w64-mingw32-windres");
    }

    res.set("ProductName", env!("CARGO_PKG_NAME"))
        .set("CompanyName", env!("CARGO_PKG_AUTHORS"))
        .set("FileDescription", env!("CARGO_PKG_DESCRIPTION"))
        .set("LegalCopyright", env!("CARGO_PKG_AUTHORS"))
        .set_icon(&path.canonicalize()?.to_string_lossy())
        .set_language(0x0409);

    res.compile()?;
    Ok(())
}

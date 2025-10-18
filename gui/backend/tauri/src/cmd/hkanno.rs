use std::{path::Path, str::FromStr as _};

use serde_hkx_for_gui::hkanno::{parse_as_hkanno, Hkanno, OutFormat};

/// path: hkx or xml path
#[tauri::command]
pub(crate) async fn load_hkanno(input: &Path) -> Result<Hkanno<'static>, String> {
    use tokio::fs;

    // read existing hkx
    let bytes = fs::read(&input)
        .await
        .map_err(|e| format!("Failed to read file({}: {e}", input.display()))?;

    let mut buffer = String::new();
    parse_as_hkanno(&bytes, &mut buffer, input)
        .map_err(|e| e.to_string())
        .map(|anno| anno.into_static())
}

#[tauri::command]
pub(crate) async fn save_hkanno(
    input: &Path,
    output: &Path,
    hkanno: Hkanno<'_>,
    format: &str,
) -> Result<(), String> {
    use tokio::fs;

    // read existing hkx
    let mut bytes = fs::read(&input)
        .await
        .map_err(|e| format!("Failed to read file({}: {e}", input.display()))?;

    let format =
        OutFormat::from_str(format).map_err(|_| format!("Invalid output format: {format}"))?;

    // update in-memory bytes
    let updated = hkanno
        .update_hkx_bytes(&mut bytes, format, input)
        .map_err(|e| format!("Failed to update hkx: {e}"))?;

    // write updated file
    fs::write(&output, updated)
        .await
        .map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(())
}

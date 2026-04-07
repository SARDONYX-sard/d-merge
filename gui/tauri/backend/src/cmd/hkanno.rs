use std::{path::Path, str::FromStr as _};

use serde_hkx_for_gui::hkanno::{parse_as_hkanno, Hkanno, OutFormat};
use tokio::fs;

/// path: hkx or xml path
#[tauri::command]
pub(crate) async fn load_hkanno(input: &Path) -> Result<Hkanno<'static>, String> {
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
    let mut bytes = fs::read(&input)
        .await
        .map_err(|e| format!("Failed to read file({}: {e}", input.display()))?;

    let format =
        OutFormat::from_str(format).map_err(|_| format!("Invalid output format: {format}"))?;

    let updated = hkanno
        .update_hkx_bytes(&mut bytes, format, input)
        .map_err(|e| format!("Failed to update hkx: {e}"))?;

    fs::write(&output, updated)
        .await
        .map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(())
}

#[tauri::command]
pub(crate) async fn preview_hkanno(input: &Path, hkanno: Hkanno<'_>) -> Result<String, String> {
    let mut bytes = fs::read(&input)
        .await
        .map_err(|e| format!("Failed to read file({}: {e}", input.display()))?;

    let updated = hkanno
        .update_hkx_bytes(&mut bytes, OutFormat::Xml, input)
        .map_err(|e| format!("Failed to update hkx: {e}"))?;

    String::from_utf8(updated).map_err(|e| format!("Failed to utf-8: {e}"))
}

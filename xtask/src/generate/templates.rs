//! `cargo x gen templates [<dir>..] [-o <dir>]`: template xml -> msgpack `.bin`.
//!
//! The output keeps the path below `meshes`, e.g.
//! `resource/xml/templates/meshes/actors/character/behaviors/0_master.xml`
//! -> `resource/assets/templates/meshes/actors/character/behaviors/0_master.bin`

use std::path::{Path, PathBuf};

use crate::Result;

pub(crate) fn run(ws: &Path, args: &[String]) -> Result {
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut output = ws.join("resource/assets/templates");

    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "-o" {
            output = ws.join(args.next().ok_or("`-o` needs a directory")?);
        } else {
            inputs.push(ws.join(arg));
        }
    }
    if inputs.is_empty() {
        inputs.push(ws.join("resource/xml/templates"));
    }

    behavior_merge::create_bin_templates(inputs.iter(), &output);
    println!("templates written to {}", output.display());
    Ok(())
}

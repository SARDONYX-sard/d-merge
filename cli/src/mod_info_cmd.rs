pub(crate) fn run_skyrim_dir(
    runtime: crate::args::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::args::Runtime;
    let rt = match runtime {
        Runtime::Se => skyrim_data_dir::Runtime::Se,
        Runtime::Le => skyrim_data_dir::Runtime::Le,
    };
    let path = skyrim_data_dir::get_skyrim_data_dir(rt)?;
    println!("{}", path.display());
    Ok(())
}

pub(crate) fn run_mods(glob: &str, is_vfs: bool) -> Result<(), Box<dyn std::error::Error>> {
    let infos = mod_info::get_all(glob, is_vfs)?;
    let json = sonic_rs::to_string_pretty(&infos)?;
    println!("{json}");
    Ok(())
}

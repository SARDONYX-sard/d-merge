use nemesis_merge::{errors::Error, PriorityMap};
use rayon::prelude::*;
use std::path::Path;

pub(crate) fn parse_ids_ini(path: &Path, start_idx: usize) -> Result<PriorityMap, Error> {
    let raw = std::fs::read_to_string(path).map_err(|e| Error::FailedIo {
        path: path.to_path_buf(),
        source: e,
    })?;
    let lines = collect_ids(&raw);
    let map = lines
        .into_par_iter()
        .enumerate()
        .map(|(idx, id)| (id.to_string(), start_idx + idx))
        .collect();
    Ok(map)
}

fn collect_ids(raw: &str) -> Vec<&str> {
    raw.par_lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(';'))
        .collect()
}

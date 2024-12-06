pub mod error;

use crate::error::{Error, Result};
use rayon::prelude::*;
use std::{fs, path::Path};

/// Represents a node in the directory structure.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirEntry {
    /// Relative or absolute path
    #[cfg_attr(feature = "serde", serde(rename = "id"))]
    pub path: String,
    /// The name of the entry (file or directory).
    #[cfg_attr(feature = "serde", serde(rename = "label"))]
    pub name: String,
    /// The sub-entries contained within this directory, if applicable.
    /// This will be `None` if the entry is a file.
    pub children: Option<Vec<DirEntry>>,
}

/// Creates a hierarchical structure representing the contents of a directory,
/// filtering the entries based on allowed file extensions.
///
/// # Errors
/// Returns an error if there is an issue reading the directory or its contents.
pub fn build_dir_tree<const N: usize>(
    path: impl AsRef<Path>,
    allowed_extensions: [&str; N],
) -> Result<DirEntry> {
    let path = path.as_ref();

    let name = path
        .file_name()
        .map_or_else(|| path.to_string_lossy(), |os_str| os_str.to_string_lossy())
        .to_string();

    let children = if path.is_dir() {
        // Collect entries in a vector for parallel processing
        let entries: Vec<_> = fs::read_dir(path)
            .map_err(|e| Error::IoError {
                source: e,
                path: path.to_path_buf(),
            })?
            .collect();

        // Use parallel processing to handle entries
        let child_nodes: Vec<DirEntry> = entries
            .into_par_iter()
            .filter_map(|entry| {
                let entry = entry.ok()?; // Skip entries that failed to read
                let child_path = entry.path();

                // Check if the file has an allowed extension if it's a file
                if child_path.is_file() {
                    if let Some(extension) = child_path.extension().and_then(|ext| ext.to_str()) {
                        if !allowed_extensions
                            .par_iter()
                            .any(|&ext| ext.eq_ignore_ascii_case(extension))
                        {
                            return None; // Skip files with disallowed extensions
                        }
                    } else {
                        return None; // Skip files without an extension
                    }
                }

                build_dir_tree(&child_path, allowed_extensions)
                    .ok()
                    .and_then(|child_node| {
                        if child_node.children.is_some() || child_path.is_file() {
                            Some(child_node)
                        } else {
                            None // Skip empty directories
                        }
                    })
            })
            .collect();

        if child_nodes.is_empty() {
            None
        } else {
            Some(child_nodes)
        }
    } else {
        None // No children if the path is a file
    };

    Ok(DirEntry {
        path: path.to_string_lossy().to_string(),
        name,
        children,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_tree() {
        let filter = ["hkx", "xml", "json", "yaml"];
        let test_dir_path = "./../../d-merge/";

        match build_dir_tree(test_dir_path, filter) {
            Ok(directory_tree) => {
                fs::create_dir_all("../dummy/").unwrap();
                fs::write("../dummy/dump.txt", format!("{directory_tree:#?}")).unwrap();
            }
            Err(e) => panic!("Error building directory tree: {}", e),
        }
    }

    #[test]
    fn test() {
        use rayon::prelude::*;

        const FILTER: [&str; 4] = ["hkx", "xml", "json", "yaml"];
        // const FILTER: [&str; 2] = ["hkx", "xml"];
        let dirs = ["./../../d-merge/dummy"];

        let (entries, errors): (Vec<_>, Vec<_>) = dirs
            .par_iter()
            .map(|dir| build_dir_tree(dir, FILTER).map_err(|err| err.to_string()))
            .partition(Result::is_ok);

        // Collect only successful entries
        let entries: Vec<DirEntry> = entries.into_iter().map(Result::unwrap).collect();

        // Collect error messages and join them
        if !errors.is_empty() {
            let error_messages: Vec<String> = errors.into_iter().map(Result::unwrap_err).collect();
            panic!("{}", error_messages.join("\n"));
        }

        fs::create_dir_all("../dummy/").unwrap();
        fs::write("../dummy/dump.txt", format!("{entries:#?}")).unwrap();
    }
}

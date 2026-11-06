//! Paths built in one buffer.

use std::path::{Path, PathBuf};

/// `root/parts[0]/parts[1]/...`. Chained `Path::join` would allocate a path for each part.
///
/// e.g., `D:/out` and `[".d_merge", ".debug"]` -> `D:/out/.d_merge/.debug`
#[must_use]
#[inline]
pub fn joined(root: &Path, parts: &[&str]) -> PathBuf {
    let capacity = root.as_os_str().len() + parts.iter().map(|part| part.len() + 1).sum::<usize>();
    let mut path = PathBuf::with_capacity(capacity);
    path.push(root);
    for part in parts {
        path.push(part);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_the_same_as_chained_joins() {
        let root = Path::new("D:/out");
        assert_eq!(joined(root, &[".d_merge", ".debug"]), root.join(".d_merge").join(".debug"));
        assert_eq!(joined(root, &[]), root);
        assert_eq!(joined(Path::new(""), &["a"]), Path::new("a"));
    }

    #[test]
    fn does_not_grow_its_buffer() {
        let path = joined(Path::new("D:/out"), &["a", "bcd"]);
        assert!(path.capacity() >= path.as_os_str().len());
        assert!(path.capacity() <= "D:/out".len() + "a".len() + "bcd".len() + 2);
    }
}

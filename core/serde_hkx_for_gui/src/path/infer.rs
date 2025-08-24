use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Generate an output path based on the roots_path and input_path.
///
/// If input_path starts with any of the roots_path entries, the relative path
/// will be used. Otherwise, only the file stem will be appended.
pub fn generate_output_path<P>(input: P, output_dir: &str, strip_roots: &[String]) -> PathBuf
where
    P: AsRef<Path>,
{
    let input_path = input.as_ref();

    strip_roots
        .par_iter()
        .find_first(|root| input_path.starts_with(root))
        .map_or_else(
            || {
                let file_name = input_path.file_stem().unwrap_or_default();
                Path::new(output_dir).join(file_name)
            },
            |root| {
                let relative_path = input_path.strip_prefix(root).unwrap_or(input_path);
                Path::new(output_dir).join(relative_path)
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_output_path_with_root_match() {
        let output_dir = "output";
        let input = Path::new("root_dir/sub_dir/sub_dir2");
        let roots_path = vec!["root_dir".to_string()];
        let result = generate_output_path(input, output_dir, &roots_path);
        assert_eq!(result, Path::new("output/sub_dir/sub_dir2"));
    }

    #[test]
    fn test_generate_output_path_without_root_match() {
        let output_dir = "output";
        let input = Path::new("other_dir/file.hkx");
        let roots_path = vec!["root_dir".to_string()];

        let mut result = generate_output_path(input, output_dir, &roots_path);
        result.set_extension("xml");
        assert_eq!(result, Path::new("output/file.xml"));
    }

    #[test]
    fn test_generate_output_path_multiple_roots() {
        let output_dir = "output";
        let input = Path::new("another_root/sub_dir/file.hkx");
        let roots_path = vec!["root_dir".to_string(), "another_root".to_string()];

        let mut result = generate_output_path(input, output_dir, &roots_path);
        result.set_extension("xml");
        assert_eq!(result, Path::new("output/sub_dir/file.xml"));
    }
}

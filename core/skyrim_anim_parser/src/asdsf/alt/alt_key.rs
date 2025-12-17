/// Converts a `~`-separated identifier into a `\`-separated vanilla-style path,
/// appending `.txt` at the end. Writes the result into the provided `&mut String`.
///
/// # Returns
/// * `Some(())` if the conversion succeeded
/// * `None` if the input did not contain a `~` separator
pub fn to_normal_txt_project_name(s: &str, out: &mut String) -> Option<()> {
    let (a, b) = s.split_once('~')?;
    out.reserve(a.len() + b.len() + 5); // "\\" + ".txt" + margin
    out.push_str(a);
    out.push('\\');
    out.push_str(b);
    out.push_str(".txt");
    Some(())
}

/// Converts a vanilla-style path of the form `Folder\File.txt` into a `~`-separated
/// identifier and writes it into the provided `&mut String`.
///
/// # Returns
///
/// * `Some(())` if the path matches the expected format and the conversion succeeds
/// * `None` if the path is invalid, including (but not limited to):
///   - The path does not end with `.txt`
///   - The path contains fewer than two components
///   - The path contains more than two components (i.e. nested directories)
pub fn to_alt_txt_project_name(path: &str, out: &mut String) -> Option<()> {
    let path = path.strip_suffix(".txt")?;
    let mut parts = path.split('\\');
    let first = parts.next()?;
    let second = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    out.reserve(first.len() + second.len() + 1); // '~'
    out.push_str(first);
    out.push('~');
    out.push_str(second);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_normal_txt_project_name() {
        let input = "DefaultMaleData~DefaultMale";
        let expected = "DefaultMaleData\\DefaultMale.txt";
        let mut buf = String::new();
        to_normal_txt_project_name(input, &mut buf).unwrap();
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_to_alt_txt_project_name() {
        let input = "DefaultMaleData\\DefaultMale.txt";
        let expected = "DefaultMaleData~DefaultMale";
        let mut buf = String::new();
        to_alt_txt_project_name(input, &mut buf).unwrap();
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_invalid_to_alt_txt_project_name() {
        let input = "A\\B\\C.txt";
        let mut buf = String::new();
        assert_eq!(to_alt_txt_project_name(input, &mut buf), None);

        let input2 = "A\\B";
        assert_eq!(to_alt_txt_project_name(input2, &mut buf), None);
    }
}

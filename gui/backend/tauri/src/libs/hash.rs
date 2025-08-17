/// # Why use this?
/// The frontend selection can be deleted.
/// Therefore, the conversion status shifts when using index.
/// So, using hash from path solves this problem.
/// The exact same hash function is implemented in frontend and tested.
pub const fn hash_djb2(key: &str) -> u32 {
    let mut hash: u32 = 5381;
    let bytes = key.as_bytes();
    let mut i = 0;

    // NOTE: For const, it is necessary to loop with while instead of using for loop(iter).
    while i < bytes.len() {
        hash = ((hash << 5).wrapping_add(hash)) ^ (bytes[i] as u32);
        i += 1;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hash() {
        let input = "example";
        let hash1 = hash_djb2(input);
        let hash2 = hash_djb2(input);
        assert_eq!(
            hash1, hash2,
            "Different hash values were generated for the same input"
        );
    }

    #[test]
    fn test_different_hashes_for_different_inputs() {
        let hash1 = hash_djb2("example1");
        let hash2 = hash_djb2("example2");
        assert_ne!(
            hash1, hash2,
            "Same hash values were generated for different inputs"
        );
    }

    #[test]
    fn test_empty_string() {
        let hash = hash_djb2("");
        assert_eq!(
            hash, 5381,
            "Hash for empty string does not match the expected initial value"
        );
    }
}

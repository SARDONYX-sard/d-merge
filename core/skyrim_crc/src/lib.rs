//! ref: https://www.nexusmods.com/skyrim/articles/50508/
use crc::{Algorithm, Crc};
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// CRC32 for Skyrim (Poly: 0x04C11DB7, reverse data bytes, reverse CRC result)
const SKYRIM_CRC32_ALG: Algorithm<u32> = Algorithm {
    width: 32,
    poly: 0x04C11DB7,
    init: 0x00000000,
    refin: true,
    refout: true,
    xorout: 0x00000000,
    check: 0xCBF43926,
    residue: 0x00000000,
};

/// Calculate CRC32
#[inline]
pub const fn calc_crc32(input: &str) -> u32 {
    calc_crc32_from_bytes(input.as_bytes())
}

/// Calculate CRC32 from bytes.
#[inline]
pub const fn calc_crc32_from_bytes(input: &[u8]) -> u32 {
    let crc = Crc::<u32>::new(&SKYRIM_CRC32_ALG);
    crc.checksum(input)
}

/// Calculate CRC32 from [`u32`].
#[inline]
pub const fn calc_crc32_from_u32(input: u32) -> u32 {
    calc_crc32_from_bytes(&input.to_le_bytes())
}

/// Try to decode a CRC32 by brute force using parallel processing.
///
///
/// - `decode_crc32(7891816);` => `// Found match: [e4, 35, 83, 50](hex) | 1350776292 (dec)`
/// - u32::MAX process time: 42.30s(single: 220s)
pub fn decode_crc32(target_crc: u32) {
    let crc32 = Crc::<u32>::new(&SKYRIM_CRC32_ALG);
    let count = AtomicUsize::new(0);

    (0..=u32::MAX).into_par_iter().for_each(|data| {
        let bytes = data.to_le_bytes();
        let crc = crc32.checksum(&bytes);

        if crc == target_crc {
            let _ = count.fetch_add(1, Ordering::Relaxed);
            println!("Found match: {bytes:x?}(hex) | {data} (dec)");
        }
    });

    println!("Total matches found: {}", count.load(Ordering::Relaxed));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_calculations() {
        assert_eq!(
            calc_crc32("meshes\\actors\\dragon\\animations"),
            3692944883,
            "CRC32 for 'meshes\\actors\\dragon\\animations' is incorrect"
        );

        assert_eq!(
            calc_crc32("ground_bite"), // lowercase file stem, without ".hkx"
            3191128947,                // 0xbe34c373
            "CRC32 for 'ground_bite' is incorrect"
        );

        // In fact, a value of 7891816(0x786b68) is expected, but only `hkx` does not match the value.
        assert_eq!(
            calc_crc32("hkx"),
            2652099066,
            "CRC32 for '.hkx' is incorrect"
        );
    }
}

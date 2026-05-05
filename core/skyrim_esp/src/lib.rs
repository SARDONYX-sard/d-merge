//! # Dummy ESP Generator
//!
//! [Playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=3c8d00beb7a0efbcc7bdfc763801759b)
//!
//! Generates a minimal [Elder Scrolls Plugin (ESP)][esp] file containing only a
//! `TES4` header record. This is sufficient to act as a sentinel/dummy plugin
//! for tools like FNIS that require a plugin to be present on disk.
//!
//! ## ESP binary layout
//!
//! ```text
//! ┌─ TES4 record ─────────────────────────────────────────────┐
//! │  type       [4 bytes]  ASCII tag "TES4"                   │
//! │  data_size  [4 bytes]  u32 LE — byte length of body       │
//! │  flags      [4 bytes]  u32 LE — record flags (0 = none)   │
//! │  form_id    [4 bytes]  u32 LE — always 0 for TES4         │
//! │  timestamp  [4 bytes]  u32 LE — editor timestamp (unused) │
//! │  version_control [4 bytes] u32 LE (unused)                │
//! │                                                           │
//! │  ┌─ HEDR sub_record ──────────────────────────────────┐   │
//! │  │  tag      [4 bytes]  "HEDR"                        │   │
//! │  │  size     [2 bytes]  u16 LE — always 12            │   │
//! │  │  version  [4 bytes]  f32 LE — plugin format ver    │   │
//! │  │  num_recs [4 bytes]  u32 LE — non-Group record cnt │   │
//! │  │  next_id  [4 bytes]  u32 LE — next available FormID│   │
//! │  └────────────────────────────────────────────────────┘   │
//! │                                                           │
//! │  ┌─ CNAM sub_record (optional) ───────────────────────┐   │
//! │  │  tag   [4 bytes]  "CNAM"                           │   │
//! │  │  size  [2 bytes]  u16 LE                           │   │
//! │  │  data  [n bytes]  null-terminated UTF-8 author     │   │
//! │  └────────────────────────────────────────────────────┘   │
//! │                                                           │
//! │  ┌─ SNAM sub_record (optional) ───────────────────────┐   │
//! │  │  tag   [4 bytes]  "SNAM"                           │   │
//! │  │  size  [2 bytes]  u16 LE                           │   │
//! │  │  data  [n bytes]  null-terminated UTF-8 description│   │
//! │  └────────────────────────────────────────────────────┘   │
//! └───────────────────────────────────────────────────────────┘
//! ```
//!
//! [esp]: https://en.uesp.net/wiki/Skyrim_Mod:Mod_File_Format

use std::{
    ffi::CStr,
    io::{self, Write},
};

/// Configuration for the generated ESP header.
#[derive(Debug, PartialEq)]
pub struct Config {
    /// Plugin format version written into `HEDR`.
    pub version: Version,
    /// Optional plugin author string written into the `CNAM` sub_record.
    ///
    /// # Note
    /// Max size: 511 bytes + null terminator.(Truncated by the Creation Kit)
    pub author: Option<&'static CStr>,
    /// Optional plugin description written into the `SNAM` sub_record.
    /// Displayed in mod managers and the in-game load order screen.
    ///
    /// # Note
    /// Max size: 511 bytes + null terminator.
    pub description: Option<&'static CStr>,
}

/// https://en.uesp.net/wiki/Skyrim_Mod:Mod_File_Format/TES4
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Version {
    /// Skyrim SE
    Form44,
    /// Skyrim LE
    Form43,
}

impl Version {
    /// Skyrim SE uses `1.7`, LE uses `0.94` -> to_le_bytes.
    const fn to_le_bytes(self) -> [u8; 4] {
        match self {
            Self::Form44 => 1.7_f32,
            Self::Form43 => 0.94,
        }
        .to_le_bytes()
    }
}

/// Write a single sub_record directly into `w` without intermediate allocation.
///
/// Since the ESP format requires the TES4 record to contain `data_size` (the
/// total byte length of its body) *before* the body itself, we cannot stream
/// the outer TES4 header without knowing the body size up front.
/// [`body_size`] must therefore be computed by the caller via
/// [`sub_record_size`] before the first write.
///
/// # Layout written
///
/// ```text
/// [tag: 4 bytes][size: u16 LE][data: n bytes]
/// ```
#[inline]
fn write_sub_record(w: &mut impl Write, tag: &[u8; 4], data: &[u8]) -> io::Result<()> {
    let size = u16::try_from(data.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "FNIS.esp generation ERR(d_merge creator miss): sub_record data too large: {} bytes (max 65535)",
                data.len()
            ),
        )
    })?;
    w.write_all(tag)?;
    w.write_all(&size.to_le_bytes())?;
    w.write_all(data)
}

/// Return the encoded byte length of a sub_record without allocating.
///
/// Use this to pre-compute the TES4 `data_size` field.
///
/// ```text
/// sub_record_size(data) == 4 (tag) + 2 (size field) + data.len()
/// ```
#[inline]
const fn sub_record_size(data_len: usize) -> usize {
    4 + 2 + data_len
}

/// Write a minimal ESP file into `w`.
///
/// The entire file is written in one pass with no heap allocation beyond the
/// fixed-size HEDR payload on the stack. `w` should be a [`BufWriter`] when
/// targeting a [`File`] to amortize system_call overhead.
///
/// [`BufWriter`]: std::io::BufWriter
///
/// # Errors
///
pub fn write_dummy_esp<W>(w: &mut W, config: &Config) -> io::Result<()>
where
    W: Write,
{
    // --- Pre-compute body size (needed for the outer TES4 header) ----------
    // https://en.uesp.net/wiki/Skyrim_Mod:Mod_File_Format/TES4
    /// header len: f32 + u32 + u32, always fixed
    const HEADER_DATA_LEN: usize = 12;
    let mut body_size = sub_record_size(HEADER_DATA_LEN);

    if let Some(a) = config.author {
        body_size += sub_record_size(a.to_bytes_with_nul().len());
    }
    if let Some(d) = config.description {
        body_size += sub_record_size(d.to_bytes_with_nul().len());
    }

    // --- TES4 outer record header -------------------------------------------
    // | type     [4]  | ASCII "TES4"                        |
    // | data_size[4]  | u32 LE — byte length of body below  |
    // | flags    [4]  | u32 LE — 0 = no flags               |
    // | form_id  [4]  | u32 LE — always 0 for TES4          |
    // | timestamp[4]  | u32 LE — editor timestamp (unused)  |
    // | ver_ctrl [4]  | u32 LE — version control (unused)   |
    w.write_all(b"TES4")?;
    w.write_all(&(body_size as u32).to_le_bytes())?;
    // flags: 0x0 — deliberately NOT setting the ESL flag (0x200).
    // FNIS.esp must remain a plain ESP: tools such as FNIS Sexy Move
    // break when this dummy is ESL-flagged, even though it contains no records.
    // See: https://github.com/ShikyoKira/Project-New-Reign---Nemesis-Main/issues/128
    w.write_all(&0_u32.to_le_bytes())?; // flags
    w.write_all(&0_u32.to_le_bytes())?; // form_id
    w.write_all(&0_u32.to_le_bytes())?; // timestamp
    w.write_all(&0_u32.to_le_bytes())?; // version_control

    // --- Header sub_record -----------------------------------------------------
    // - https://en.uesp.net/wiki/Skyrim_Mod:Mod_File_Format/TES4
    //
    // Required. Fixed 12-byte payload:
    //   version       [4]  f32 LE — plugin format version (0.94 = LE, 1.7 = SE)
    //   num_records   [4]  u32 LE — non-GROUP record count excluding TES4
    //   next_object_id[4]  u32 LE — next FormID the CK would assign
    let mut header_data = [0_u8; HEADER_DATA_LEN];
    header_data[0..4].copy_from_slice(&config.version.to_le_bytes());
    header_data[4..8].copy_from_slice(&0_u32.to_le_bytes()); // num_records
    header_data[8..12].copy_from_slice(&0x0800_u32.to_le_bytes()); // next_object_id
    write_sub_record(w, b"HEDR", &header_data)?;

    // --- CNAM sub_record (optional) ------------------------------------------
    // Null-terminated UTF-8 string. Displayed as plugin author in mod managers.
    // Creation Kit truncates to 511 bytes + NUL.
    if let Some(author) = config.author {
        write_sub_record(w, b"CNAM", author.to_bytes_with_nul())?;
    }

    // --- SNAM sub_record (optional) ------------------------------------------
    // Null-terminated UTF-8 string. Displayed as plugin description in the
    // in-game load order screen and mod managers.
    if let Some(desc) = config.description {
        write_sub_record(w, b"SNAM", desc.to_bytes_with_nul())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn write_to_vec(config: &Config) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        write_dummy_esp(&mut buf, config).expect("write_dummy_esp failed");
        buf.into_inner()
    }

    /// Expected binary of a FNIS dummy ESP with:
    ///   version = 0.94, author = "SARDONYX", description = "FNIS dummy esp generated by d_merge."
    ///
    /// Verified against the reference hex dump:
    /// ```text
    /// 00000000: 5445 5334 4c00 0000 0000 0000 0000 0000  TES4L...........
    /// 00000010: 0000 0000 0000 0000 4845 4452 0c00 d7a3  ........HEDR....
    /// 00000020: 703f 0000 0000 0008 0000 434e 414d 0900  .?........CNAM..
    /// 00000030: 5341 5244 4f4e 5958 0053 4e41 4d25 0046  SARDONYX.SNAM%.F
    /// 00000040: 4e49 5320 6475 6d6d 7920 6573 7020 6765  NIS dummy esp ge
    /// 00000050: 6e65 7261 7465 6420 6279 2064 5f6d 6572  nerated by d_mer
    /// 00000060: 6765 2e00                               ge..
    /// ```
    #[rustfmt::skip]
    const EXPECTED: &[u8] = &[
        // TES4 outer record header (24 bytes)
        0x54, 0x45, 0x53, 0x34,         // type:          "TES4"
        0x4c, 0x00, 0x00, 0x00,         // data_size:     76 (body length)
        0x00, 0x00, 0x00, 0x00,         // flags:         0x0 (plain ESP, not ESL-flagged)
        0x00, 0x00, 0x00, 0x00,         // form_id:       0
        0x00, 0x00, 0x00, 0x00,         // timestamp:     0
        0x00, 0x00, 0x00, 0x00,         // version_control: 0

        // HEDR sub_record (4 + 2 + 12 = 18 bytes)
        0x48, 0x45, 0x44, 0x52,         // tag:           "HEDR"
        0x0c, 0x00,                     // size:          12
        0xd7, 0xa3, 0x70, 0x3f,         // version:       0.94_f32 LE
        0x00, 0x00, 0x00, 0x00,         // num_records:   0
        0x00, 0x08, 0x00, 0x00,         // next_object_id: 0x0800

        // CNAM sub_record (4 + 2 + 9 = 15 bytes): "SARDONYX\0"
        0x43, 0x4e, 0x41, 0x4d,         // tag:           "CNAM"
        0x09, 0x00,                     // size:          9 (8 chars + NUL)
        0x53, 0x41, 0x52, 0x44,         // 'S','A','R','D'
        0x4f, 0x4e, 0x59, 0x58,         // 'O','N','Y','X'
        0x00,                           // NUL terminator

        // SNAM sub_record (4 + 2 + 37 = 43 bytes): "FNIS dummy esp generated by d_merge.\0"
        0x53, 0x4e, 0x41, 0x4d,         // tag:           "SNAM"
        0x25, 0x00,                     // size:          37 (36 chars + NUL)
        0x46, 0x4e, 0x49, 0x53,         // 'F','N','I','S'
        0x20, 0x64, 0x75, 0x6d,         // ' ','d','u','m'
        0x6d, 0x79, 0x20, 0x65,         // 'm','y',' ','e'
        0x73, 0x70, 0x20, 0x67,         // 's','p',' ','g'
        0x65, 0x6e, 0x65, 0x72,         // 'e','n','e','r'
        0x61, 0x74, 0x65, 0x64,         // 'a','t','e','d'
        0x20, 0x62, 0x79, 0x20,         // ' ','b','y',' '
        0x64, 0x5f, 0x6d, 0x65,         // 'd','_','m','e'
        0x72, 0x67, 0x65, 0x2e,         // 'r','g','e','.'
        0x00,                           // NUL terminator
    ];

    #[test]
    fn write_dummy_esp_matches_reference_binary() {
        let config = Config {
            version: Version::Form43,
            author: Some(c"SARDONYX"),
            description: Some(c"FNIS dummy esp generated by d_merge."),
        };
        assert_eq!(write_to_vec(&config), EXPECTED);
    }

    #[test]
    fn le_hedr_version_bytes() {
        let config = Config {
            version: Version::Form43,
            author: None,
            description: None,
        };
        // offset 30..34: TES4 header (24) + HEDR tag (4) + HEDR size (2) = 30
        assert_eq!(&write_to_vec(&config)[30..34], &[0xd7, 0xa3, 0x70, 0x3f]);
    }

    #[test]
    fn se_hedr_version_bytes() {
        let config = Config {
            version: Version::Form44,
            author: None,
            description: None,
        };
        assert_eq!(&write_to_vec(&config)[30..34], &[0x9a, 0x99, 0xd9, 0x3f]);
    }

    #[test]
    fn write_dummy_esp_no_optional_fields() {
        let config = Config {
            version: Version::Form44,
            author: None,
            description: None,
        };

        let bytes = write_to_vec(&config);

        let actual_data_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(
            actual_data_size, 18,
            "Without CNAM/SNAM the body is only the HEDR sub_record (18 bytes)"
        );
        assert_eq!(bytes.len(), 24 + 18, "total file size mismatch");

        // Must still be a valid TES4 record.
        assert_eq!(&bytes[0..4], b"TES4");
        assert_eq!(&bytes[24..28], b"HEDR");
    }

    #[test]
    fn write_dummy_esp_flags_should_be_non_esl_flag() {
        // See: https://github.com/ShikyoKira/Project-New-Reign---Nemesis-Main/issues/128
        let config = Config {
            version: Version::Form44,
            author: None,
            description: None,
        };

        let bytes = write_to_vec(&config);

        let flags = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert_eq!(flags & 0x200, 0, "ESL flag must not be set on FNIS.esp");
    }
}

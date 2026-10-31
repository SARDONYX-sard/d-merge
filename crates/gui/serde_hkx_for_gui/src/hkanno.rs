//! # hkanno
//!
//! `hkanno` is a utility tool designed to extract and modify internal data from
//! `hkaSplineCompressedAnimation` fields in a custom text format.
//!
//! It provides a lightweight and human-editable representation of animation annotations,
//! allowing users to inspect and rewrite the embedded annotation data used in Havok animation assets.
//!
//! ## Output Format (hkanno v2)
//!
//! ```text
//! # hkanno v2
//! # numOriginalFrames: <usize>        <- hkaSplineCompressedAnimation.numFrames
//! # duration: <f32>                   <- hkaSplineCompressedAnimation.duration
//! # numAnnotationTracks: <usize>      <- hkaSplineCompressedAnimation.annotationTracks.len()
//!
//! trackName: <String>                 <- hkaAnnotationTrack.trackName
//! # numAnnotations: <usize>           <- hkaAnnotationTrack.annotations.len()
//! <time: f32> <text: StringPtr>       <- hkaAnnotationTrack.annotations[n].time, text
//! <time: f32> <text: StringPtr>
//! ...
//!
//! trackName: <String>
//! # numAnnotations: <usize>
//! <time: f32> <text: StringPtr>
//! ...
//! ```
//!
//! ## Sample
//!
//! ```txt
//! # hkanno v2
//! # numOriginalFrames: 38
//! # duration: 1.5
//! # numAnnotationTracks: 3
//!
//! trackName: PairedRoot
//! # numAnnotations: 3
//! 0.100000 MCO_DodgeOpen
//! 0.400000 MCO_DodgeClose
//! 0.900000 MCO_Recovery
//!
//! trackName: 2_
//! # numAnnotations: 0
//!
//! trackName: Foot_L
//! # numAnnotations: 2
//! 0.250000 MCO_Step
//! 0.900000 MCO_Land
//! ```
//!
//! ## Parsing Rules
//!
//! - Lines beginning with `#` are treated as comments or metadata.
//! - `trackName:` lines are **not comments** — they are structural delimiters that
//!   explicitly mark the start of a new annotation track.
//! - Each `trackName` section must be immediately followed by one `# numAnnotations` line
//!   and then a list of `<time> <text>` pairs.
//! - The order of tracks is significant and is preserved when writing back into HKX data.
//! - Empty lines are ignored.
//!
//! ## Design Notes
//!
//! This format is designed for both human readability and strict structure.
//! Each `trackName` block maps 1:1 with a `hkaAnnotationTrack`, making it possible
//! to freely edit annotations (add/remove/reorder) while maintaining a stable mapping
//! back to the Havok binary structure.
//!
//! The first metadata lines (`numOriginalFrames`, `duration`, `numAnnotationTracks`)
//! represent global animation properties, followed by per-track sections.
//!
//! This makes the format easy to diff, version control, or reserialize back into
//! the Havok binary format for modding, debugging, or data analysis purposes.

use std::{borrow::Cow, fmt, path::Path};

use havok_classes::Classes;
use rayon::prelude::*;
use serde_hkx_features::ClassMap;
pub use serde_hkx_features::Format;
use snafu::ResultExt as _;

/// # hkanno module
///
/// Provides a structured representation of Havok animation annotations extracted
/// from `hkaSplineCompressedAnimation` objects inside HKX files.
///
/// The primary purpose of this module is to allow reading, editing, and re-serializing
/// the embedded annotation data in a lightweight and human-readable format.
///
/// This module supports both borrowed (`Cow::Borrowed`) and owned (`Cow::Owned`) data,
/// enabling zero-copy extraction when possible.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Hkanno<'a> {
    /// `hkaSplineCompressedAnimation` index. e.g. `#0003`
    pub ptr: Cow<'a, str>,
    /// Number of frames in the original animation.
    pub num_original_frames: i32,
    /// Total duration (in seconds) of the animation.
    pub duration: f32,
    /// A list of annotation tracks, each containing time–text pairs.
    pub annotation_tracks: Vec<AnnotationTrack<'a>>,
}

impl<'a> Hkanno<'a> {
    /// Converts a borrowed Hkanno into an owned `'static` Hkanno.
    pub fn into_static(self) -> Hkanno<'static> {
        Hkanno {
            ptr: Cow::Owned(self.ptr.into_owned()),
            num_original_frames: self.num_original_frames,
            duration: self.duration,
            annotation_tracks: self
                .annotation_tracks
                .into_par_iter()
                .map(|track| AnnotationTrack {
                    track_name: track.track_name.map(|t| Cow::Owned(t.into_owned())),
                    annotations: track
                        .annotations
                        .into_par_iter()
                        .map(|ann| Annotation {
                            time: ann.time,
                            text: ann.text.map(|t| Cow::Owned(t.into_owned())),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// Write the edited Hkanno back into an existing ClassMap
    ///
    /// # Errors
    /// If missing/multiple `hkaSplineCompressedAnimation`.
    pub fn write_to_classmap(self, class_map: &mut ClassMap<'a>) -> Result<(), HkannoError> {
        use havok_classes::Classes;
        use havok_types::{I32, StringPtr};

        let mut animations: Vec<_> =
            class_map.par_iter_mut().filter(|(_, class)| is_hka_animation_derived(class)).collect();
        let (_, animation_class) = {
            match animations.len() {
                0 => return MissingHkaAnimationClassSnafu.fail(),
                1 => animations.swap_remove(0),
                _ => {
                    return Err(HkannoError::MultipleHkaAnimationFound { count: animations.len() });
                }
            }
        };

        let (num_original_frames, duration, annotation_tracks) = match animation_class {
            Classes::hkaAnimation(class) => {
                (&mut I32::Number(0), &mut class.m_duration, &mut class.m_annotationTracks)
            }
            Classes::hkaDeltaCompressedAnimation(class) => (
                &mut I32::Number(0),
                &mut class.parent.m_duration,
                &mut class.parent.m_annotationTracks,
            ),
            Classes::hkaInterleavedUncompressedAnimation(class) => (
                &mut I32::Number(0),
                &mut class.parent.m_duration,
                &mut class.parent.m_annotationTracks,
            ),
            Classes::hkaQuantizedAnimation(class) => (
                &mut I32::Number(0),
                &mut class.parent.m_duration,
                &mut class.parent.m_annotationTracks,
            ),
            Classes::hkaSplineCompressedAnimation(class) => (
                &mut class.m_numFrames,
                &mut class.parent.m_duration,
                &mut class.parent.m_annotationTracks,
            ),
            Classes::hkaWaveletCompressedAnimation(class) => (
                &mut I32::Number(0),
                &mut class.parent.m_duration,
                &mut class.parent.m_annotationTracks,
            ),
            _ => return Err(HkannoError::MissingHkaAnimationClass),
        };

        // Safety: Assuming the frontend correctly maintains the following values, they can be overwritten.
        *num_original_frames = I32::Number(self.num_original_frames);
        *duration = self.duration;

        *annotation_tracks = self
            .annotation_tracks
            .into_par_iter()
            .map(|track| havok_classes::hkaAnnotationTrack {
                __ptr: None,
                m_trackName: StringPtr::new(track.track_name),
                m_annotations: track
                    .annotations
                    .into_par_iter()
                    .map(|ann| havok_classes::hkaAnnotationTrackAnnotation {
                        __ptr: None,
                        m_time: ann.time,
                        m_text: StringPtr::from_option(ann.text),
                    })
                    .collect(),
            })
            .collect();

        Ok(())
    }

    /// Updates the given HKX/XML file bytes with the annotation data in `self`.
    ///
    /// This function performs no file I/O. The caller is responsible for reading
    /// and writing the file contents. It only mutates and serializes the
    /// in-memory `ClassMap`.
    ///
    /// # Arguments
    /// * `bytes` - Raw HKX or XML file bytes.
    /// * `format` - output format
    /// * `input` - The source file path (used only for error context and extension check).
    ///
    /// # Returns
    /// A new byte vector containing the updated HKX data.
    ///
    /// # Errors
    /// Returns a [`HkannoError`] if:
    /// - Deserialization of the input bytes fails.
    /// - Annotation update fails.
    /// - Serialization of the updated data fails.
    #[inline(never)] // keep the generic (de)serialization in this crate (see `parse_as_hkanno`)
    pub fn update_hkx_bytes(
        self,
        bytes: &[u8],
        output_format: Format,
        input: &Path,
    ) -> Result<Vec<u8>, HkannoError> {
        let mut output = Vec::new();
        hkx_io::with_class_map(bytes, input, &mut |mut classes| {
            self.clone().into_static().write_to_classmap(&mut classes).map_err(|e| {
                serde_hkx_features::error::Error::IoError { source: std::io::Error::other(e) }
            })?;

            output = match output_format {
                Format::Amd64 | Format::Win32 | Format::Xml => {
                    serde_hkx_features::serde::ser::to_bytes(&mut classes, output_format)
                }
                Format::Json | Format::Toml => {
                    let mut classes =
                        serde_hkx_features::types_wrapper::ClassPtrMap::from_class_map(classes);
                    serde_hkx_features::serde_extra::ser::to_bytes(&mut classes, output_format)
                }
            }
            .with_context(|_| serde_hkx_features::error::SerSnafu { input: input.to_path_buf() })?;
            Ok(())
        })
        .context(SerdeHkxFeatureSnafu)?;
        Ok(output)
    }
}

/// Represents a single annotation track extracted from a Havok animation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnnotationTrack<'a> {
    /// The name of this annotation track (e.g. `PairedRoot`, `2_`, etc.).
    /// Corresponds to `hkaAnnotationTrack.trackName`.
    pub track_name: Option<Cow<'a, str>>,

    /// The collection of annotation entries in this track.
    pub annotations: Vec<Annotation<'a>>,
}

/// Represents a single annotation event, consisting of a timestamp and a text string.
///
/// The `text` field uses `Cow<'a, str>` so that data may be borrowed
/// directly from the parsed HKX data or owned after conversion.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Annotation<'a> {
    /// The time (in seconds) at which this annotation occurs.
    pub time: f32,
    /// The annotation text, typically referencing an event or signal name.
    pub text: Option<Cow<'a, str>>,
}

impl<'a> fmt::Display for Hkanno<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Header (global animation properties)
        writeln!(f, "# hkanno v2")?;
        writeln!(f, "# numOriginalFrames: {}", self.num_original_frames)?;
        writeln!(f, "# duration: {}", self.duration)?;
        writeln!(f, "# numAnnotationTracks: {}", self.annotation_tracks.len())?;
        writeln!(f)?;

        // Each track block
        for track in &self.annotation_tracks {
            writeln!(
                f,
                "trackName: {}",
                track.track_name.as_deref().unwrap_or(havok_types::NULL_STR)
            )?;
            writeln!(f, "# numAnnotations: {}", track.annotations.len())?;

            for ann in &track.annotations {
                let text = ann.text.as_deref().unwrap_or(havok_types::NULL_STR);
                writeln!(f, "{:.6} {}", ann.time, text)?;
            }

            // Separate tracks by one blank line
            writeln!(f)?;
        }

        Ok(())
    }
}

/// Parses the annotation part of the hkanno text (the inverse of [`Hkanno`]'s `Display`).
///
/// Only `annotation_tracks` can be edited as text; `ptr`, `numOriginalFrames` and
/// `duration` come from the original file, so the caller keeps them.
///
/// Strings are borrowed from `text`.
///
/// # Rules
/// - blank lines and `#` lines are ignored
/// - `trackName: <name>` (case-insensitive) starts a new track; `␀` means "no name"
/// - `<time> <text>` adds an annotation to the current track
///   (a dummy unnamed track is created when no `trackName:` came before)
/// - an unparsable time becomes `NaN` (the editor's diagnostics report it before saving)
///
/// # Example
/// ```
/// use serde_hkx_for_gui::hkanno::annotation_tracks_from_text;
///
/// let tracks = annotation_tracks_from_text("trackName: Root\n0.100000 MCO_DodgeOpen");
/// assert_eq!(tracks[0].track_name.as_deref(), Some("Root"));
/// assert_eq!(tracks[0].annotations[0].text.as_deref(), Some("MCO_DodgeOpen"));
/// ```
pub fn annotation_tracks_from_text(text: &str) -> Vec<AnnotationTrack<'_>> {
    /// `␀` is how `Display` writes a null string.
    fn non_null(s: &str) -> Option<Cow<'_, str>> {
        (s != havok_types::NULL_STR).then_some(Cow::Borrowed(s))
    }

    let mut tracks = Vec::new();
    let mut current: Option<AnnotationTrack<'_>> = None;

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        // e.g., `trackName: Root`
        if let Some((head, name)) = line.split_once(':')
            && head.trim_end().eq_ignore_ascii_case("trackName")
        {
            tracks.extend(current.take());
            current = Some(AnnotationTrack {
                track_name: non_null(name.trim()),
                annotations: Vec::new(),
            });
            continue;
        }

        if line.starts_with('#') {
            continue;
        }

        // e.g., `0.100000 MCO_DodgeOpen`
        let (time, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        current
            .get_or_insert_with(|| AnnotationTrack { track_name: None, annotations: Vec::new() })
            .annotations
            .push(Annotation { time: parse_float_prefix(time), text: non_null(rest.trim()) });
    }

    tracks.extend(current);
    tracks
}

/// JS `parseFloat`: the longest leading number, or `NaN`.
///
/// e.g., `"0.5s"` -> `0.5`, `"abc"` -> `NaN`
fn parse_float_prefix(s: &str) -> f32 {
    let end = s
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E')))
        .unwrap_or(s.len());
    // Shrink until it parses, e.g., `1e` -> `1`.
    (1..=end).rev().find_map(|len| s[..len].parse::<f32>().ok()).unwrap_or(f32::NAN)
}

/// Parses a borrowed `Hkanno` structure from an already deserialized `ClassMap`.
///
/// This function expects a `ClassMap` containing Havok animation data and extracts
/// annotation tracks and entries without cloning strings unnecessarily.
/// All annotation text is returned as `Cow<'a, str>` references into the `ClassMap`.
///
/// # Behavior
///
/// - Searches for the class derived from `hkaAnimation` (e.g., `hkaSplineCompressedAnimation`) in the `ClassMap`.
/// - If none is found, returns [`HkannoError::MissingHkaAnimationClass`].
/// - If multiple are found, returns [`HkannoError::MultipleHkaAnimationFound`] with the count.
/// - Each annotation track and annotation is converted into [`AnnotationTrack`] and [`Annotation`] structures.
///
/// # Errors
///
/// Returns a [`HkannoError`] for any of the following cases:
///
/// - [`HkannoError::MissingHkaAnimationClass`] – no `hkaAnimation` found in the `ClassMap`.
/// - [`HkannoError::MultipleHkaAnimationFound`] – more than one found.
/// - [`HkannoError::UnsupportedI32Variant`] – the number-of-frames field is an unsupported variant (`EventId` or `VariableId`).
pub fn parse_hkanno_borrowed<'a>(class_map: ClassMap<'a>) -> Result<Hkanno<'a>, HkannoError> {
    use havok_types::I32;

    // Find the one `hkaAnimation`
    let (ptr, animation_class) = {
        // Find C++ classes that inherit from `hkaAnimation` C++
        let mut animation_classes: Vec<_> = class_map
            .into_par_iter()
            .filter(|(_, class)| is_hka_animation_derived(class))
            .collect();

        match animation_classes.len() {
            0 => return MissingHkaAnimationClassSnafu.fail(),
            1 => animation_classes.swap_remove(0),
            _ => {
                return Err(HkannoError::MultipleHkaAnimationFound {
                    count: animation_classes.len(),
                });
            }
        }
    };

    const FPS: f32 = 30.0;
    let (num_original_frames, duration, annotation_tracks): (f32, _, _) = match animation_class {
        Classes::hkaAnimation(class) => {
            (class.m_duration * FPS, class.m_duration, class.m_annotationTracks)
        }
        Classes::hkaDeltaCompressedAnimation(class) => (
            class.parent.m_duration * FPS,
            class.parent.m_duration,
            class.parent.m_annotationTracks,
        ),
        Classes::hkaInterleavedUncompressedAnimation(class) => (
            class.parent.m_duration * FPS,
            class.parent.m_duration,
            class.parent.m_annotationTracks,
        ),
        Classes::hkaQuantizedAnimation(class) => (
            class.parent.m_duration * FPS,
            class.parent.m_duration,
            class.parent.m_annotationTracks,
        ),
        Classes::hkaSplineCompressedAnimation(class) => (
            match &class.m_numFrames {
                I32::Number(n) => *n as f32,
                I32::EventId(event) => {
                    return Err(HkannoError::UnsupportedI32Variant { variant: event.to_string() });
                }
                I32::VariableId(var) => {
                    return Err(HkannoError::UnsupportedI32Variant { variant: var.to_string() });
                }
            },
            class.parent.m_duration,
            class.parent.m_annotationTracks,
        ),
        Classes::hkaWaveletCompressedAnimation(class) => (
            class.parent.m_duration * FPS,
            class.parent.m_duration,
            class.parent.m_annotationTracks,
        ),
        _ => return Err(HkannoError::MissingHkaAnimationClass),
    };

    let tracks = annotation_tracks
        .into_par_iter()
        .map(|track| {
            let annotations = track
                .m_annotations
                .into_par_iter()
                .map(|ann| Annotation { time: ann.m_time, text: ann.m_text.into_inner() })
                .collect::<Vec<_>>();
            AnnotationTrack { track_name: track.m_trackName.into_inner(), annotations }
        })
        .collect::<Vec<_>>();

    Ok(Hkanno {
        ptr,
        num_original_frames: num_original_frames.ceil() as i32, // Rather than adding 1, it's likely rounding up.
        duration,
        annotation_tracks: tracks,
    })
}

/// Does this class inherit from `hkaAnimation`?
const fn is_hka_animation_derived(class: &Classes<'_>) -> bool {
    matches!(
        class,
        Classes::hkaAnimation(_)
            | Classes::hkaDeltaCompressedAnimation(_)
            | Classes::hkaInterleavedUncompressedAnimation(_)
            | Classes::hkaQuantizedAnimation(_)
            | Classes::hkaSplineCompressedAnimation(_)
            | Classes::hkaWaveletCompressedAnimation(_)
    )
}

/// Parses a HKX or XML file into an `Hkanno` structure directly from raw bytes.
///
/// This function is a convenience wrapper that deserializes the input bytes
/// using `serde_hkx_features::serde::de::deserialize` and then calls
/// [`parse_hkanno_borrowed`] to extract animation annotation data.
///
/// # Arguments
///
/// * `bytes` - Raw file contents.
/// * `input` - File path, used for error reporting and extension checking.
///
/// # Behavior
///
/// - Automatically detects and parses `.hkx` or `.xml` files.
/// - Returns an [`Hkanno`] structure containing annotation tracks and entries as `Cow<'a, str>`.
///
/// # Errors
///
/// Returns a [`HkannoError`] if:
///
/// - The bytes cannot be deserialized ([`HkannoError::SerdeHkxFeatureError`]),
///   e.g., an unsupported extension or broken data.
/// - No / multiple `hkaAnimation` classes exist
///   ([`HkannoError::MissingHkaAnimationClass`], [`HkannoError::MultipleHkaAnimationFound`]).
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use std::error::Error;
///
/// use serde_hkx_for_gui::hkanno::parse_as_hkanno;
///
/// fn example() -> Result<(), Box<dyn Error>> {
///     let path = Path::new("example.hkx"); // or xml(from hkx)
///     let bytes = std::fs::read(path)?;
///
///     let hkanno = parse_as_hkanno(&bytes, path)?;
///
///     println!("Number of frames: {}", hkanno.num_original_frames);
///     Ok(())
/// }
/// ```
///
/// # Binary size
/// `#[inline(never)]`: the deserialization inside is huge generic code; inlined into a caller
/// crate it would be copied there (~15 MB in a non-LTO build).
#[inline(never)]
pub fn parse_as_hkanno<'a>(bytes: &'a [u8], input: &Path) -> Result<Hkanno<'a>, HkannoError> {
    // Owned (`into_static`): XML is decoded into a temporary string the classes borrow from.
    let mut hkanno = None;
    hkx_io::with_class_map(bytes, input, &mut |classes| {
        let parsed = parse_hkanno_borrowed(classes).map_err(|e| {
            serde_hkx_features::error::Error::IoError { source: std::io::Error::other(e) }
        })?;
        hkanno = Some(parsed.into_static());
        Ok(())
    })
    .context(SerdeHkxFeatureSnafu)?;
    hkanno.ok_or(HkannoError::MissingHkaAnimationClass)
}

/// Custom error type for hkanno parsing operations.
#[derive(Debug, snafu::Snafu)]
pub enum HkannoError {
    /// Raised when the HKX data could not be parsed into a valid ClassMap.
    #[snafu(display("internal serde_hkx_features err: {source}"))]
    SerdeHkxFeatureError { source: serde_hkx_features::error::Error },

    /// No `hkaAnimation`-derived class found
    MissingHkaAnimationClass,

    /// expected one `hkaAnimation` per `hkx`, but multiple were obtained. Got count: {count}
    MultipleHkaAnimationFound { count: usize },

    /// Raised when an unsupported I32 variant was encountered.
    #[snafu(display("Unsupported i32 in animation field: {variant}"))]
    UnsupportedI32Variant { variant: String },

    /// Raised when file IO fails.
    #[snafu(display("Failed to read file: {source}"))]
    IoError { source: std::io::Error },
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, path::Path};

    use super::*;

    #[test]
    fn test_hkanno_to_string_format() {
        // Dummy Hkanno with 2 tracks and optional text
        let hkanno = Hkanno {
            ptr: Cow::Borrowed("#0003"),
            num_original_frames: 10,
            duration: 0.8,
            annotation_tracks: vec![
                AnnotationTrack {
                    track_name: Some(Cow::Borrowed("Track1")),
                    annotations: vec![
                        Annotation { time: 0.1, text: Some(Cow::Borrowed("Start")) },
                        Annotation { time: 0.5, text: Some(Cow::Borrowed("Mid")) },
                    ],
                },
                AnnotationTrack {
                    track_name: Some(Cow::Borrowed("Track2")),
                    annotations: vec![
                        Annotation { time: 0.3, text: Some(Cow::Borrowed("Alt1")) },
                        Annotation {
                            time: 0.7,
                            text: None, // missing text
                        },
                    ],
                },
            ],
        };

        assert_eq!(&hkanno.ptr, "#0003");

        let output = hkanno.to_string();

        // Header checks
        assert!(output.contains("# numOriginalFrames: 10"));
        assert!(output.contains("# duration: 0.8"));
        assert!(output.contains("# numAnnotationTracks: 2"));

        // Track1 checks
        assert!(output.contains("trackName: Track1"));
        assert!(output.contains("# numAnnotations: 2"));
        assert!(output.contains("0.100000 Start"));
        assert!(output.contains("0.500000 Mid"));

        // Track2 checks
        assert!(output.contains("trackName: Track2"));
        assert!(output.contains("0.300000 Alt1"));
        // None text replaced by NULL_STR
        assert!(output.contains("0.700000 \u{2400}"));
    }

    #[test]
    #[ignore = "Requires local file"]
    fn test_parse_as_hkanno_from_file_path() {
        let path = "../../../dummy/convert/input/MCO_Dodge-B-1.xml";
        let bytes = std::fs::read(path).expect("Failed to read test HKX file");

        let hkanno =
            parse_as_hkanno(&bytes, Path::new(path)).expect("Failed to parse HKX file as Hkanno");

        dbg!(&hkanno);

        println!("Parsed Hkanno:\n{}", hkanno);
    }
}

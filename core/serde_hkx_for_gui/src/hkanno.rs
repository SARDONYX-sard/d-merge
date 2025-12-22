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

use havok_classes::Classes;
use rayon::prelude::*;
use serde_hkx_features::ClassMap;
use snafu::ResultExt as _;
use std::{borrow::Cow, fmt, path::Path};

pub use serde_hkx_features::OutFormat;

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
        use havok_types::{StringPtr, I32};

        let mut animations: Vec<_> = class_map
            .par_iter_mut()
            .filter(|(_, class)| is_hka_animation_derived(class))
            .collect();
        let (_, animation_class) = {
            match animations.len() {
                0 => return MissingHkaAnimationClassSnafu.fail(),
                1 => animations.swap_remove(0),
                _ => {
                    return Err(HkannoError::MultipleHkaAnimationFound {
                        count: animations.len(),
                    })
                }
            }
        };

        let (num_original_frames, duration, annotation_tracks) = match animation_class {
            Classes::hkaAnimation(class) => (
                &mut I32::Number(0),
                &mut class.m_duration,
                &mut class.m_annotationTracks,
            ),
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
    pub fn update_hkx_bytes(
        self,
        bytes: &mut Vec<u8>,
        format: OutFormat,
        input: &Path,
    ) -> Result<Vec<u8>, HkannoError> {
        let mut text = String::new();

        // Deserialize bytes → ClassMap
        let mut class_map: ClassMap<'_> =
            serde_hkx_features::serde_extra::de::deserialize(bytes, &mut text, input)
                .context(SerdeHkxFeatureSnafu)?;
        self.write_to_classmap(&mut class_map)?; // Update annotations (pure memory operation)

        // Serialize back to bytes(NOTE: Binary data requires pre-sorting, so it is marked as &mut class_map.)
        let updated_bytes = match format {
            OutFormat::Amd64 | OutFormat::Win32 | OutFormat::Xml => {
                serde_hkx_features::serde::ser::to_bytes(input, format, &mut class_map)
            }
            OutFormat::Json | OutFormat::Toml => {
                let mut class_map =
                    serde_hkx_features::types_wrapper::ClassPtrMap::from_class_map(class_map);
                serde_hkx_features::serde_extra::ser::to_bytes(input, format, &mut class_map)
            }
            _ => unreachable!("This being called means a new format type has been created."),
        }
        .context(SerdeHkxFeatureSnafu)?;

        Ok(updated_bytes)
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

/// Parses a borrowed `Hkanno` structure from an already deserialized `ClassMap`.
///
/// This function expects a `ClassMap` containing Havok animation data and extracts
/// annotation tracks and entries without cloning strings unnecessarily.
/// All annotation text is returned as `Cow<'a, str>` references into the `ClassMap`.
///
/// # Behavior
///
/// - Searches for `hkaSplineCompressedAnimation` objects in the `ClassMap`.
/// - If no spline is found, returns [`HkannoError::MissingSpline`].
/// - If multiple splines are found, returns [`HkannoError::MultipleSplinesFound`] with the count.
/// - Each annotation track and annotation is converted into [`AnnotationTrack`] and [`Annotation`] structures.
///
/// # Errors
///
/// Returns a [`HkannoError`] for any of the following cases:
///
/// - [`HkannoError::MissingSpline`] – no spline found in the `ClassMap`.
/// - [`HkannoError::MultipleSplinesFound`] – more than one spline found.
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
                })
            }
        }
    };

    const FPS: f32 = 30.0;
    let (num_original_frames, duration, annotation_tracks): (f32, _, _) = match animation_class {
        Classes::hkaAnimation(class) => (
            class.m_duration * FPS,
            class.m_duration,
            class.m_annotationTracks,
        ),
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
                    return Err(HkannoError::UnsupportedI32Variant {
                        variant: event.to_string(),
                    });
                }
                I32::VariableId(var) => {
                    return Err(HkannoError::UnsupportedI32Variant {
                        variant: var.to_string(),
                    });
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
                .map(|ann| Annotation {
                    time: ann.m_time,
                    text: ann.m_text.into_inner(),
                })
                .collect::<Vec<_>>();
            AnnotationTrack {
                track_name: track.m_trackName.into_inner(),
                annotations,
            }
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
/// * `bytes` - Raw file contents as a byte vector (`Vec<u8>`).
/// * `text` - Mutable `String` buffer used to avoid XML ownership or lifetime issues during deserialization.
/// * `path` - File path, used for error reporting and extension checking.
///
/// # Behavior
///
/// - Automatically detects and parses `.hkx` or `.xml` files.
/// - Populates the provided `text` buffer with intermediate string data if necessary.
/// - Returns an [`Hkanno`] structure containing annotation tracks and entries as `Cow<'a, str>`
///   references to the deserialized data.
///
/// # Errors
///
/// Returns a [`HkannoError`] if:
///
/// - The input file extension is missing or not `.hkx` / `.xml` ([`HkannoError::MissingExtension`]).
/// - The bytes cannot be parsed correctly ([`HkannoError::ParseError`]).
/// - Any other internal deserialization or annotation extraction error occurs.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use std::error::Error;
///
/// use serde_hkx_for_gui::hkanno::{parse_as_hkanno, HkannoError};
///
/// fn example() -> Result<(), Box<dyn Error>> {
///     let path = Path::new("example.hkx"); // or xml(from hkx)
///     let bytes = std::fs::read(path)?;
///     let mut buffer = String::new(); // To avoid ownership error xml receiver.
///
///     // parse_as_hkanno returns Result<Hkanno, HkannoError>
///     let hkanno = parse_as_hkanno(&bytes, &mut buffer, path)?;
///
///     println!("Number of frames: {}", hkanno.num_original_frames);
///     Ok(())
/// }
#[inline]
pub fn parse_as_hkanno<'a>(
    bytes: &'a Vec<u8>,
    text: &'a mut String,
    path: &Path,
) -> Result<Hkanno<'a>, HkannoError> {
    let class_map: ClassMap<'a> = serde_hkx_features::serde::de::deserialize(bytes, text, path)
        .context(SerdeHkxFeatureSnafu)?;
    parse_hkanno_borrowed(class_map)
}

/// Custom error type for hkanno parsing operations.
#[derive(Debug, snafu::Snafu)]
pub enum HkannoError {
    /// Raised when the HKX data could not be parsed into a valid ClassMap.
    #[snafu(display("internal serde_hkx_features err: {source}"))]
    SerdeHkxFeatureError {
        source: serde_hkx_features::error::Error,
    },

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
    use super::*;
    use std::{borrow::Cow, path::Path};

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
                        Annotation {
                            time: 0.1,
                            text: Some(Cow::Borrowed("Start")),
                        },
                        Annotation {
                            time: 0.5,
                            text: Some(Cow::Borrowed("Mid")),
                        },
                    ],
                },
                AnnotationTrack {
                    track_name: Some(Cow::Borrowed("Track2")),
                    annotations: vec![
                        Annotation {
                            time: 0.3,
                            text: Some(Cow::Borrowed("Alt1")),
                        },
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
        let path = "../../dummy/convert/input/MCO_Dodge-B-1.xml";
        let bytes = std::fs::read(path).expect("Failed to read test HKX file");
        let mut buffer = String::new();

        let hkanno = parse_as_hkanno(&bytes, &mut buffer, Path::new(path))
            .expect("Failed to parse HKX file as Hkanno");

        dbg!(&hkanno);

        println!("Parsed Hkanno:\n{}", hkanno);
    }
}

//! # hkanno
//!
//! `hkanno` is a utility tool designed to extract and modify internal data from
//! `hkaSplineCompressedAnimation` fields in a custom format.
//!
//! It provides a lightweight and editable representation of animation annotations,
//! allowing users to inspect and rewrite the embedded data used in Havok animation assets.
//!
//! ## Output Format
//!
//! ```text
//! # numOriginalFrames: <usize>        <- hkaSplineCompressedAnimation.numFrames
//! # duration: <f32>                   <- hkaSplineCompressedAnimation.duration
//! # numAnnotationTracks: <usize>      <- hkaSplineCompressedAnimation.annotationTracks.len()
//! # numAnnotations: <usize>           <- hkaSplineCompressedAnimation.annotationTracks[0].hkaAnnotationTrack.annotations.len()
//! <time: f32> <text: StringPtr>       <- hkaSplineCompressedAnimation.annotationTracks[0].hkaAnnotationTrack.annotations[0].time, text
//! <time: f32> <text: StringPtr>
//! ...
//! # numAnnotations: <usize>           <- hkaSplineCompressedAnimation.annotationTracks[1].hkaAnnotationTrack.annotations.len()
//! <time: f32> <text: StringPtr>
//! <time: f32> <text: StringPtr>
//! ...
//! ```
//!
//! ## Sample
//!
//! ```txt
//! # numOriginalFrames: 38
//! # duration: 1.5
//! # numAnnotationTracks: 97
//! # numAnnotations: 38
//! 0.100000 MCO_DodgeOpen
//! 0.400000 MCO_DodgeClose
//! 0.900000 MCO_Recovery
//! ```
//!
//! Each annotation entry contains a timestamp (`time`) and a text pointer (`text`),
//! representing a single annotation event extracted from the original animation data.
//!
//! This format is intended to be both human-readable and easy to serialize back into
//! the binary Havok format for modding or analysis purposes.

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
        use havok_types::I32;

        // Find the spline(s)
        let mut splines: Vec<_> = class_map
            .par_iter_mut()
            .filter(|(_, class)| matches!(class, Classes::hkaSplineCompressedAnimation(_)))
            .collect();
        let (_, spline) = {
            match splines.len() {
                0 => return MissingSplineSnafu.fail(),
                1 => splines.swap_remove(0),
                _ => {
                    return Err(HkannoError::MultipleSplinesFound {
                        count: splines.len(),
                    })
                }
            }
        };
        let Classes::hkaSplineCompressedAnimation(anim) = spline else {
            return MissingSplineSnafu.fail();
        };

        anim.m_numFrames = I32::Number(self.num_original_frames);
        anim.parent.m_duration = self.duration;

        for (track, edited_track) in anim
            .parent
            .m_annotationTracks
            .iter_mut()
            .zip(self.annotation_tracks)
        {
            for (ann, edited_ann) in track.m_annotations.iter_mut().zip(edited_track.annotations) {
                ann.m_time = edited_ann.time;
                ann.m_text = havok_types::StringPtr::from_option(edited_ann.text);
            }
        }

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
        // FIXME: xml preserve ordering.
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
        writeln!(f, "# numOriginalFrames: {}", self.num_original_frames)?;
        writeln!(f, "# duration: {}", self.duration)?;
        writeln!(f, "# numAnnotationTracks: {}", self.annotation_tracks.len())?;

        for track in &self.annotation_tracks {
            if track.annotations.is_empty() {
                continue;
            }

            writeln!(f, "# numAnnotations: {}", track.annotations.len())?;
            for ann in &track.annotations {
                let text = ann.text.as_deref().unwrap_or(havok_types::NULL_STR);
                writeln!(f, "{:.6} {}", ann.time, text)?;
            }
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
    use havok_classes::Classes;
    use havok_types::I32;

    // Find the spline(s)
    let (ptr, spline) = {
        let mut splines: Vec<_> = class_map
            .into_par_iter()
            .filter(|(_, class)| matches!(class, Classes::hkaSplineCompressedAnimation(_)))
            .collect();

        match splines.len() {
            0 => return MissingSplineSnafu.fail(),
            1 => splines.swap_remove(0),
            _ => {
                return Err(HkannoError::MultipleSplinesFound {
                    count: splines.len(),
                })
            }
        }
    };

    let Classes::hkaSplineCompressedAnimation(anim) = spline else {
        return Err(HkannoError::MissingSpline);
    };

    let tracks = anim
        .parent
        .m_annotationTracks
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
            AnnotationTrack { annotations }
        })
        .collect::<Vec<_>>();

    let num_original_frames = match &anim.m_numFrames {
        I32::Number(n) => *n,
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
    };

    Ok(Hkanno {
        ptr,
        num_original_frames,
        duration: anim.parent.m_duration,
        annotation_tracks: tracks,
    })
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

    /// Raised when the expected animation class was not found.
    #[snafu(display("No `hkaSplineCompressedAnimation` class found"))]
    MissingSpline,

    /// Multiple hkaSplineCompressedAnimation classes found.
    #[snafu(display("expected one `hkaSplineCompressedAnimation` per `hkx`, but multiple were obtained. Got count: {count}"))]
    MultipleSplinesFound { count: usize },

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
    use std::borrow::Cow;

    #[test]
    fn test_hkanno_to_string_format() {
        // Dummy Hkanno with 2 tracks and optional text
        let hkanno = Hkanno {
            ptr: Cow::Borrowed("#0003"),
            num_original_frames: 10,
            duration: 0.8,
            annotation_tracks: vec![
                AnnotationTrack {
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
                    annotations: vec![
                        Annotation {
                            time: 0.3,
                            text: Some(Cow::Borrowed("Alt1")),
                        },
                        Annotation {
                            time: 0.7,
                            text: None,
                        }, // missing text
                    ],
                },
            ],
        };

        assert_eq!(&hkanno.ptr, "#0003");

        let output = hkanno.to_string();

        // Basic structure checks
        assert!(output.contains("# numOriginalFrames: 10"));
        assert!(output.contains("# duration: 0.8"));
        assert!(output.contains("# numAnnotationTracks: 2"));
        assert!(output.contains("# numAnnotations: 2"));
        assert!(output.contains("0.100000 Start"));
        assert!(output.contains("0.700000 \u{2400}")); // None text replaced by NULL_STR
    }

    #[test]
    #[ignore = "Need Local file."]
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

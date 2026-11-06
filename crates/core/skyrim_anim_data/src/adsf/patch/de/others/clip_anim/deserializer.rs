use json_patch::JsonPath;
use nemesis_patch_syntax::cursor::Cursor;
use winnow::{
    Parser,
    ascii::multispace0,
    error::{StrContext::Expected, StrContextValue::Description},
};

use super::{ClipAnimDiffPatch, raw_diff::RawDiff};
use crate::{
    adsf::patch::de::{
        error::{Error, Result},
        others::raw_block::{Op, RawBlock, next_block},
    },
    common_parser::lines::{one_line, parse_one_line},
};

/// Parse `animationdatasinglefile.txt` clip animation block patch.
///
/// # Errors
/// Parse failed.
pub fn parse_clip_anim_diff_patch(adsf_patch: &str, priority: usize) -> Result<ClipAnimDiffPatch<'_>> {
    let mut patcher_de = PatchDeserializer::new(adsf_patch);
    patcher_de.root().map_err(|err| patcher_de.cursor.to_readable(err))?;

    super::raw_diff::into_patch_map(patcher_de.raw_diffs, priority)
}

/// Nemesis patch deserializer
#[derive(Debug, Clone)]
struct PatchDeserializer<'a> {
    /// Where the parsing is.
    cursor: Cursor<'a, Error>,

    /// Raw diff blocks captured during parsing.
    raw_diffs: Vec<RawDiff<'a>>,

    /// When an `ORIGINAL` comment arrives, we need to parse it for the number of
    /// len elements, but we don't treat it as a diff until `CLOSE`.
    /// This is the flag for that purpose.
    ignore_close: bool,

    /// Indicates the current json position during one patch file.
    ///
    /// e.g. `["trigger_names_len"]`
    path: JsonPath<'a>,

    /// Array end push Operation?
    op: Op,

    /// The index of the `trigger_names` array element being processed.
    seq_index: Option<usize>,
}

impl<'de> PatchDeserializer<'de> {
    const fn new(input: &'de str) -> Self {
        Self {
            cursor: Cursor::new(input),
            raw_diffs: Vec::new(),
            path: JsonPath::new(),
            op: Op::Add,
            ignore_close: false,
            seq_index: None,
        }
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // Parser methods

    /// Capture a raw diff block if present at the current position.
    ///
    /// The diff block is associated with the current `JsonPath`.
    ///
    /// # Return
    /// Got diff?
    fn maybe_capture_diff(&mut self) -> Result<bool> {
        let Some(RawBlock { text, op, original_len }) = next_block(&mut self.cursor, &mut self.ignore_close, self.op)?
        else {
            return Ok(false);
        };
        let path = self.path.clone();
        self.raw_diffs.push(RawDiff { path, text, op, seq_index: self.seq_index, original_len });
        Ok(true)
    }

    /// Any length info from 1 line.
    fn parse_len_line(&mut self) -> Result<usize> {
        self.maybe_capture_diff()?;
        let len =
            self.cursor.parse_next(parse_one_line::<usize>.context(Expected(Description("length_line: usize"))))?;
        #[cfg(feature = "tracing")]
        tracing::trace!("{:?}, line Length = {len}", self.path);
        Ok(len)
    }

    /// Parse 1 line (but ignore new line)
    fn parse_str_line(&mut self, expected: &'static str) -> Result<()> {
        self.maybe_capture_diff()?;
        let _s = self.cursor.parse_next(one_line.context(Expected(Description(expected))))?;
        #[cfg(feature = "tracing")]
        tracing::debug!(?self.path, ?_s);

        Ok(())
    }

    fn parse_line_f32(&mut self) -> Result<()> {
        self.maybe_capture_diff()?;
        let _value = self.cursor.parse_next(parse_one_line::<f32>.context(Expected(Description("f32"))))?;
        #[cfg(feature = "tracing")]
        tracing::debug!(?self.path, ?_value);

        Ok(())
    }

    /// Parse a single vanilla `trigger_name` entry.
    fn parse_trigger_name_line(&mut self) -> Result<()> {
        let _name = self.cursor.parse_next(one_line.context(Expected(Description("trigger_name: str"))))?;
        #[cfg(feature = "tracing")]
        tracing::debug!(?_name);

        Ok(())
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Parse 1 adsf clip anim patch
    fn root(&mut self) -> Result<()> {
        self.cursor.parse_next(multispace0)?;

        self.parse_name()?;
        self.parse_clip_id()?;
        self.parse_play_back_speed()?;
        self.parse_crop_start_local_time()?;
        self.parse_crop_end_local_time()?;
        self.parse_trigger_names()?;

        #[cfg(feature = "tracing")]
        tracing::debug!("{:#?}", self.raw_diffs);

        self.cursor.parse_next(multispace0)?;
        if !self.cursor.input.is_empty() {
            return Err(Error::IncompleteParse);
        }

        Ok(())
    }

    fn parse_name(&mut self) -> Result<()> {
        self.path.push("name".into());
        self.parse_str_line("name")?;
        self.path.pop();

        Ok(())
    }

    fn parse_clip_id(&mut self) -> Result<()> {
        self.path.push("clip_id".into());
        self.parse_str_line("clip_id")?;
        self.path.pop();

        Ok(())
    }

    fn parse_play_back_speed(&mut self) -> Result<()> {
        self.path.push("play_back_speed".into());
        self.parse_line_f32()?;
        self.path.pop();

        Ok(())
    }

    fn parse_crop_start_local_time(&mut self) -> Result<()> {
        self.path.push("crop_start_local_time".into());
        self.parse_line_f32()?;
        self.path.pop();

        Ok(())
    }

    fn parse_crop_end_local_time(&mut self) -> Result<()> {
        self.path.push("crop_end_local_time".into());
        self.parse_line_f32()?;
        self.path.pop();

        Ok(())
    }

    fn parse_trigger_names(&mut self) -> Result<()> {
        self.path.push("trigger_names_len".into());
        let len = self.parse_len_line()?;
        self.path.pop();

        let mut index = 0;
        self.seq_index = Some(index);

        while index < len {
            if self.maybe_capture_diff()? {
                continue;
            }
            if let Err(Error::Context { .. }) = self.parse_trigger_name_line() {
                return Err(self.cursor.message_to_readable(format!(
                    "Invalid TriggerNames.\n\nExpected {len} vanilla entries, but found only {index}."
                )));
            }

            index += 1;
            self.seq_index = Some(index);
        }

        // capture array push
        self.op = Op::SeqPush;
        self.seq_index = None;
        self.maybe_capture_diff()?;
        self.op = Op::Add;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use json_patch::{Action, JsonPatch, Op, ValueWithPriority};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::asdsf::patch::de::NonNestedArrayDiff;

    #[test]
    fn test_replace_anim_block_diff_patch() {
        let input = "
<!-- MOD_CODE ~test~ OPEN -->
TurnRight[test]
<!-- ORIGINAL -->
TurnRight[mirrored]
<!-- CLOSE -->
18
1
0
0
2
clipStart:6.65767
<!-- MOD_CODE ~test~ OPEN -->
clipEnd:8.1234
<!-- ORIGINAL -->
clipEnd:6.65767
<!-- CLOSE -->
";

        let patches = parse_clip_anim_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));
        let expected = ClipAnimDiffPatch {
            name: Some("TurnRight[test]".into()),
            trigger_names_patches: NonNestedArrayDiff {
                one: Default::default(),
                seq: vec![ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Seq { op: Op::Replace, range: 1..2 },
                        value: simd_json::json_typed!(borrowed, ["clipEnd:8.1234"]),
                    },
                    priority: 0,
                }],
            },
            ..Default::default()
        };

        assert_eq!(patches, expected);
    }

    #[test]
    fn test_add_anim_block_diff_patch() {
        let input = "
<!-- MOD_CODE ~test~ OPEN -->
TurnRight[test]
<!-- ORIGINAL -->
TurnRight[mirrored]
<!-- CLOSE -->
18
1
0
0
0
<!-- MOD_CODE ~test~ OPEN -->
clipStart:6.65767
clipEnd:6.65767
<!-- CLOSE -->
";

        let patches = parse_clip_anim_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));
        let expected = ClipAnimDiffPatch {
            name: Some("TurnRight[test]".into()),
            trigger_names_patches: NonNestedArrayDiff {
                one: Default::default(),
                seq: vec![ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::SeqPush,
                        value: simd_json::json_typed!(borrowed, ["clipStart:6.65767", "clipEnd:6.65767",]),
                    },
                    priority: 0,
                }],
            },
            ..Default::default()
        };

        assert_eq!(patches, expected);
    }
}

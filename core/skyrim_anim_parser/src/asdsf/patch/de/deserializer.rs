use super::error::{Error, Result};
use super::line_parsers::{take_raw_diff, version_v3};
use crate::asdsf::patch::de::raw_diff::{Op, RawDiff};
use crate::asdsf::patch::de::DiffPatchAnimSetData;
use crate::common_parser::comment::close_comment_line;
use crate::common_parser::lines::{num_bool_line, one_line, parse_one_line};

use json_patch::{JsonPath, ValueWithPriority};
use serde_hkx::errors::readable::ReadableError;
use std::collections::HashMap;
use winnow::combinator::opt;
use winnow::{
    ascii::multispace0,
    error::{ContextError, ErrMode},
    Parser,
};

pub type PatchesMap<'a> = HashMap<JsonPath<'a>, ValueWithPriority<'a>>;

/// Parse `animationsetdatasinglefile.txt` patch.
///
/// # Errors
/// Parse failed.
pub fn parse_anim_set_diff_patch(
    asdsf_patch: &str,
    priority: usize,
) -> Result<DiffPatchAnimSetData<'_>> {
    let mut patcher_de = PatchDeserializer::new(asdsf_patch);
    patcher_de
        .root_class()
        .map_err(|err| patcher_de.to_readable_err(err))?;

    super::raw_diff::into_patch_map(patcher_de.raw_diffs, priority)
}

/// Nemesis patch deserializer
#[derive(Debug, Clone)]
struct PatchDeserializer<'a> {
    /// mutable pointer to str
    input: &'a str,
    /// This is readonly for error report. Not move position.
    original: &'a str,

    /// Raw diff blocks captured during parsing.
    raw_diffs: Vec<RawDiff<'a>>,

    ///When an `ORIGINAL` comment arrives,
    /// we need to parse it for the number of len elements, but we don't treat it as a diff until CLOSE.
    /// This is the flag for that purpose.
    ignore_close: bool,

    /// Indicates the current json position during one patch file.
    ///
    /// e.g. `["attack", "[9]", "clip_names", "clip_name"]`
    path: JsonPath<'a>,

    /// Array end push Operation?
    op: Op,

    /// Parsed category.
    ///
    /// But that doesn't necessarily mean it's correct.
    /// For example, an addition at the end of a category might actually be a diff for the next category.
    category: ArrayType,

    /// The index of the array being processed.
    /// Since patching the entire array does not output the index to path, we store it here.
    seq_index: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ArrayType {
    /// `Array<Trigger(Str)>`
    Trigger,

    /// `Array<Condition>`
    Condition,

    /// `Array<Attack>`
    Attack,

    /// - `Array<Str>` for `Attack.clip_names`
    ClipName,

    /// `Array<AnimInfo>`
    AnimInfo,
}

impl<'de> PatchDeserializer<'de> {
    const fn new(input: &'de str) -> Self {
        Self {
            input,
            original: input,
            raw_diffs: Vec::new(),
            path: JsonPath::new(),
            op: Op::Add,
            ignore_close: false,
            category: ArrayType::Trigger,
            seq_index: None,
        }
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // Parser methods

    fn parse_next<O>(
        &mut self,
        mut parser: impl Parser<&'de str, O, ErrMode<ContextError>>,
    ) -> Result<O> {
        parser
            .parse_next(&mut self.input)
            .map_err(|err| Error::ContextError { err })
    }

    /// Convert Visitor errors to position-assigned errors.
    ///
    /// # Why is this necessary?
    /// Because Visitor errors that occur within each `Deserialize` implementation cannot indicate the error location in XML.
    #[cold]
    fn to_readable_err(&self, err: Error) -> Error {
        let readable = match err {
            Error::ContextError { err } => ReadableError::from_context(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
            Error::ReadableError { source } => source,
            err => ReadableError::from_display(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
        };

        Error::ReadableError { source: readable }
    }

    /// Capture a raw diff block if present at the current position.
    ///
    /// The diff block is associated with the current JsonPath.
    fn maybe_capture_diff(&mut self) -> Result<()> {
        if self.ignore_close && self.parse_next(opt(close_comment_line))?.is_some() {
            self.ignore_close = false;
        };

        if let Some((raw, has_original)) = self.parse_next(take_raw_diff)? {
            let path = self.path.clone();

            let op = match (has_original, raw.is_empty()) {
                (true, true) => Op::Remove,
                (true, false) => Op::Replace,
                (false, true) => Op::Add,
                (false, false) => self.op,
            };

            if has_original {
                self.ignore_close = true;
            }

            self.raw_diffs.push(RawDiff {
                path,
                text: raw,
                op,
                seq_index: self.seq_index,
                category: self.category,
            });
        }
        Ok(())
    }

    fn parse_array(&mut self, inner_type: ArrayType) -> Result<()> {
        self.category = inner_type;

        let len = self.parse_len_line()?;
        for index in 0..len {
            self.seq_index = Some(index);

            // The array of attacks is nested, so index specification is required.
            if matches!(inner_type, ArrayType::Attack) {
                self.path.push(format!("[{index}]").into());
            }

            // seq inner
            match inner_type {
                ArrayType::Condition => self.parse_condition()?,
                ArrayType::Attack => self.parse_attack()?,
                ArrayType::AnimInfo => self.parse_anim_info()?,
                ArrayType::Trigger | ArrayType::ClipName => self.parse_str_line()?,
            };

            if matches!(inner_type, ArrayType::Attack) {
                self.path.pop();
            }
        }

        // capture array push
        self.op = Op::SeqPush;
        self.seq_index = None;
        self.maybe_capture_diff()?;
        self.op = Op::Add;

        Ok(())
    }

    /// Any length info from 1 line.
    fn parse_len_line(&mut self) -> Result<usize> {
        use winnow::error::{StrContext::Expected, StrContextValue::Description};

        self.maybe_capture_diff()?;
        let len = self.parse_next(
            parse_one_line::<usize>.context(Expected(Description("length_line: usize"))),
        )?;
        #[cfg(feature = "tracing")]
        tracing::trace!("{:?}, line Length = {len}", self.path);
        Ok(len)
    }

    fn parse_num_bool(&mut self) -> Result<()> {
        self.maybe_capture_diff()?;
        self.parse_next(num_bool_line)?;
        Ok(())
    }

    /// Parse 1 line(but ignore new line)
    fn parse_str_line(&mut self) -> Result<()> {
        self.maybe_capture_diff()?;
        let _s = self.parse_next(one_line)?;
        #[cfg(feature = "tracing")]
        tracing::debug!(?self.path, ?_s);

        Ok(())
    }

    fn parse_line_to<T>(&mut self) -> Result<()>
    where
        T: core::str::FromStr + core::fmt::Debug + Copy,
    {
        self.maybe_capture_diff()?;
        let _value = self.parse_next(parse_one_line::<T>)?;
        #[cfg(feature = "tracing")]
        tracing::debug!(?self.path, ?_value);

        Ok(())
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Parse 1 asdsf patch
    fn root_class(&mut self) -> Result<()> {
        self.parse_next(multispace0)?;
        self.parse_next(version_v3)?;

        self.parse_array(ArrayType::Trigger)?; // triggers
        self.parse_array(ArrayType::Condition)?;
        self.parse_array(ArrayType::Attack)?;
        self.parse_array(ArrayType::AnimInfo)?;

        #[cfg(feature = "tracing")]
        tracing::debug!("{:#?}", self.raw_diffs);
        Ok(())
    }

    fn parse_condition(&mut self) -> Result<()> {
        self.path.push("name".into());
        self.parse_str_line()?;
        self.path.pop();

        self.path.push("value_a".into());
        self.parse_line_to::<i32>()?;
        self.path.pop();

        self.path.push("value_b".into());
        self.parse_line_to::<i32>()?;
        self.path.pop();

        Ok(())
    }

    fn parse_attack(&mut self) -> Result<()> {
        self.path.push("attack_trigger".into());
        self.parse_str_line()?;
        self.path.pop();

        self.path.push("is_contextual".into());
        self.parse_num_bool()?;
        self.path.pop();

        self.path.push("clip_names".into());
        self.parse_array(ArrayType::ClipName)?;
        self.path.pop();

        Ok(())
    }

    fn parse_anim_info(&mut self) -> Result<()> {
        self.path.push("hashed_path".into());
        self.parse_line_to::<u32>()?;
        self.path.pop();

        self.path.push("hashed_file_name".into());
        self.parse_line_to::<u32>()?;
        self.path.pop();

        self.path.push("ascii_extension".into());
        self.parse_line_to::<u32>()?;
        self.path.pop();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asdsf::patch::de::{
        patch_map::SeqPatchMap, AttacksDiff, DiffPatchAnimSetData, NonNestedArrayDiff,
    };
    use json_patch::{Action, JsonPatch, Op, ValueWithPriority};

    // V3                             <- version
    // 0                              <- triggers_len
    // 0                              <- conditions_len
    // 4                              <- attacks_len
    //
    // attackStart_Attack1            <- attack_trigger[0]
    // 0                              <- is_contextual
    // 1                              <- clip_names_len
    // Attack1                        <- clip_names[0]
    //
    // attackStart_Attack2            <- attack_trigger[1]
    // 0                              <- is_contextual
    // 1                              <- clip_names_len
    // Attack1_Mirrored               <- clip_names[0]
    //
    // attackStart_MC_1HMLeft         <- attack_trigger[2]
    // 0                              <- is_contextual
    // 1                              <- clip_names_len
    // MC_1HM_AttackLeft02            <- clip_names[0]
    //
    // attackStart_MC_1HMRight        <- attack_trigger[3]
    // 0                              <- is_contextual
    // 1                              <- clip_names_len
    // MC 1HM AttackRight01           <- clip_names[0]
    //
    // 2                              <- anim_infos_len
    //
    // 3064642194                     <- hashed_path[0]
    // 1047251415                     <- hashed_file_name[0]
    // 7891816                        <- ascii_extension[0]
    //
    // 3064642194                     <- hashed_path[1]
    // 19150068                       <- hashed_file_name[1]
    // 7891816                        <- ascii_extension[1]

    #[cfg_attr(feature = "tracing", quick_tracing::init)]
    #[test]
    fn test_replace_anim_block_diff_patch() {
        let input = "
V3
0
0
4
attackStart_Attack1
0
1
<!-- MOD_CODE ~test~ OPEN -->
AttackTestReplacedClipName
<!-- ORIGINAL -->
Attack1
<!-- CLOSE -->
attackStart_Attack2
0
1
Attack1_Mirrored
attackStart_MC_1HMLeft
0
1
MC_1HM_AttackLeft02
attackStart_MC_1HMRight
0
1
MC 1HM AttackRight01
2
<!-- MOD_CODE ~test~ OPEN -->
4000000000
2000000000
7891816
<!-- ORIGINAL -->
3995179646
1440038008
7891816
<!-- CLOSE -->
3064642194
19150068
7891816
<!-- MOD_CODE ~test~ OPEN -->
4000000003
2000000003
7891816

$crc32[meshes\\actors\\dragon\\animations]$
$crc32[ground_bite]$
7891816

4000000005
2000000005
7891816
<!-- CLOSE -->
";
        let patches = parse_anim_set_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));

        let expected = DiffPatchAnimSetData {
            version: None,
            triggers_patches: vec![],
            conditions_patches: NonNestedArrayDiff::default(),
            attacks_patches: AttacksDiff {
                one: Default::default(),
                seq: {
                    let mut map = SeqPatchMap::new();
                    map.insert(
                        json_patch::json_path!["[0]", "clip_names"],
                        ValueWithPriority {
                            patch: JsonPatch {
                                action: Action::Seq {
                                    op: Op::Replace,
                                    range: 0..1,
                                },
                                value: simd_json::json_typed!(
                                    borrowed,
                                    ["AttackTestReplacedClipName"]
                                ),
                            },
                            priority: 0,
                        },
                    );
                    map
                },
            },
            anim_infos_patches: NonNestedArrayDiff {
                one: Default::default(),
                seq: vec![
                    ValueWithPriority {
                        patch: JsonPatch {
                            action: Action::Seq {
                                op: Op::Replace,
                                range: 0..1,
                            },
                            value: simd_json::json_typed!(borrowed, [
                                {
                                    "hashed_path": "4000000000",
                                    "hashed_file_name": "2000000000",
                                    "ascii_extension": "7891816"
                                },
                            ]),
                        },
                        priority: 0,
                    },
                    ValueWithPriority {
                        patch: JsonPatch {
                            action: Action::SeqPush,
                            value: simd_json::json_typed!(borrowed, [
                                {
                                    "hashed_path": "4000000003",
                                    "hashed_file_name": "2000000003",
                                    "ascii_extension": "7891816"
                                },
                                {
                                    "hashed_path": "3692944883", // crc32 macro replaced
                                    "hashed_file_name": "3191128947", // crc32 macro replaced
                                    "ascii_extension": "7891816"
                                },
                                {
                                    "hashed_path": "4000000005",
                                    "hashed_file_name": "2000000005",
                                    "ascii_extension": "7891816"
                                },
                            ]),
                        },
                        priority: 0,
                    },
                ],
            },
        };
        pretty_assertions::assert_eq!(patches, expected);
    }

    #[cfg_attr(feature = "tracing", quick_tracing::init)]
    #[test]
    #[ignore = "because we need external test files"]
    fn parse() {
        let nemesis_xml = {
            let file_name = "1HMShield.txt";
            let path = std::path::Path::new("../../dummy/debug/asdsf").join(file_name);
            std::fs::read_to_string(path).unwrap()
        };
        dbg!(parse_anim_set_diff_patch(&nemesis_xml, 0).unwrap_or_else(|e| panic!("{e}")));
    }
}

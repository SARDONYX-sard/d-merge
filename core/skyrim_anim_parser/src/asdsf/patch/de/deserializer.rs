use crate::asdsf::normal::AnimInfo;
use crate::asdsf::patch::de::error::{Error, Result};
use crate::asdsf::patch::de::{
    current_state::{CurrentState, PartialTriggers},
    DiffPatchAnimSetData, ParserKind,
};
use crate::common_parser::comment::{close_comment, comment_kind, take_till_close, CommentKind};
use crate::common_parser::lines::{num_bool_line, one_line, parse_one_line, verify_line_parses_to};
use json_patch::{JsonPatch, Op, OpRange, OpRangeKind, ValueWithPriority};
use serde_hkx::errors::readable::ReadableError;
use winnow::combinator::eof;
use winnow::{
    ascii::multispace0,
    combinator::opt,
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    Parser,
};

/// Parse animationsetdatasinglefile.txt patch.
///
/// # Errors
/// Parse failed.
pub fn parse_anim_set_diff_patch(
    input: &str,
    priority: usize,
) -> Result<DiffPatchAnimSetData<'_>, Error> {
    let mut deserializer = Deserializer::new(input, priority);
    deserializer
        .root()
        .map_err(|err| deserializer.to_readable_err(err))?;
    Ok(deserializer.output_patches)
}

/// Nemesis patch deserializer
#[derive(Debug)]
struct Deserializer<'a> {
    /// mutable pointer to str
    input: &'a str,
    /// This is readonly for error report. Not move position.
    original: &'a str,

    /// Output
    output_patches: DiffPatchAnimSetData<'a>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    pub current: CurrentState<'a>,

    priority: usize,
}

impl<'de> Deserializer<'de> {
    fn new(input: &'de str, priority: usize) -> Self {
        Self {
            input,
            original: input,
            output_patches: DiffPatchAnimSetData::default(),
            current: CurrentState::new(),
            priority,
        }
    }

    fn parse_next<O>(
        &mut self,
        mut parser: impl Parser<&'de str, O, ErrMode<ContextError>>,
    ) -> Result<O> {
        parser
            .parse_next(&mut self.input)
            .map_err(|err| Error::Context { err })
    }

    /// Parse by argument parser no consume.
    ///
    /// If an error occurs, it is converted to [`ReadableError`] and returned.
    fn parse_peek<O>(
        &self,
        mut parser: impl Parser<&'de str, O, ErrMode<ContextError>>,
    ) -> Result<O> {
        let (_, res) = parser
            .parse_peek(self.input)
            .map_err(|err| Error::Context { err })?;
        Ok(res)
    }

    /// Parse by argument parser no consume.
    ///
    /// If an error occurs, it is converted to [`ReadableError`] and returned.
    fn parse_peek2<O>(
        &self,
        mut parser: impl Parser<&'de str, O, ErrMode<ContextError>>,
    ) -> Result<(&'de str, O)> {
        let (remain, ret) = parser
            .parse_peek(self.input)
            .map_err(|err| Error::Context { err })?;
        Ok((remain, ret))
    }

    /// Convert Visitor errors to position-assigned errors.
    ///
    /// # Why is this necessary?
    /// Because Visitor errors that occur within each `Deserialize` implementation cannot indicate the error location in XML.
    #[cold]
    fn to_readable_err(&self, err: Error) -> Error {
        let readable = match err {
            Error::Context { err } => ReadableError::from_context(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
            Error::Readable { source } => source,
            err => ReadableError::from_display(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
        };

        Error::Readable { source: readable }
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Parse 1 file patch
    fn root(&mut self) -> Result<()> {
        self.parse_next(multispace0)?;

        while let Some(line_kind) = self.current.next() {
            match line_kind {
                ParserKind::Version => {
                    let should_take = self.parse_opt_start_comment()?;

                    let version = self.parse_next(
                        one_line
                            .verify(|s: &str| s.eq_ignore_ascii_case("V3"))
                            .context(Expected(Description("version: \"V3\""))),
                    )?;

                    if should_take {
                        self.current.replace_one(version)?;
                        self.parse_opt_close_comment()?;
                    }
                }
                ParserKind::TriggersLen
                | ParserKind::ConditionsLen
                | ParserKind::AttacksLen
                | ParserKind::AnimInfosLen => {
                    let _len = self.parse_next(
                        verify_line_parses_to::<usize>
                            .context(Expected(Description("length_line: usize"))),
                    )?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("{line_kind:#?} = {_len:#?}");
                }
                ParserKind::Triggers => {
                    let mut start_index = 0;
                    while self.parse_peek(opt(parse_one_line::<usize>))?.is_none() {
                        let diff_start = self.parse_opt_start_comment()?;
                        if diff_start {
                            self.current.set_range_start(start_index)?;
                        }
                        let trigger = self
                            .parse_next(one_line.context(Expected(Description("trigger: Str"))))?;
                        if self.current.mode_code.is_some() {
                            self.current.push_as_trigger(trigger)?;
                        }

                        self.parse_opt_close_comment()?;
                        self.parse_next(multispace0)?;
                        start_index += 1;
                    }
                }
                ParserKind::Conditions => {
                    let mut start_index = 0;
                    while {
                        // NOTE: The condition is that value_b: i32 and attacks_len: usize are the boundaries, and both are numeric values.
                        // Read two lines ahead and peek len + attack_trigger: Str. If Str exists (and is not i32),
                        let (remain, _) = self.parse_peek2(opt(parse_one_line::<usize>))?;
                        let (_, maybe_value_b) = opt(parse_one_line::<usize>)
                            .parse_peek(remain)
                            .map_err(|err| Error::Context { err })?;
                        maybe_value_b.is_some()
                    } {
                        // TODO:
                        let diff_start = self.parse_opt_start_comment()?;
                        if diff_start {
                            self.current.set_range_start(start_index)?;
                        }
                        let trigger = self
                            .parse_next(one_line.context(Expected(Description("trigger: Str"))))?;
                        if self.current.mode_code.is_some() {
                            self.current.push_as_trigger(trigger)?;
                        }

                        self.parse_opt_close_comment()?;
                        self.parse_next(multispace0)?;
                        start_index += 1;
                    }
                }
                ParserKind::Attacks => {
                    let mut start_index = 0;
                    while self.parse_peek(opt(parse_one_line::<usize>))?.is_none() {
                        let diff_start = self.parse_opt_start_comment()?;
                        if diff_start {
                            self.current.set_range_start(start_index)?;
                        }
                        let attack_trigger = self.parse_next(
                            one_line
                                .verify(|s: &str| s.starts_with("attackStart"))
                                .context(Label("trigger: Str"))
                                .context(Expected(Description("start_with(\"attackStart\""))),
                        )?;
                        if self.current.mode_code.is_some() {
                            // self.current.push_as_trigger(attack_trigger)?;
                        }

                        self.parse_opt_close_comment()?;
                        self.parse_next(multispace0)?;

                        let _is_contextual = self.parse_next(
                            num_bool_line.context(Expected(Description("is_contextual: bool"))),
                        )?;
                        let _clip_names_len = self.parse_next(
                            parse_one_line::<usize>
                                .context(Expected(Description("clip_names_len: usize"))),
                        )?;
                        tracing::debug!(?attack_trigger, ?_is_contextual, ?_clip_names_len);

                        let mut clip_names_start_index = 0;
                        while {
                            let is_attack_trigger = self
                                .parse_peek(opt(
                                    one_line.verify(|s: &str| s.starts_with("attackStart"))
                                ))?
                                .is_some();
                            let is_anim_info_len =
                                self.parse_peek(opt(parse_one_line::<usize>))?.is_some();
                            !is_attack_trigger && !is_anim_info_len
                        } {
                            let diff_start = self.parse_opt_start_comment()?;
                            if diff_start {
                                self.current.set_range_start(clip_names_start_index)?;
                            }
                            let clip_name = self.parse_next(
                                one_line.context(Expected(Description("clip_name: Str"))),
                            )?;
                            if self.current.mode_code.is_some() {
                                let _ = clip_name; // TODO: push
                                                   // self.current.push_as_trigger(clip_name)?;
                            }

                            self.parse_opt_close_comment()?;
                            self.parse_next(multispace0)?;
                            clip_names_start_index += 1;
                            tracing::debug!(?clip_name);
                        }

                        start_index += 1;
                    }
                }
                ParserKind::AnimInfos => {
                    let mut start_index = 0;
                    while self.parse_peek(opt(eof))?.is_none() {
                        let diff_start = self.parse_opt_start_comment()?;
                        if diff_start {
                            self.current.set_range_start(start_index)?;
                        }
                        let anim_info = self.anim_info()?;
                        tracing::debug!(?anim_info);

                        self.parse_opt_close_comment()?;
                        self.parse_next(multispace0)?;
                        start_index += 1;
                    }
                    break;
                }
            };
        }

        self.parse_next(multispace0)?;
        if !self.input.is_empty() {
            return Err(Error::IncompleteParse);
        }

        Ok(())
    }

    fn anim_info(&mut self) -> Result<AnimInfo<'de>> {
        let should_take_in_this = self.parse_opt_start_comment()?;
        let hashed_path = self.parse_next(
            verify_line_parses_to::<u32>.context(Expected(Description("hashed_path: u32"))),
        )?;
        if should_take_in_this {
            self.current.replace_one(hashed_path.clone())?; // FIXME: correct clone?
        }
        self.parse_opt_close_comment()?;

        let should_take_in_this = self.parse_opt_start_comment()?;
        let hashed_file_name = self.parse_next(
            verify_line_parses_to::<u32>.context(Expected(Description("hashed_file_name: u32"))),
        )?;
        if should_take_in_this {
            self.current.replace_one(hashed_file_name.clone())?;
        }
        self.parse_opt_close_comment()?;

        let should_take_in_this = self.parse_opt_start_comment()?;
        let ascii_extension = self.parse_next(
            one_line
                .verify(|s: &str| s == "7891816")
                .context(Label("ascii_extension: u32"))
                .context(Expected(StringLiteral("7891816"))),
        )?;
        if should_take_in_this {
            self.current.replace_one(ascii_extension.clone())?;
        }
        self.parse_opt_close_comment()?;

        Ok(AnimInfo {
            hashed_path,
            hashed_file_name,
            ascii_extension,
        })
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// # Return
    /// Is the mode code comment?
    fn parse_opt_start_comment(&mut self) -> Result<bool> {
        if let Some(comment_ty) = self.parse_next(opt(comment_kind))? {
            #[cfg(feature = "tracing")]
            tracing::debug!(?comment_ty);
            match comment_ty {
                CommentKind::ModCode(id) => {
                    self.current.mode_code = Some(id);
                    // When there are no additional differences, it is 100% Remove.
                    let found_end_diff_sym = self.parse_opt_close_comment()?;
                    if found_end_diff_sym {
                        self.current.force_removed = true;
                    };
                    return Ok(true);
                }
                _ => return Ok(false),
            }
        }
        Ok(false)
    }

    /// Processes the close comment (`ORIGINAL` or `CLOSE`) depending on whether it was encountered,
    /// and returns whether it was encountered or not.
    fn parse_opt_close_comment(&mut self) -> Result<bool> {
        if let Some(comment_ty) = self.parse_next(opt(close_comment))? {
            #[cfg(feature = "tracing")]
            tracing::debug!(?comment_ty);
            match comment_ty {
                CommentKind::Original => {
                    self.current.set_is_passed_original();
                    let op = self.current.judge_operation();
                    if op != Op::Remove {
                        self.parse_next(take_till_close)?;
                        self.merge_to_output()?;
                    }
                    return Ok(true);
                }
                CommentKind::Close => {
                    self.merge_to_output()?;
                    return Ok(true);
                }
                _ => {}
            }
        }
        Ok(false)
    }

    /// This is the method that is called when a single differential change comment pair finishes calling.
    fn merge_to_output(&mut self) -> Result<(), Error> {
        let op = self.current.judge_operation();
        if let Some(mut partial_patch) = self.current.patch.take() {
            #[allow(clippy::match_same_arms)] // TODO: Remove this
            match self.current.current_kind()? {
                ParserKind::Version => {
                    if let Some(version) = partial_patch.version.take() {
                        self.output_patches.version.replace(version);
                    }
                }
                ParserKind::TriggersLen
                | ParserKind::ConditionsLen
                | ParserKind::AttacksLen
                | ParserKind::AnimInfosLen => {}
                ParserKind::Triggers => {
                    if let Some(triggers) = partial_patch.triggers.take() {
                        let PartialTriggers { range, values } = triggers;
                        let values = if op == Op::Remove { vec![] } else { values };
                        self.output_patches
                            .triggers_patches
                            .push(ValueWithPriority {
                                patch: JsonPatch {
                                    op: OpRangeKind::Seq(OpRange { op, range }),
                                    value: simd_json::json_typed!(borrowed, values),
                                },
                                priority: self.priority,
                            });
                    }
                }
                ParserKind::Conditions => {} // TODO
                ParserKind::Attacks => {}    // TODO:
                ParserKind::AnimInfos => {}  // TODO:
            }

            self.current.clear_flags(); // new patch is generated so clear flags.
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asdsf::patch::de::{AnimInfosDiff, ConditionsDiff, NestedPatches};
    use json_patch::{json_path, JsonPatch, OpRangeKind, ValueWithPriority};

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

    #[quick_tracing::init]
    #[ignore = "Not complete yet"]
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

4000000004
2000000004
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
            conditions_patches: ConditionsDiff::default(),
            attacks_patches: NestedPatches {
                base: vec![ValueWithPriority {
                    patch: JsonPatch {
                        op: OpRangeKind::Seq(OpRange {
                            op: Op::Replace,
                            range: 0..4,
                        }),
                        value: simd_json::json_typed!(borrowed, []),
                    },
                    priority: 0,
                }],
                children: vec![(
                    json_path!["attack_trigger"],
                    ValueWithPriority {
                        patch: JsonPatch {
                            op: OpRangeKind::Pure(Op::Replace),
                            value: simd_json::json_typed!(borrowed, "AttackTestReplacedClipName"),
                        },
                        priority: 0,
                    },
                )],
            },
            anim_infos_patches: AnimInfosDiff {
                one: Default::default(),
                seq: vec![
                    ValueWithPriority {
                        patch: JsonPatch {
                            op: OpRangeKind::Seq(OpRange {
                                op: Op::Replace,
                                range: 0..1,
                            }),
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
                            op: OpRangeKind::Seq(OpRange {
                                op: Op::Add,
                                range: 3..6,
                            }),
                            value: simd_json::json_typed!(borrowed, [
                                {
                                    "hashed_path": "4000000003",
                                    "hashed_file_name": "2000000003",
                                    "ascii_extension": "7891816"
                                },
                                {
                                    "hashed_path": "4000000004",
                                    "hashed_file_name": "2000000004",
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
        assert_eq!(patches, expected);
    }
}

use crate::asdsf::normal::{AnimInfo, Condition};
use crate::asdsf::patch::de::error::{Error, Result};
use crate::asdsf::patch::de::{
    current_state::{CurrentState, FieldKind, ParserKind},
    DiffPatchAnimSetData,
};
use crate::asdsf::patch::de::{AnimInfoDiff, ConditionDiff};
use crate::common_parser::comment::{close_comment, comment_kind, take_till_close, CommentKind};
use crate::common_parser::lines::{num_bool_line, one_line, parse_one_line, verify_line_parses_to};
use json_patch::{Action, JsonPatch, Op, ValueWithPriority};
use serde_hkx::errors::readable::ReadableError;
use winnow::{
    ascii::multispace0,
    combinator::{eof, opt},
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
                ParserKind::Version => self.version()?,
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
                ParserKind::Triggers => self.triggers()?,
                ParserKind::Conditions => self.conditions()?,
                ParserKind::Attacks => self.attacks()?,
                ParserKind::AnimInfos => {
                    self.anim_infos()?;
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

    fn version(&mut self) -> Result<()> {
        let should_take = self.parse_opt_start_comment()?;

        let version = self.parse_next(
            one_line
                .verify(|s: &str| s.eq_ignore_ascii_case("V3"))
                .context(Label("version"))
                .context(Expected(StringLiteral("V3"))),
        )?;

        if should_take {
            self.current.replace_one(FieldKind::Version(version))?;
            self.parse_opt_close_comment()?;
        }
        Ok(())
    }

    fn triggers(&mut self) -> Result<(), Error> {
        let mut start_index = 0;
        // until `condition_len`(usize)
        while self.parse_peek(opt(parse_one_line::<usize>))?.is_none() {
            let diff_start = self.parse_opt_start_comment()?;
            if diff_start {
                self.current.set_main_range_start(start_index)?;
            }
            let trigger =
                self.parse_next(one_line.context(Expected(Description("trigger: Str"))))?;
            if self.current.mode_code.is_some() {
                self.current.push_as_trigger(trigger)?;
            }

            self.parse_opt_close_comment()?;
            self.parse_next(multispace0)?;
            start_index += 1;
            self.current.increment_main_range();
        }
        Ok(())
    }

    fn conditions(&mut self) -> Result<(), Error> {
        let mut start_index = 0;
        while {
            // NOTE: In the case of patches, `attack_len: usize` cannot be trusted, so it is necessary to parse ahead.
            let (remain, variable_name) =
                self.parse_peek2(opt(one_line.verify(|s: &str| is_variable_name_starts(s))))?;
            let (remain, _value_a) = opt(parse_one_line::<usize>)
                .parse_peek(remain)
                .map_err(|err| Error::Context { err })?;
            let (_, value_b) = opt(parse_one_line::<usize>)
                .parse_peek(remain)
                .map_err(|err| Error::Context { err })?;
            variable_name.is_some() && _value_a.is_some() && value_b.is_some()
        } {
            let diff_start = self.parse_opt_start_comment()?;
            if diff_start {
                self.current.set_main_range_start(start_index)?;
            }

            let variable_name = {
                let should_take_in_this = self.parse_opt_start_comment()?;
                let variable_name = self.parse_next(
                    one_line
                        .verify(|s: &str| is_variable_name_starts(s))
                        .context(Label("variable_name: Str"))
                        .context(Expected(StringLiteral("iLeftHandType")))
                        .context(Expected(StringLiteral("iRightHandType")))
                        .context(Expected(StringLiteral("iWantMountedWeaponAnims")))
                        .context(Expected(StringLiteral("bWantMountedWeaponAnims"))),
                )?;

                if should_take_in_this {
                    self.current.one_field_patch =
                        Some(FieldKind::ConditionVariableName(variable_name.clone()));
                    self.parse_opt_close_comment()?;
                }
                variable_name
            };

            let value_a = {
                let should_take_in_this = self.parse_opt_start_comment()?;

                let value_a = self.parse_next(
                    parse_one_line::<i32>.context(Expected(Description("value_a: i32"))),
                )?;

                if should_take_in_this {
                    self.current.one_field_patch = Some(FieldKind::ConditionValueA(value_a));
                    self.parse_opt_close_comment()?;
                }
                value_a
            };

            let value_b = {
                let should_take_in_this = self.parse_opt_start_comment()?;

                let value_b = self.parse_next(
                    parse_one_line::<i32>.context(Expected(Description("value_b: i32"))),
                )?;

                if should_take_in_this {
                    self.current.one_field_patch = Some(FieldKind::ConditionValueA(value_b));
                    self.parse_opt_close_comment()?;
                }
                value_b
            };

            #[cfg(feature = "tracing")]
            tracing::trace!(?variable_name, ?value_a, ?value_b);

            if self.current.mode_code.is_some() {
                let condition = Condition {
                    variable_name,
                    value_a,
                    value_b,
                };

                self.current
                    .patch
                    .get_or_insert_default()
                    .conditions
                    .push(condition);
            }

            self.parse_opt_close_comment()?;
            self.parse_next(multispace0)?;
            start_index += 1;
            self.current.increment_main_range();
        }
        self.current.one_field_patch = None;

        Ok(())
    }

    fn attacks(&mut self) -> Result<(), Error> {
        let mut start_index = 0;
        while self.parse_peek(opt(parse_one_line::<usize>))?.is_none() {
            let diff_start = self.parse_opt_start_comment()?;
            if diff_start {
                self.current.set_main_range_start(start_index)?;
            }

            #[allow(unused)] // TODO: Support attack diff patch
            let attack_trigger = self.parse_next(
                one_line
                    .verify(|s: &str| is_attack_starts(s))
                    .context(Label("trigger: Str"))
                    .context(Expected(Description("start_with(`bashPowerStart`)")))
                    .context(Expected(Description("start_with(`attackStart`)")))
                    .context(Expected(Description("start_with(`attackPowerStart`)"))),
            )?;
            if self.current.mode_code.is_some() {
                // self.current.push_as_trigger(attack_trigger)?;
            }

            self.parse_opt_close_comment()?;
            self.parse_next(multispace0)?;

            let _is_contextual = self
                .parse_next(num_bool_line.context(Expected(Description("is_contextual: bool"))))?;
            let _clip_names_len = self.parse_next(
                parse_one_line::<usize>.context(Expected(Description("clip_names_len: usize"))),
            )?;
            #[cfg(feature = "tracing")]
            tracing::debug!(?attack_trigger, ?_is_contextual, ?_clip_names_len);

            let mut clip_names_start_index = 0;
            while {
                let is_attack_trigger = self
                    .parse_peek(opt(one_line.verify(|s: &str| is_attack_starts(s))))?
                    .is_some();

                let is_anim_info_len = self.parse_peek(opt(parse_one_line::<usize>))?.is_some();
                !is_attack_trigger && !is_anim_info_len
            } {
                let diff_start = self.parse_opt_start_comment()?;
                if diff_start {
                    self.current.set_main_range_start(clip_names_start_index)?;
                }
                let clip_name =
                    self.parse_next(one_line.context(Expected(Description("clip_name: Str"))))?;
                if self.current.mode_code.is_some() {
                    let _ = clip_name; // TODO: push
                                       // self.current.push_as_trigger(clip_name)?;
                }

                self.parse_opt_close_comment()?;
                self.parse_next(multispace0)?;
                clip_names_start_index += 1;

                #[cfg(feature = "tracing")]
                tracing::debug!(?clip_name);
            }

            start_index += 1;
        }
        Ok(())
    }

    fn anim_infos(&mut self) -> Result<(), Error> {
        let mut start_index = 0;
        while self.parse_peek(opt(eof))?.is_none() {
            let diff_start = self.parse_opt_start_comment()?;
            if diff_start {
                self.current.set_main_range_start(start_index)?;
            }
            let anim_info = self.anim_info()?;

            #[cfg(feature = "tracing")]
            tracing::debug!(?anim_info);

            if self.current.mode_code.is_some() {
                self.current
                    .patch
                    .get_or_insert_default()
                    .anim_infos
                    .push(anim_info);
            }

            self.parse_opt_close_comment()?;
            self.parse_next(multispace0)?;
            start_index += 1;
            self.current.increment_main_range();
        }
        Ok(())
    }

    fn anim_info(&mut self) -> Result<AnimInfo<'de>> {
        let hashed_path = {
            let should_take_in_this = self.parse_opt_start_comment()?;
            let hashed_path = self.parse_next(
                verify_line_parses_to::<u32>.context(Expected(Description("hashed_path: u32"))),
            )?;
            if should_take_in_this {
                // FIXME: correct clone?
                self.current
                    .replace_one(FieldKind::AnimInfoHashedPath(hashed_path.clone()))?;
                self.parse_opt_close_comment()?;
            }
            hashed_path
        };

        let hashed_file_name = {
            let should_take_in_this = self.parse_opt_start_comment()?;
            let hashed_file_name = self.parse_next(
                verify_line_parses_to::<u32>
                    .context(Expected(Description("hashed_file_name: u32"))),
            )?;
            if should_take_in_this {
                self.current
                    .replace_one(FieldKind::AnimInfoHashedFileName(hashed_file_name.clone()))?;
                self.parse_opt_close_comment()?;
            }
            hashed_file_name
        };

        let ascii_extension = {
            let should_take_in_this = self.parse_opt_start_comment()?;
            let ascii_extension = self.parse_next(
                one_line
                    .verify(|s: &str| s == "7891816")
                    .context(Label("ascii_extension: u32"))
                    .context(Expected(StringLiteral("7891816"))),
            )?;
            if should_take_in_this {
                self.current
                    .replace_one(FieldKind::AnimInfoAsciiExtension(ascii_extension.clone()))?;
                self.parse_opt_close_comment()?;
            }
            ascii_extension
        };

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
                    #[cfg(feature = "tracing")]
                    tracing::debug!(?op, ?self.current);
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
                    let triggers = core::mem::take(&mut partial_patch.triggers);
                    let values = if op == Op::Remove { vec![] } else { triggers };
                    let values = ValueWithPriority {
                        patch: JsonPatch {
                            action: Action::Seq {
                                op,
                                range: self.current.take_main_range()?,
                            },
                            value: values.into(),
                        },
                        priority: self.priority,
                    };
                    self.output_patches.triggers_patches.push(values);
                }
                ParserKind::Conditions => {
                    let one_diff = match self.current.one_field_patch.take() {
                        Some(FieldKind::ConditionVariableName(variable_name)) => {
                            Some(ConditionDiff {
                                variable_name: Some(variable_name),
                                ..Default::default()
                            })
                        }
                        Some(FieldKind::ConditionValueA(value_a)) => Some(ConditionDiff {
                            value_a: Some(value_a),
                            ..Default::default()
                        }),
                        Some(FieldKind::ConditionValueB(value_b)) => Some(ConditionDiff {
                            value_b: Some(value_b),
                            ..Default::default()
                        }),
                        None => None,
                        unexpected => {
                            return Err(Error::ExpectedOneFieldOfCondition {
                                other: format!("{unexpected:?}"),
                            })
                        }
                    };

                    let range = self.current.take_main_range()?;
                    if let Some(diff) = one_diff {
                        // one_patch
                        if op == Op::Replace {
                            self.output_patches
                                .conditions_patches
                                .one
                                .insert(range.start, diff);
                        } else {
                            return Err(Error::InvalidOpForOneField { op });
                        }
                    } else {
                        // seq pattern
                        let conditions = core::mem::take(&mut partial_patch.conditions);
                        let values = if op == Op::Remove { vec![] } else { conditions };
                        let values = ValueWithPriority {
                            patch: JsonPatch {
                                action: Action::Seq { op, range },
                                value: values.into(),
                            },
                            priority: self.priority,
                        };
                        self.output_patches.conditions_patches.seq.push(values);
                    }
                }
                ParserKind::Attacks => {} // TODO: Support attack diff
                ParserKind::AnimInfos => {
                    let one_diff = match self.current.one_field_patch.take() {
                        Some(FieldKind::AnimInfoAsciiExtension(ascii_extension)) => {
                            Some(AnimInfoDiff {
                                ascii_extension: Some(ascii_extension),
                                ..Default::default()
                            })
                        }
                        Some(FieldKind::AnimInfoHashedFileName(hashed_file_name)) => {
                            Some(AnimInfoDiff {
                                hashed_file_name: Some(hashed_file_name),
                                ..Default::default()
                            })
                        }
                        Some(FieldKind::AnimInfoHashedPath(hashed_path)) => Some(AnimInfoDiff {
                            hashed_path: Some(hashed_path),
                            ..Default::default()
                        }),
                        None => None,
                        unexpected => {
                            return Err(Error::ExpectedOneFieldOfAnimInfo {
                                other: format!("{unexpected:?}"),
                            })
                        }
                    };

                    let range = self.current.take_main_range()?;
                    if let Some(diff) = one_diff {
                        // one_patch
                        if op == Op::Replace {
                            self.output_patches
                                .anim_infos_patches
                                .one
                                .insert(range.start, diff);
                        } else {
                            return Err(Error::InvalidOpForOneField { op });
                        }
                    } else {
                        // seq pattern
                        let anim_infos = core::mem::take(&mut partial_patch.anim_infos);
                        let values = if op == Op::Remove { vec![] } else { anim_infos };
                        let values = ValueWithPriority {
                            patch: JsonPatch {
                                action: Action::Seq { op, range },
                                value: values.into(),
                            },
                            priority: self.priority,
                        };
                        self.output_patches.anim_infos_patches.seq.push(values);
                    }
                }
            }

            self.current.clear_flags(); // new patch is generated so clear flags.
        }

        Ok(())
    }
}

/// In the case of patches, attack_len cannot be trusted, so we have no choice but to parse ahead.
///
/// To do so, we need to limit the variables that can be used.
/// Fortunately, these are the only variables that appear in asdsf.
/// However, this also means that other variables cannot be changed in the patch.
fn is_variable_name_starts(s: &str) -> bool {
    s.starts_with("iLeftHandType")
        || s.starts_with("iRightHandType")
        || s.starts_with("iWantMountedWeaponAnims")
        || s.starts_with("bWantMountedWeaponAnims")
}

fn is_attack_starts(s: &str) -> bool {
    starts_with_ignore_ascii(s, "attackStart")
        || starts_with_ignore_ascii(s, "attackPowerStart")
        || starts_with_ignore_ascii(s, "bashStart")
        || starts_with_ignore_ascii(s, "bashPowerStart")
}

fn starts_with_ignore_ascii(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len()
        && s.get(..prefix.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asdsf::patch::de::{AnimInfosDiff, ConditionsDiff};
    use json_patch::{Action, JsonPatch, ValueWithPriority};

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

        // For local file test
        // let path = r#""#;
        // let input = std::fs::read_to_string(path).unwrap();
        // let patches = parse_anim_set_diff_patch(&input, 0).unwrap_or_else(|e| panic!("{e}"));

        let expected = DiffPatchAnimSetData {
            version: None,
            triggers_patches: vec![],
            conditions_patches: ConditionsDiff::default(),
            attacks_patches: (),
            anim_infos_patches: AnimInfosDiff {
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
                            action: Action::Seq {
                                op: Op::Add,
                                range: 2..5,
                            },
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

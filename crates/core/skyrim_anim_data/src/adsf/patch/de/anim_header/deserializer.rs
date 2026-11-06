use json_patch::Op;
use nemesis_patch_syntax::cursor::Cursor;
use winnow::{
    Parser,
    ascii::multispace0,
    combinator::opt,
    error::{StrContext::*, StrContextValue::*},
};

use super::current_state::CurrentState;
use crate::{
    adsf::patch::de::{
        anim_header::{AnimHeaderDiffPatch, DiffProjectAssets, LineKind, current_state::PartialProjectAssets},
        error::{Error, Result},
    },
    common_parser::{
        delete_line::delete_this_line,
        line_diff::{DiffFlags, LineDiffParser},
        lines::{num_bool_line, one_line, verify_line_parses_to},
    },
};

/// Parse animationdatasinglefile.txt clip motion block patch.
///
/// # Errors
/// Parse failed.
pub fn parse_anim_header_diff_patch(input: &str) -> Result<AnimHeaderDiffPatch<'_>, Error> {
    let mut deserializer = Deserializer::new(input);
    deserializer.root().map_err(|err| deserializer.cursor.to_readable(err))?;
    Ok(deserializer.output_patches)
}

/// Nemesis patch deserializer
#[derive(Debug)]
struct Deserializer<'a> {
    /// Where the parsing is.
    cursor: Cursor<'a, Error>,

    /// Output
    output_patches: AnimHeaderDiffPatch<'a>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    pub current: CurrentState<'a>,
}

impl<'de> Deserializer<'de> {
    fn new(input: &'de str) -> Self {
        Self { cursor: Cursor::new(input), output_patches: AnimHeaderDiffPatch::DEFAULT, current: CurrentState::new() }
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Parse 1 file patch
    fn root(&mut self) -> Result<()> {
        self.cursor.parse_next(multispace0)?;

        while let Some(line_kind) = self.current.next() {
            match line_kind {
                LineKind::LeadInt => {
                    let should_take = self.parse_opt_start_comment()?;
                    self.cursor.parse_next(multispace0)?;

                    let lead_int = self
                        .cursor
                        .parse_next(verify_line_parses_to::<i32>.context(Expected(Description("lead_int: i32"))))?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("lead_int = {lead_int:#?}");

                    if should_take {
                        self.current.replace_lead_int(lead_int)?;
                        self.parse_opt_close_comment()?;
                    }
                    self.cursor.parse_next(multispace0)?;
                }
                LineKind::ProjectAssetsLen => {
                    let diff_start = self.parse_opt_start_comment()?;
                    if diff_start {
                        self.current.set_range_start(0)?;
                    }
                    let _len = self
                        .cursor
                        .parse_next(verify_line_parses_to::<usize>.context(Expected(Description("length: usize"))))?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("{line_kind:#?} = {_len:#?}");
                }
                LineKind::ProjectAssets => {
                    let mut start_index = 0;
                    // until has_motion_data line
                    while self.cursor.parse_peek(opt(num_bool_line))?.is_none() {
                        let diff_start = self.parse_opt_start_comment()?;
                        if diff_start {
                            self.current.set_range_start(start_index)?;
                        }

                        if self.cursor.parse_next(opt(delete_this_line))?.is_some() {
                            start_index += 1;
                            self.current.increment_project_assets_range();
                        } else {
                            let project_asset = self.cursor.parse_next(one_line)?;
                            if self.current.flags.mode_code.is_some() {
                                self.current.push_as_project_assets(project_asset)?;
                            }
                        }

                        self.parse_opt_close_comment()?;
                        self.cursor.parse_next(multispace0)?;
                        start_index += 1;
                    }
                }
                LineKind::HasMotionData => {
                    let should_take = self.parse_opt_start_comment()?;
                    self.cursor.parse_next(multispace0)?;

                    let _has_motion_data = self
                        .cursor
                        .parse_next(num_bool_line.context(Expected(Description("has_motion_data: '0' | '1'"))))?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("has_motion_data = {_has_motion_data}");

                    if should_take {
                        self.parse_opt_close_comment()?;
                    }
                    self.cursor.parse_next(multispace0)?;
                }
            };
        }

        self.cursor.parse_next(multispace0)?;
        if !self.cursor.input.is_empty() {
            return Err(Error::IncompleteParse);
        }

        Ok(())
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
}

impl<'de> LineDiffParser<'de> for Deserializer<'de> {
    type Error = Error;

    fn cursor(&mut self) -> &mut Cursor<'de, Error> {
        &mut self.cursor
    }

    fn flags(&mut self) -> &mut DiffFlags<'de> {
        &mut self.current.flags
    }

    fn has_patch(&self) -> bool {
        self.current.patch.is_some()
    }

    /// This is the method that is called when a single differential change comment pair finishes calling.
    fn merge_to_output(&mut self) -> Result<(), Error> {
        let op = self.current.flags.judge_operation(self.current.patch.is_some());
        if let Some(mut partial_patch) = self.current.patch.take() {
            match self.current.current_kind()? {
                LineKind::LeadInt => {
                    if let Some(lead_int) = partial_patch.lead_int.take() {
                        self.output_patches.lead_int.replace(lead_int);
                    }
                }
                LineKind::ProjectAssetsLen | LineKind::HasMotionData => {}
                LineKind::ProjectAssets => {
                    if let Some(project_assets) = partial_patch.project_assets.take() {
                        let PartialProjectAssets { range, values } = project_assets;
                        let values = if op == Op::Remove { vec![] } else { values };
                        self.output_patches.project_assets = Some(DiffProjectAssets { op, range, values });
                    }
                }
            }

            self.current.flags.clear(); // new patch is generated so clear flags.
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[quick_tracing::init]
    #[test]
    fn test_patch_modes_extended() {
        // 432: 1                                    // <-- lead_int (maybe only 1)
        // 433: 3                                    // <-- project_assets_len: usize
        // 435: Behaviors\ChickenBehavior.hkx        // <-- project_asset[0]
        // 436: Characters\ChickenCharacter.hkx      // <-- project_asset[1]
        // 437: Character Assets\skeleton.HKX        // <-- project_asset[2]
        // 439: 1                                    // <-- has motion data (1/0): bool
        let input = r##"

        <!-- MOD_CODE ~test~ OPEN -->
93
<!-- ORIGINAL -->
1
<!-- CLOSE -->
3
Behaviors\ChickenBehavior.hkx
<!-- MOD_CODE ~test~ OPEN -->
Characters\ChickenTest0.hkx
Characters\ChickenTest1.hkx
<!-- ORIGINAL -->
Characters\ChickenCharacter.hkx
Character Assets\skeleton.HKX
<!-- CLOSE -->
1

"##;

        let patches = parse_anim_header_diff_patch(input).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            patches,
            AnimHeaderDiffPatch {
                lead_int: Some("93".into()),
                project_assets: Some(DiffProjectAssets {
                    op: Op::Replace,
                    range: 1..3,
                    values: vec!["Characters\\ChickenTest0.hkx".into(), "Characters\\ChickenTest1.hkx".into(),],
                })
            }
        );
    }
}

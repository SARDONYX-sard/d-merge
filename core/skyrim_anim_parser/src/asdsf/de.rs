//! Parses animation data from asdsf(animationsetdatasinglefile.txt)
use super::{AnimInfo, AnimSetData, Asdsf, Attack, Condition};
use crate::lines::{lines, num_bool_line, one_line, parse_one_line, Str};
use serde_hkx::errors::readable::ReadableError;
use winnow::{
    combinator::opt,
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    seq, ModalResult, Parser,
};

/// Parses the animation data structure from the input.
///
/// # Errors
/// If parsing fails, returns human readable error.
pub fn parse_asdsf(input: &str) -> Result<Asdsf<'_>, ReadableError> {
    asdsf.parse(input).map_err(|e| ReadableError::from_parse(e))
}

fn asdsf<'a>(input: &mut &'a str) -> ModalResult<Asdsf<'a>> {
    let txt_projects = txt_projects.parse_next(input)?;

    let mut anim_set_list = vec![];
    #[cfg(feature = "tracing")]
    let mut i = 0;
    while let Ok(anim_set_data) = anim_set_data
        .context(Label("AnimSetData"))
        .parse_next(input)
    {
        #[cfg(feature = "tracing")]
        {
            tracing::debug!(i);
            tracing::debug!(?anim_set_data);
            i += 1;
        }
        anim_set_list.push(anim_set_data);
    }

    Ok(Asdsf {
        txt_projects,
        anim_set_list,
    })
}

/// Parses the project names from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn txt_projects<'a>(input: &mut &'a str) -> ModalResult<Vec<Str<'a>>> {
    let line_len = parse_one_line
        .context(Expected(Description("project_names_len: usize")))
        .parse_next(input)?;

    let mut txt_projects = vec![];
    for _ in 0..line_len {
        let project_name = one_line
            .verify(|line: &str| line.ends_with(".txt"))
            .context(Expected(Description("project_name: *.txt")))
            .parse_next(input)?;
        txt_projects.push(project_name);
    }

    Ok(txt_projects)
}

fn anim_set_data<'a>(input: &mut &'a str) -> ModalResult<AnimSetData<'a>> {
    let file_names_len = opt(one_line
        .verify(|line: &str| line != "V3")
        .try_map(|s| s.as_ref().parse::<usize>())
        .context(Expected(Description("file_names_len: usize"))))
    .parse_next(input)?;

    let mut file_names = None;
    if let Some(file_names_len) = file_names_len {
        file_names = Some(
            lines(file_names_len)
                .context(Expected(Description("file_names: Vec<Str>")))
                .parse_next(input)?,
        );
    }

    let version = one_line
        .verify(|line: &str| line == "V3")
        .context(Expected(Description("version == V3")))
        .parse_next(input)?;

    let triggers_len = parse_one_line
        .context(Expected(Description("triggers_len: usize")))
        .parse_next(input)?;
    let triggers = lines(triggers_len)
        .context(Expected(Description("triggers: Vec<Str>")))
        .parse_next(input)?;

    let conditions_len = parse_one_line
        .context(Expected(Description("conditions_len: usize")))
        .parse_next(input)?;
    let conditions = conditions(conditions_len)
        .context(Expected(Description("conditions: Vec<Str>")))
        .parse_next(input)?;

    let attacks_len = parse_one_line
        .context(Expected(Description("attacks_len: usize")))
        .parse_next(input)?;
    let attacks = attacks(attacks_len)
        .context(Expected(Description("attacks: Vec<Str>")))
        .parse_next(input)?;

    let anim_infos_len = parse_one_line
        .context(Expected(Description("anim_infos_len: usize")))
        .parse_next(input)?;
    let anim_infos = anim_infos(anim_infos_len)
        .context(Expected(Description("anim_infos: Vec<Str>")))
        .parse_next(input)?;

    Ok(AnimSetData {
        file_names_len,
        file_names,
        version,
        triggers_len,
        triggers,
        conditions_len,
        conditions,
        attacks_len,
        attacks,
        anim_infos_len,
        anim_infos,
    })
}

fn conditions<'a>(
    line_len: usize,
) -> impl Parser<&'a str, Vec<Condition<'a>>, ErrMode<ContextError>> {
    move |input: &mut &'a str| {
        let mut conditions = vec![];
        for _ in 0..line_len {
            conditions.push(
                seq! {
                    Condition {
                        variable_name: one_line.context(Expected(Description("variable_name: str"))),
                        value_a: parse_one_line.context(Expected(Description("value_a: i32"))),
                        value_b: parse_one_line.context(Expected(Description("value_b: i32"))),
                    }
                }
                .context(Label("Condition"))
                .parse_next(input)?,
            );
        }

        Ok(conditions)
    }
}

fn attacks<'a>(line_len: usize) -> impl Parser<&'a str, Vec<Attack<'a>>, ErrMode<ContextError>> {
    move |input: &mut &'a str| {
        let mut attacks = vec![];
        for _ in 0..line_len {
            let attack = seq! {
                    Attack {
                        attack_trigger: one_line.context(Expected(Description("attack_trigger: str"))),
                        unknown: num_bool_line.context(Expected(Description("unknown: 0 | 1"))),
                        clip_names_len: parse_one_line.context(Expected(Description("clip_names_len: usize"))),
                        clip_names: lines(clip_names_len).context(Expected(Description("clip_names: Vec<str>"))),
                    }
                }
                .context(Label("Attack"))
                .parse_next(input)?;
            attacks.push(attack);
        }

        Ok(attacks)
    }
}

fn anim_infos<'a>(line_len: usize) -> impl Parser<&'a str, Vec<AnimInfo>, ErrMode<ContextError>> {
    move |input: &mut &'a str| {
        let mut anim_infos = vec![];
        for _ in 0..line_len {
            anim_infos.push(
                seq! {
                    AnimInfo {
                        hashed_path: parse_one_line.context(Expected(Description("hashed_path: u32"))),
                        hashed_file_name: parse_one_line.context(Expected(Description("hashed_file_name: u32"))),
                        ascii_extension: parse_one_line.context(Expected(Description("ascii_extension: u32"))),
                    }
                }
                .context(Label("AnimInfo"))
                .parse_next(input)?,
            );
        }

        Ok(anim_infos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use rayon::prelude::*;
    // use std::collections::HashMap;

    fn test_parse(input: &str) {
        match parse_asdsf(input) {
            Ok(asdsf) => {
                std::fs::create_dir_all("../../../../dummy/debug").unwrap();

                // Key value pair?
                // let asdsf: HashMap<_, _> = asdsf
                //     .txt_projects
                //     .par_iter()
                //     .zip(asdsf.anim_set_list)
                //     .collect();

                std::fs::write(
                    "../../../../dummy/debug/asdsf_debug.log",
                    format!("{asdsf:#?}"),
                )
                .unwrap();
            }
            Err(err) => panic!("{err}"),
        }
    }

    #[test]
    fn should_parse() {
        let s = include_str!(
            "../../../../resource/xml/templates/meshes/animationsetdatasinglefile.txt"
        );
        test_parse(s);
    }
}

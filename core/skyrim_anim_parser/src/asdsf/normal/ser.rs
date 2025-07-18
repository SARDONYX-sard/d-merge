use crate::{
    asdsf::normal::{AnimInfo, AnimSetData, AnimSetList, Asdsf, Attack, Condition, TxtProjects},
    common_parser::lines::Str,
};

const NEW_LINE: &str = "\r\n";

/// Converts an `Asdsf` struct back into the original text format with `\r\n` line endings.
pub fn serialize_asdsf(data: &Asdsf<'_>) -> String {
    let mut out = String::new();

    write_projects(&mut out, &data.txt_projects);

    for (_, anim_set_list) in &data.txt_projects.0 {
        write_file_names(&mut out, anim_set_list);
        for (_, anim_set) in &anim_set_list.0 {
            write_anim_set(&mut out, anim_set);
        }
    }

    out
}

fn write_projects(out: &mut String, projects: &TxtProjects) {
    out.push_str(&projects.0.len().to_string());
    out.push_str(NEW_LINE);
    for (project_name, _) in &projects.0 {
        out.push_str(project_name);
        out.push_str(NEW_LINE);
    }
}

pub(crate) fn write_file_names(out: &mut String, anim_set_list: &AnimSetList<'_>) {
    let file_names_len = anim_set_list.0.len();
    if file_names_len == 0 {
        return;
    }

    out.push_str(&file_names_len.to_string());
    out.push_str(NEW_LINE);

    for name in anim_set_list.0.keys() {
        out.push_str(name);
        out.push_str(NEW_LINE);
    }
}

pub(crate) fn write_anim_set(out: &mut String, anim_set: &AnimSetData<'_>) {
    write_version(out, &anim_set.version);
    write_triggers(out, &anim_set.triggers);
    write_conditions(out, &anim_set.conditions);
    write_attacks(out, &anim_set.attacks);
    write_anim_infos(out, &anim_set.anim_infos);
}

fn write_version(out: &mut String, version: &Str<'_>) {
    out.push_str(version);
    out.push_str(NEW_LINE);
}

fn write_triggers(out: &mut String, triggers: &[Str<'_>]) {
    out.push_str(&triggers.len().to_string());
    out.push_str(NEW_LINE);
    for trig in triggers {
        out.push_str(trig);
        out.push_str(NEW_LINE);
    }
}

fn write_conditions(out: &mut String, conditions: &[Condition<'_>]) {
    out.push_str(&conditions.len().to_string());
    out.push_str(NEW_LINE);
    for cond in conditions {
        out.push_str(&cond.variable_name);
        out.push_str(NEW_LINE);
        out.push_str(&cond.value_a.to_string());
        out.push_str(NEW_LINE);
        out.push_str(&cond.value_b.to_string());
        out.push_str(NEW_LINE);
    }
}

fn write_attacks(out: &mut String, attacks: &[Attack<'_>]) {
    out.push_str(&attacks.len().to_string());
    out.push_str(NEW_LINE);
    for atk in attacks {
        out.push_str(&atk.attack_trigger);
        out.push_str(NEW_LINE);
        out.push_str(if atk.is_contextual { "1" } else { "0" });
        out.push_str(NEW_LINE);
        out.push_str(&atk.clip_names_len.to_string());
        out.push_str(NEW_LINE);
        for clip in &atk.clip_names {
            out.push_str(clip);
            out.push_str(NEW_LINE);
        }
    }
}

fn write_anim_infos(out: &mut String, infos: &[AnimInfo]) {
    out.push_str(&infos.len().to_string());
    out.push_str(NEW_LINE);
    for info in infos {
        out.push_str(info.hashed_path.as_ref());
        out.push_str(NEW_LINE);
        out.push_str(info.hashed_file_name.as_ref());
        out.push_str(NEW_LINE);
        out.push_str(info.ascii_extension.as_ref());
        out.push_str(NEW_LINE);
    }
}

#[cfg(test)]
mod tests {
    use crate::asdsf::normal::de::parse_asdsf;

    use super::*;

    fn normalize_to_crlf(input: &str) -> std::borrow::Cow<'_, str> {
        if input.contains("\r\n") {
            input.into()
        } else {
            input.replace("\r", "").replace("\n", "\r\n").into()
        }
    }

    #[test]
    fn test_serialize_asdsf() {
        let expected = normalize_to_crlf(include_str!(
            "../../../../../resource/xml/templates/meshes/animationsetdatasinglefile.txt"
        ));
        let asdsf = parse_asdsf(&expected).unwrap_or_else(|e| panic!("{e}"));

        // std::fs::write("../../dummy/debug/adsf_debug.txt", format!("{:#?}", adsf)).unwrap();
        let actual = serialize_asdsf(&asdsf);

        // std::fs::create_dir_all("../../dummy").unwrap();
        // std::fs::write("../../dummy/adsf.txt", &actual).unwrap();

        let res = dbg!(actual == expected);
        if !res {
            let diff = ::diff::diff(&actual, &expected);
            std::fs::write("../../../dummy/diff.txt", diff).unwrap();
            panic!("actual != expected");
        }
        assert!(res);
    }
}

//! Animation data from alterative asdsf(animationsetdatasinglefile.txt)
//! This refers to data from fragments of paths specified in the Nemesis patch and modifies the data structure to apply the patch.
//!
//! This enables the patch to be applied quickly.
mod alt_key;
pub mod ser;

use indexmap::IndexMap;

use crate::{
    asdsf::{
        alt::alt_key::{to_alt_txt_project_name, to_normal_txt_project_name},
        normal::{AnimSetList, Asdsf, TxtProjects},
    },
    common_parser::lines::Str,
};

/// Represents the entire animation data structure.
///
/// Before merging the `animationsetdatasinglefile.txt` file, it exists in `meshes/animationsetdata` in Animation.bsa.
///
/// However, please note that there are no txt references such as Vampire in the `animationsetdatasinglefile.txt` file.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone)]
pub struct AltAsdsf<'a> {
    /// A list of project names parsed from the input.
    pub txt_projects: AltTxtProjects<'a>,
}

/// Alternative representation of `TxtProjects` for the Nemesis patch.
///
/// - key: project data file names. This is the same as the path fragment specified in the Nemesis patch, but without the txt extension and with `\` replaced by `~`.
///
///   e.g. `ChickenProjectData~ChickenProject`, `DefaultMale~DefaultMale`
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone)]
pub struct AltTxtProjects<'a>(IndexMap<Str<'a>, AnimSetList<'a>>);

impl<'a> TryFrom<Asdsf<'a>> for AltAsdsf<'a> {
    type Error = String;

    fn try_from(asdsf: Asdsf<'a>) -> Result<Self, Self::Error> {
        let mut txt_projects = IndexMap::new();
        for (key, anim_set_list) in asdsf.txt_projects.0 {
            let mut buf = String::new();
            to_alt_txt_project_name(key.as_ref(), &mut buf)
                .ok_or_else(|| format!("Invalid path: {key}"))?;
            let alt_key = Str::from(buf);
            txt_projects.insert(alt_key, anim_set_list);
        }
        Ok(AltAsdsf {
            txt_projects: AltTxtProjects(txt_projects),
        })
    }
}

impl<'a> TryFrom<AltAsdsf<'a>> for Asdsf<'a> {
    type Error = String;

    fn try_from(alt_asdsf: AltAsdsf<'a>) -> Result<Self, Self::Error> {
        let mut txt_projects = IndexMap::new();
        for (key, anim_set_list) in alt_asdsf.txt_projects.0 {
            let mut buf = String::new();
            to_normal_txt_project_name(key.as_ref(), &mut buf)
                .ok_or_else(|| format!("Invalid path: {key}"))?;
            let normal_key = Str::from(buf);
            txt_projects.insert(normal_key, anim_set_list);
        }
        Ok(Asdsf {
            txt_projects: TxtProjects(txt_projects),
        })
    }
}

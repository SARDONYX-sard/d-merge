//! For `character/behaviors/mt_behavior.xml` 1 file patch
mod alternative;
mod furniture_root;

pub(crate) use alternative::{
    FNIS_AA_MT_AUTO_GEN_5218, FNIS_AA_MT_AUTO_GEN_5219, FNIS_AA_MT_AUTO_GEN_5220,
    FNIS_AA_MT_AUTO_GEN_5221, FNIS_BA_BLEND_TRANSITION_5231, FNIS_BA_BLEND_TRANSITION_5232,
};
pub(crate) use furniture_root::{FNIS_FU_MT_5216, FNIS_GLOBAL_FU_MT_STATE_ID};

use crate::behaviors::tasks::fnis::patch_gen::JsonPatchPairs;

/// Generate the Havok class of `character/behaviors/mt_behavior.xml`.
///
/// These are classes that are generated only once per file.
///
/// # Note
/// Generated for alternative animations(FNIS_aa).
/// However, they are actually also reused in Offset Arm Animations, so they must be generated.
///
/// See: `FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt`
pub fn new_mt_global_patch<'a>(
    anim_groups_states: Vec<String>,
    priority: usize,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let mut one_patches = self::alternative::new_mt_global_patch(priority);
    let mut seq_patches = vec![];

    self::furniture_root::new_mt_global_patch(
        (&mut one_patches, &mut seq_patches),
        anim_groups_states,
        priority,
    );

    (one_patches, seq_patches)
}

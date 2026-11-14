//! The form of a new work on the start screen, see `ui::start_screen`.

use gui_shared::i18n::I18nKey;
use nemesis_diff::is_valid_mod_code;

use crate::workspace::TemplateScope;

/// e.g. `NewWork { code: "mymod", output: "D:/MO2/mods/MyMod", scope: TemplateScope::Nemesis }`
#[derive(Debug, Default)]
pub struct NewWork {
    pub code: String,
    /// The mod's root: what it is exported to.
    pub output: String,
    /// Which templates it can patch, for good. See [`TemplateScope`].
    pub scope: TemplateScope,
}

impl NewWork {
    /// Why it cannot be made, if so. `exists`: is there a work with this code?
    pub fn problem(&self, exists: impl FnOnce(&str) -> bool) -> Option<I18nKey> {
        let code = self.code.trim();
        if !is_valid_mod_code(code) {
            return Some(I18nKey::NemesisModCodeInvalid);
        }
        // The conflict tool's own, which would be overwritten by it.
        if code.eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE) || exists(code) {
            return Some(I18nKey::NemesisWorkExists);
        }
        self.output.trim().is_empty().then_some(I18nKey::NemesisOutputMissing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(code: &str, output: &str, exists: bool) -> Option<I18nKey> {
        NewWork { code: code.to_owned(), output: output.to_owned(), scope: TemplateScope::Nemesis }.problem(|_| exists)
    }

    #[test]
    fn a_new_work_needs_a_free_code_and_an_output() {
        assert_eq!(problem("mymod", "D:/out", false), None);
        assert_eq!(problem(" mymod ", "D:/out", false), None);
        assert_eq!(problem("", "D:/out", false), Some(I18nKey::NemesisModCodeInvalid));
        assert_eq!(problem("toolongcode", "D:/out", false), Some(I18nKey::NemesisModCodeInvalid));
        assert_eq!(problem("mymod", "D:/out", true), Some(I18nKey::NemesisWorkExists));
        assert_eq!(problem("upatch", "D:/out", false), Some(I18nKey::NemesisWorkExists));
        assert_eq!(problem("mymod", "  ", false), Some(I18nKey::NemesisOutputMissing));
    }
}

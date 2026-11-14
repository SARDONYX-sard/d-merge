//! The windows over the page, drawn while open: each changes the work and what is selected, and
//! says what others must show of it ([`WindowDone`]).
//!
//! ```text
//! code_choice   which code of a mod with several      -> WindowDone::OpenCode
//! picker        a template to add as a behavior
//! class_picker  New object: a class                   (a behavior is shown)
//! repeat        many objects from one
//! anim_data     the animation data of the mod's clips -> WindowDone::Reload
//! new_clips     clips of the clip data
//! duplicate     files to another project              -> WindowDone::Duplicated
//! ```

use egui_widgets::notify::Notification;
use gui_shared::i18n::I18nMap;

use crate::patch::{
    tool::{selection::Selection, work_state::Work},
    ui::{
        anim_data::{self, FormAsk},
        code_choice::{ChoiceAsk, ModCodeChoice},
        panels::Panels,
        repeat::RepeatAsk,
    },
};

/// What a window did that others show.
#[derive(Debug)]
pub(in crate::patch) enum WindowDone {
    /// Objects changed under the editor: it loads the open one again.
    Reload,
    /// Files went to this project of the animation data: it opens in the list.
    Duplicated(String),
    /// This code of a mod with several is to be opened.
    OpenCode(ModCodeChoice, String),
}

impl Panels {
    /// Draws the windows that are open. See the module docs.
    pub(in crate::patch) fn windows(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        (work, sel): (&mut Work, &mut Selection),
        notify: &mut Notification,
    ) -> Vec<WindowDone> {
        let mut done = Vec::new();
        done.extend(self.code_choice_window(ctx, i18n));
        if !work.place.is_open() {
            return done;
        }

        self.template_picker_window(ctx, i18n, (work, sel), notify);
        done.extend(self.anim_data_window(ctx, i18n, (work, sel), notify));
        self.new_clips_window(ctx, i18n, (work, sel));
        done.extend(self.duplicate_window(ctx, i18n, work, sel.active));
        self.repeat_window(ctx, i18n, (work, sel));
        sel.active = sel.active.min(work.workspace.behaviors.len().saturating_sub(1));
        if !work.workspace.behaviors.is_empty() {
            self.class_picker_window(ctx, i18n, (work, sel));
        }
        done
    }

    /// Which code of a mod with several to open.
    fn code_choice_window(&mut self, ctx: &egui::Context, i18n: &I18nMap) -> Option<WindowDone> {
        let ask = self.code_choice.as_ref()?.window(ctx, i18n)?;
        let choice = self.code_choice.take()?;
        match ask {
            ChoiceAsk::Open(code) => Some(WindowDone::OpenCode(choice, code)),
            ChoiceAsk::Close => None,
        }
    }

    /// The templates of the work's template directory: a click adds one as a behavior.
    fn template_picker_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        (work, sel): (&mut Work, &mut Selection),
        notify: &mut Notification,
    ) {
        let place = (work.place.template_dir.as_str(), work.workspace.scope);
        if let Some(path) = self.picker.window(ctx, i18n, place) {
            work.add_behavior(sel, &path, notify, i18n);
        }
    }

    /// The animation data of the mod's clips: `Generate` makes it.
    fn anim_data_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        (work, sel): (&mut Work, &mut Selection),
        notify: &mut Notification,
    ) -> Option<WindowDone> {
        let ask = self.anim_data_dialog.as_mut()?.window(ctx, i18n)?;
        let dialog = self.anim_data_dialog.take()?;
        match ask {
            FormAsk::Done => {
                dialog.generate((work, sel), i18n, notify);
                Some(WindowDone::Reload)
            }
            FormAsk::Close => None,
        }
    }

    /// New clips of the clip data: `Create` adds them.
    fn new_clips_window(&mut self, ctx: &egui::Context, i18n: &I18nMap, (work, sel): (&mut Work, &mut Selection)) {
        let Some(ask) = self.new_clips.as_mut().and_then(|form| form.window(ctx, i18n)) else { return };
        let Some(form) = self.new_clips.take() else { return };
        if ask == FormAsk::Done {
            form.create((work, sel));
        }
    }

    /// Files of the animation data `active` to another project.
    fn duplicate_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        work: &mut Work,
        active: usize,
    ) -> Option<WindowDone> {
        let ask = self.duplicate_form.as_mut()?.window(ctx, i18n)?;
        let form = self.duplicate_form.take()?;
        if ask == FormAsk::Close {
            return None;
        }
        let (files, to) = form.picked()?;
        anim_data::duplicate_files(work, active, (&files, &to));
        Some(WindowDone::Duplicated(to))
    }

    /// Many objects from one: `Create` makes them, and shows the first.
    fn repeat_window(&mut self, ctx: &egui::Context, i18n: &I18nMap, (work, sel): (&mut Work, &mut Selection)) {
        let code = work.place.mod_code.trim().to_owned();
        let Some(form) = &mut self.repeat else { return };
        let Some(ask) = form.window(ctx, i18n, nemesis_diff::is_existing_mod_code(&code)) else { return };
        let Some(form) = self.repeat.take() else { return };
        if ask == RepeatAsk::Close {
            return;
        }

        let Some(behavior) = work.workspace.behaviors.get_mut(form.behavior()) else { return };
        let first = form.create(behavior, &code);
        sel.active = form.behavior();
        work.reset_names();
        work.mark_dirty();
        sel.selected = first.map(Into::into);
    }

    /// New object: a class picked is a new object of it, shown.
    fn class_picker_window(&mut self, ctx: &egui::Context, i18n: &I18nMap, (work, sel): (&mut Work, &mut Selection)) {
        let behavior = &work.workspace.behaviors[sel.active];
        let Some(class) = self.class_picker.window(ctx, i18n, behavior) else { return };

        let code = work.place.mod_code.trim().to_owned();
        let Some(id) = work.workspace.behaviors[sel.active].add_class(&code, &class) else { return };
        work.object_changed(&id);
        sel.selected = Some(id.into());
        self.class_picker.close();
    }
}

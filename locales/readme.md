# Localization

Directories are BCP-47 language tags ([list](https://partnerhub.warnermediagroup.com/metadata/languages)).
English is the doc comment of each `I18nKey` (`crates/gui/gui_shared/src/i18n.rs`).

## Rules

- Reuse a key first. One likely to be reused is named for no page from the start.
- Keys in alphabetical order (checked by a test).
- Names: `*_label`, `*_hover`, `*_hint`.
- `directory` (`dir` in labels), never `folder`.
- `behavior`, `actor` stay English.

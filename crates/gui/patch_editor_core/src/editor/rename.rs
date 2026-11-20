//! `F2`: renames an event or a variable the mod adds, in every object of the behavior.
//!
//! | Where the cursor is                                  | What is renamed                     |
//! | ---------------------------------------------------- | ----------------------------------- |
//! | `$eventID[myEvent]$` / `$variableID[myVar]$`          | that name                           |
//! | `<hkcstring>myEvent</hkcstring>` in `eventNames` ... | that name                           |
//!
//! It changes the name's line in the names object (`hkbBehaviorGraphStringData`) and every
//! `$eventID[name]$` / `$variableID[name]$` of it (case ignored, as Nemesis does). The numbers that
//! point at it stay: its index does not change.
//!
//! Only a name the mod adds: one of the template is used by other mods and by the animations
//! (their annotations name events), which a rename here would not reach.

use std::{borrow::Cow, ops::Range};

use egui_code_editor::DocumentEdit;
use nemesis_diff::{schema, vars};

use super::language::{Kind, PREFIXES, Symbol, symbol_at};
use crate::workspace::{Behavior, STRING_DATA};

/// The name at `line` / `column` of the object `text`: its kind, its char columns in the line, and
/// the name.
///
/// e.g. `<hkparam name="eventId">$eventID[myEvent]$</hkparam>`, on `myEvent` ->
/// `(Event, 33..40, "myEvent")`
pub(super) fn name_at(text: &str, line: usize, column: usize) -> Option<(Kind, Range<usize>, &str)> {
    let line_text = text.split('\n').nth(line)?;
    if let Some(Symbol::Name(kind, name)) = symbol_at(line_text, column) {
        return Some((kind, columns_of(line_text, name), name));
    }

    let kind = match schema::array_element(text, line, column)?.field {
        "eventNames" => Kind::Event,
        "variableNames" => Kind::Variable,
        _ => return None,
    };
    let name = cstring(line_text)?;
    Some((kind, columns_of(line_text, name), name))
}

/// The edits that rename `old` (of `kind`) to `new` in every object of `behavior` but those of the
/// referenced mods. See the module docs.
pub(super) fn edits<'b>(
    behavior: &'b Behavior,
    kind: Kind,
    (old, new): (&str, &str),
) -> Result<Vec<DocumentEdit<'b>>, Cow<'static, str>> {
    check_name(new)?;
    let field = kind.names_field();
    // The behavior's own, as `ids::IdIndex` finds it: not a referenced mod's.
    let own = behavior.texts().find(|object| object.class == STRING_DATA && !behavior.is_ref(object.id));
    let (names_id, names) = own.map(|object| (object.id, object.text)).ok_or("This behavior has no names object")?;
    let template = behavior.template_object(names_id).map(|object| object.text.as_str());
    if template.is_some_and(|template| vars::name_line(template, field, old).is_some()) {
        return Err(format!("`{old}` is the template's: other mods and the animations use it").into());
    }
    let line = vars::name_line(names, field, old).ok_or_else(|| format!("`{old}` is not one this mod adds"))?;
    if old == new {
        return Ok(Vec::new());
    }
    let taken = |text: &str| vars::name_line(text, field, new).is_some();
    if !old.eq_ignore_ascii_case(new) && (taken(names) || behavior.ref_names_objects().any(|(_, text)| taken(text))) {
        return Err(format!("`{new}` is taken").into());
    }

    let mut edits = Vec::new();
    let line_text = names.split('\n').nth(line).unwrap_or_default();
    if let Some(name) = cstring(line_text) {
        let columns = columns_of(line_text, name);
        edits.push(DocumentEdit { document: names_id.into(), line, columns, new_text: new.to_owned() });
    }
    for object in behavior.texts().filter(|object| !behavior.is_ref(object.id)) {
        for (line, line_text) in object.text.split('\n').enumerate() {
            edits.extend(uses(line_text, kind, old).map(|columns| DocumentEdit {
                document: object.id.into(),
                line,
                columns,
                new_text: new.to_owned(),
            }));
        }
    }
    Ok(edits)
}

/// A name Nemesis can read inside `$eventID[...]$` and an `<hkcstring>`.
fn check_name(name: &str) -> Result<(), Cow<'static, str>> {
    if name.is_empty() {
        return Err("A name is needed".into());
    }
    match name.chars().find(|&c| c.is_whitespace() || "<>&\"[]$".contains(c)) {
        Some(c) => Err(format!("A name can't have `{c}`").into()),
        None => Ok(()),
    }
}

/// The char columns of each `name` of `$eventID[name]$` (of `kind`) in `line`, case ignored.
fn uses<'l>(line: &'l str, kind: Kind, name: &'l str) -> impl Iterator<Item = Range<usize>> + 'l {
    let prefix = PREFIXES.iter().find(|(_, of)| *of == kind).map_or("", |(prefix, _)| *prefix);
    line.match_indices('$').filter_map(move |(start, _)| {
        let rest = &line[start..];
        let used = rest.get(prefix.len()..)?.split("]$").next()?;
        let is_use = rest.get(..prefix.len())?.eq_ignore_ascii_case(prefix) && used.eq_ignore_ascii_case(name);
        (is_use && rest[prefix.len() + used.len()..].starts_with("]$")).then(|| columns_of(line, used))
    })
}

/// `moveStart` of `<hkcstring>moveStart</hkcstring>`, borrowed from `line`.
fn cstring(line: &str) -> Option<&str> {
    line.trim().strip_prefix("<hkcstring>")?.strip_suffix("</hkcstring>")
}

/// The char columns of `part`, a slice of `line`.
fn columns_of(line: &str, part: &str) -> Range<usize> {
    let byte = (part.as_ptr() as usize).saturating_sub(line.as_ptr() as usize).min(line.len());
    let start = line[..byte].chars().count();
    start..start + part.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On a use and on its line in the names: the name and its columns.
    #[test]
    fn a_name_is_found_where_it_is() {
        let text = "<hkparam name=\"eventId\">$eventID[myEvent]$</hkparam>";
        let (kind, columns, name) = name_at(text, 0, 36).unwrap();
        assert_eq!((kind, columns, name), (Kind::Event, 33..40, "myEvent"));

        let names = "<hkobject name=\"#0083\" class=\"hkbBehaviorGraphStringData\">\n\t<hkparam name=\"eventNames\" numelements=\"1\">\n\t\t<hkcstring>myEvent</hkcstring>\n\t</hkparam>\n</hkobject>";
        let (kind, columns, name) = name_at(names, 2, 14).unwrap();
        assert_eq!((kind, columns, name), (Kind::Event, 13..20, "myEvent"));
        assert!(name_at(names, 0, 5).is_none(), "not a name");
    }

    /// Every use, case ignored; another kind's or a longer name's is not one.
    #[test]
    fn uses_are_found_case_ignored() {
        let line = "$EventID[myevent]$ $variableID[myEvent]$ $eventID[myEvent2]$ $eventID[myEvent]$";
        assert_eq!(uses(line, Kind::Event, "myEvent").collect::<Vec<_>>(), [9..16, 70..77]);
    }

    /// The name's line and every use, in each object; a taken name and a template's are refused.
    #[test]
    fn a_name_is_renamed_everywhere() {
        use crate::editor::ids::tests::{behavior_of, object};

        let names = "<hkparam name=\"eventNames\" numelements=\"2\">\n\t<hkcstring>a</hkcstring>\n\t<hkcstring>b</hkcstring>\n</hkparam>\n";
        let uses = "<hkparam name=\"id\">$eventID[B]$</hkparam>\n<hkparam name=\"other\">$eventID[a]$</hkparam>\n";
        let behavior =
            behavior_of(vec![object("#s", STRING_DATA, names), object("#e", "hkbEventDrivenModifier", uses)]);

        let renamed = edits(&behavior, Kind::Event, ("b", "c")).unwrap();
        let text_of = |id: &str| {
            let of_it = renamed.iter().filter(|edit| edit.document == id);
            DocumentEdit::apply(behavior.text(id).unwrap(), of_it)
        };
        assert!(text_of("#s").contains("<hkcstring>c</hkcstring>"), "its line");
        assert!(text_of("#e").contains("$eventID[c]$</hkparam>\n<hkparam name=\"other\">$eventID[a]$"), "its use only");

        assert!(edits(&behavior, Kind::Event, ("b", "A")).is_err(), "taken, case ignored");
        assert!(edits(&behavior, Kind::Event, ("b", "B")).is_ok(), "its own case");
        assert!(edits(&behavior, Kind::Event, ("x", "y")).is_err(), "not this mod's");
    }

    #[test]
    fn a_name_has_no_markup() {
        assert!(check_name("my.Event_2").is_ok());
        for bad in ["", "a b", "a]", "a$", "<a"] {
            assert!(check_name(bad).is_err(), "{bad}");
        }
    }
}

//! XML highlighting + completion in the object editor.
//!
//! | Typed                              | Candidates                                        |
//! | ---------------------------------- | ------------------------------------------------- |
//! | `$eventID[` / `$variableID[`       | the behavior's event / variable names (`name]$`)  |
//! | `<hkparam name="flags">FLAG_A\|`   | values this field has in the template (`FLAG_*`)  |
//! | `#`                                | object ids (with class and name)                  |
//!
//! The class table only knows field *types*, not enum values, so the values come from the
//! template itself: every `UPPER_CASE` token a field has anywhere in it.

use std::ops::Range;

use egui::Color32;
use egui_code_editor::{CompletionItem, CompletionList, Language, languages::Xml};
use fast_hash::HashMap;
use nemesis_diff::schema::{self, FieldSchema};

use super::workspace::Behavior;

/// Nemesis' variable syntaxes and what they complete with.
const PREFIXES: [(&str, Kind); 2] = [("$eventID[", Kind::Event), ("$variableID[", Kind::Variable)];

#[derive(Clone, Copy)]
enum Kind {
    Event,
    Variable,
}

/// Field name -> the enum / flag values it has in the template (sorted).
pub(crate) type FieldValues = HashMap<String, Vec<String>>;

/// The names and values to complete with.
#[derive(Debug)]
pub(crate) struct NemesisXml<'a> {
    pub(crate) events: &'a [String],
    pub(crate) variables: &'a [String],
    pub(crate) values: &'a FieldValues,
    /// `(id, "class name")`
    pub(crate) pointers: &'a [(String, String)],
    /// The class around the cursor and its XML fields (class schema).
    pub(crate) fields: Option<(&'a str, &'a [&'static FieldSchema])>,
}

impl Language for NemesisXml<'_> {
    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(Range<usize>, Color32)> {
        Xml.highlight(line, visuals)
    }

    /// A field: its class and type. A pointer: the object's class and name.
    fn hover(&self, line: &str, column: usize) -> Option<String> {
        let chars: Vec<char> = line.chars().collect();
        // A `#id` under the cursor.
        let is_id = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '#';
        if chars.get(column).is_some_and(|&c| is_id(c)) {
            let start = chars[..column].iter().rposition(|&c| !is_id(c)).map_or(0, |i| i + 1);
            let end =
                chars[column..].iter().position(|&c| !is_id(c)).map_or(chars.len(), |i| column + i);
            let word: String = chars[start..end].iter().collect();
            if let Some((id, doc)) = self.pointers.iter().find(|(id, _)| *id == word) {
                return Some(format!("**{id}**: {doc}"));
            }
        }
        let (class, field) = self.field_of_line(line)?;
        Some(field_doc(class, field))
    }

    /// `hkbClipGenerator.mode: String` for the field of the cursor line.
    fn signature_help(
        &self,
        line: &str,
        _column: usize,
    ) -> Option<egui_code_editor::SignatureHelp> {
        let (class, field) = self.field_of_line(line)?;
        Some(egui_code_editor::SignatureHelp {
            label: format!("{class}.{}: {}", field.name, type_label(field)),
            // Enum / flags values, the current one emphasized.
            params: field.values.iter().map(|v| (*v).to_string()).collect(),
            active: field.values.iter().position(|v| line.contains(v)).unwrap_or(usize::MAX),
            documentation: None,
        })
    }

    fn completion_triggers(&self) -> &[char] {
        &['[', '|', '#', '>', '<', '"']
    }

    fn complete(&self, line: &str, column: usize) -> CompletionList {
        let before: String = line.chars().take(column).collect();
        self.nemesis_variable(line, &before, column)
            .or_else(|| self.field_name(line, &before, column))
            .or_else(|| self.pointer_value(&before, column))
            .or_else(|| self.pointer(&before, column))
            .or_else(|| self.field_value(&before, column))
            .unwrap_or_default()
    }
}

impl NemesisXml<'_> {
    /// `<hkparam name="|`: the class's fields (`name">default</hkparam>`), or, at `<` / `<hk..`
    /// starting a line, whole `<hkparam name="f">default</hkparam>` snippets.
    fn field_name(&self, line: &str, before: &str, column: usize) -> Option<CompletionList> {
        let (class, fields) = self.fields?;
        let depth = line.chars().take_while(|&c| c == '\t').count();
        const OPEN: &str = "<hkparam name=\"";

        if let Some(at) = before.rfind(OPEN)
            && !before[at + OPEN.len()..].contains('"')
        {
            let typed = before[at + OPEN.len()..].to_lowercase();
            let items = fields
                .iter()
                .filter(|field| field.name.to_lowercase().contains(&typed))
                .map(|field| {
                    let snippet = schema::field_snippet(field, depth);
                    let insert = snippet[OPEN.len()..].to_string();
                    let mut item = item(field.name, "▪", insert, field_doc(class, field));
                    item.select = value_range(&item.insert);
                    item
                })
                .collect();
            let start = before[..at + OPEN.len()].chars().count();
            return Some(CompletionList { replace: start..column, items });
        }

        // An empty line (`Ctrl+Space`) or `<`, `<hk`, ...: whole field snippets.
        let trimmed = before.trim_start();
        if trimmed.is_empty() || (trimmed.starts_with('<') && "<hkparam".starts_with(trimmed)) {
            let items = fields
                .iter()
                .map(|field| {
                    let insert = schema::field_snippet(field, depth);
                    let mut item = item(field.name, "▪", insert, field_doc(class, field));
                    item.select = value_range(&item.insert);
                    item
                })
                .collect();
            let start = column - trimmed.chars().count();
            return Some(CompletionList { replace: start..column, items });
        }
        None
    }

    /// The field of the `<hkparam name="..">` on `line`.
    fn field_of_line(&self, line: &str) -> Option<(&str, &'static FieldSchema)> {
        let (class, fields) = self.fields?;
        let name = line.split_once("<hkparam name=\"")?.1.split('"').next()?;
        fields.iter().find(|field| field.name == name).map(|field| (class, *field))
    }

    /// After an unclosed `$eventID[` / `$variableID[`.
    fn nemesis_variable(&self, line: &str, before: &str, column: usize) -> Option<CompletionList> {
        let (at, prefix, kind) = PREFIXES
            .iter()
            .filter_map(|&(prefix, kind)| before.rfind(prefix).map(|at| (at, prefix, kind)))
            .max_by_key(|&(at, ..)| at)?;
        let typed = &before[at + prefix.len()..];
        if typed.contains(']') {
            return None;
        }
        let start = before[..at + prefix.len()].chars().count();
        // Replace an existing `]$` too, so accepting does not double it.
        let after: String = line.chars().skip(column).take(2).collect();
        let end = if after == "]$" { column + 2 } else { column };

        let (names, doc, icon) = match kind {
            Kind::Event => (self.events, "Event", "⚡"),
            Kind::Variable => (self.variables, "Variable", "𝑥"),
        };
        let typed = typed.to_lowercase();
        let items = names
            .iter()
            .enumerate()
            .filter(|(_, name)| name.to_lowercase().contains(&typed))
            .map(|(index, name)| item(name, icon, format!("{name}]$"), format!("{doc} #{index}")))
            .collect();
        Some(CompletionList { replace: start..end, items })
    }

    /// In the value of a pointer field: `null` and the objects of the field's class (or a subclass).
    fn pointer_value(&self, before: &str, column: usize) -> Option<CompletionList> {
        use nemesis_diff::schema::{TypeKind, is_subclass};
        let open = before.rfind("<hkparam name=\"")?;
        let rest = &before[open + "<hkparam name=\"".len()..];
        let (name, value) = rest.split_once('"')?;
        let value = value.split_once('>')?.1;
        let field = self.fields?.1.iter().find(|f| f.name == name)?;
        if value.contains('<')
            || (field.kind != TypeKind::Pointer && field.sub_kind != TypeKind::Pointer)
        {
            return None;
        }
        let typed = value.rsplit(char::is_whitespace).next().unwrap_or_default();
        let typed_lower = typed.to_lowercase();
        let fits = |doc: &str| {
            let class = doc.split_whitespace().next().unwrap_or_default();
            field.class.is_none_or(|expected| is_subclass(class, expected))
        };
        let null = (field.kind == TypeKind::Pointer && "null".starts_with(&typed_lower))
            .then(|| item("null", "∅", "null".to_string(), "No object".to_string()));
        let items = null
            .into_iter()
            .chain(
                self.pointers
                    .iter()
                    .filter(|(id, doc)| fits(doc) && id.to_lowercase().starts_with(&typed_lower))
                    .take(200)
                    .map(|(id, doc)| item(id, "→", id.clone(), doc.clone())),
            )
            .collect();
        Some(CompletionList { replace: column - typed.chars().count()..column, items })
    }

    /// A `#...` token being typed.
    fn pointer(&self, before: &str, column: usize) -> Option<CompletionList> {
        let token_start = before.rfind('#')?;
        let typed = &before[token_start..];
        if !typed[1..].chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') {
            return None;
        }
        let typed_lower = typed.to_lowercase();
        let items = self
            .pointers
            .iter()
            .filter(|(id, doc)| {
                id.to_lowercase().starts_with(&typed_lower)
                    || (typed.len() > 1 && doc.to_lowercase().contains(&typed_lower[1..]))
            })
            .take(200)
            .map(|(id, doc)| item(id, "→", id.clone(), doc.clone()))
            .collect();
        Some(CompletionList { replace: column - typed.chars().count()..column, items })
    }

    /// Inside `<hkparam name="x">...` (before its `</hkparam>`): the field's known values.
    fn field_value(&self, before: &str, column: usize) -> Option<CompletionList> {
        let open = before.rfind("<hkparam name=\"")?;
        let rest = &before[open + "<hkparam name=\"".len()..];
        let (field, value) = rest.split_once('"')?;
        let value = value.split_once('>')?.1;
        if value.contains('<') {
            return None;
        }
        // The schema's enum / flags values; else what the template uses for this field name.
        let schema_values = self
            .fields
            .and_then(|(_, fields)| fields.iter().find(|f| f.name == field))
            .map(|f| f.values)
            .filter(|values| !values.is_empty());
        let (values, doc): (Vec<&str>, &str) = match schema_values {
            Some(values) => (values.to_vec(), "class schema"),
            None => (
                self.values.get(field)?.iter().map(String::as_str).collect(),
                "used in the template",
            ),
        };
        // The token after the last `|` (flags) or the start of the value.
        let typed = value.rsplit('|').next().unwrap_or_default().trim_start();
        let typed_lower = typed.to_lowercase();
        let items = values
            .iter()
            .filter(|v| v.to_lowercase().contains(&typed_lower))
            .map(|v| item(v, "◆", (*v).to_string(), format!("`{field}` value ({doc})")))
            .collect();
        Some(CompletionList { replace: column - typed.chars().count()..column, items })
    }
}

/// `**hkbClipGenerator.mode**: Enum(Int8)` + its values.
fn field_doc(class: &str, field: &FieldSchema) -> String {
    let mut doc = format!("**{class}.{}**: `{}`", field.name, type_label(field));
    if let Some(target) = field.class {
        doc.push_str(&format!(" → `{target}`"));
    }
    if !field.values.is_empty() {
        let sep = if field.kind == nemesis_diff::schema::TypeKind::Flags { " | " } else { ", " };
        doc.push_str(&format!("\n\n{}", field.values.join(sep)));
    }
    doc
}

/// `Real`, `Array<Pointer>`, `Enum(Int8)`.
fn type_label(field: &FieldSchema) -> String {
    use nemesis_diff::schema::TypeKind;
    match field.kind {
        TypeKind::Array | TypeKind::SimpleArray => {
            format!("{:?}<{:?}>", field.kind, field.sub_kind)
        }
        TypeKind::Enum | TypeKind::Flags => format!("{:?}({:?})", field.kind, field.sub_kind),
        kind => format!("{kind:?}"),
    }
}

fn item(label: &str, icon: &'static str, insert: String, documentation: String) -> CompletionItem {
    CompletionItem { label: label.to_string(), icon, insert, select: None, documentation }
}

/// Every `UPPER_CASE` token per field name in the template (enum and flag values).
pub(crate) fn field_values(behavior: &Behavior) -> FieldValues {
    let mut values: HashMap<String, std::collections::BTreeSet<String>> = HashMap::default();
    for object in &behavior.objects {
        for line in object.text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("<hkparam name=\"") else { continue };
            let Some((field, rest)) = rest.split_once("\">") else { continue };
            let Some((value, _)) = rest.split_once("</hkparam>") else { continue };
            for token in value.split('|').map(str::trim) {
                let is_enum = token.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    && token
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                    && token.contains('_');
                if is_enum {
                    values.entry(field.to_string()).or_default().insert(token.to_string());
                }
            }
        }
    }
    values.into_iter().map(|(field, set)| (field, set.into_iter().collect())).collect()
}

/// `(id, "class name")` of every object (the template's and the new ones).
pub(crate) fn pointer_items(behavior: &Behavior) -> Vec<(String, String)> {
    let new = behavior.added.iter().map(|(id, text)| {
        let class =
            text.split_once("class=\"").and_then(|(_, r)| r.split_once('"')).map_or("", |(c, _)| c);
        (id.clone(), format!("{class} (new)"))
    });
    let template =
        behavior.objects.iter().map(|o| (o.id.clone(), format!("{} {}", o.class, o.name)));
    new.chain(template).collect()
}

/// The value between `>` and `</hkparam>` of a one-line snippet (selected after inserting).
fn value_range(insert: &str) -> Option<Range<usize>> {
    let start = insert.find('>')? + 1;
    let end = insert[start..].find("</hkparam>")? + start;
    // Char indices (the editor selects by char).
    Some(insert[..start].chars().count()..insert[..end].chars().count())
}

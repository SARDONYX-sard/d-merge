//! XML highlighting + completion in the object editor.
//!
//! | Typed                              | Candidates                                        |
//! | ---------------------------------- | ------------------------------------------------- |
//! | `$eventID[` / `$variableID[`       | the behavior's event / variable names (`name]$`)  |
//! | `<hkparam name="flags">FLAG_A\|`   | values this field has in the template (`FLAG_*`)  |
//! | `#`                                | object ids (with class and name)                  |
//! | `<hkparam name="id">` (int fields) | `$eventID[name]$` / `$variableID[name]$`          |
//!
//! Nemesis resolves `$eventID[moveStart]$` to the index of `moveStart` in `eventNames` when the
//! mod is installed, so a patch does not hard-code an index that another mod may shift. Hover shows
//! the index, and go-to-definition / find-references work on these and on `#id` pointers
//! ([`Symbol`]).
//!
//! The class table only knows field *types*, not enum values, so the values come from the
//! template itself: every `UPPER_CASE` token a field has anywhere in it.

use std::{borrow::Cow, fmt::Write as _, ops::Range};

use egui::Color32;
use egui_code_editor::{CompletionItem, CompletionList, Language, Location, References, languages::Xml};
use fast_hash::HashMap;
use nemesis_diff::{
    schema::{self, FieldSchema, TypeKind},
    vars,
};

use super::{tree::Graph, workspace::Behavior};

/// Nemesis' variable syntaxes and what they complete with.
const PREFIXES: [(&str, Kind); 2] = [("$eventID[", Kind::Event), ("$variableID[", Kind::Variable)];

#[derive(Clone, Copy)]
enum Kind {
    Event,
    Variable,
}

impl Kind {
    /// The array of `hkbBehaviorGraphStringData` that holds the names.
    const fn names_field(self) -> &'static str {
        match self {
            Self::Event => "eventNames",
            Self::Variable => "variableNames",
        }
    }

    const fn prefix(self) -> &'static str {
        match self {
            Self::Event => "$eventID[",
            Self::Variable => "$variableID[",
        }
    }
}

/// What can be navigated to in an object's text. Borrows from the line.
#[derive(Clone, Copy)]
enum Symbol<'a> {
    /// `#0106`, `#mymod$0`: another object.
    Pointer(&'a str),
    /// `$eventID[moveStart]$` -> `(Event, "moveStart")`
    Name(Kind, &'a str),
}

/// The symbol at char `column` of `line` (on it or right after it).
fn symbol_at(line: &str, column: usize) -> Option<Symbol<'_>> {
    let at = line.char_indices().nth(column).map_or(line.len(), |(i, _)| i);

    // `$eventID[name]$`. The prefix ignores case, as d-merge's parser does.
    for (start, _) in line.match_indices('$') {
        for (prefix, kind) in PREFIXES {
            let rest = &line[start..];
            if rest.get(..prefix.len()).is_some_and(|p| p.eq_ignore_ascii_case(prefix))
                && let Some(end) = rest.find("]$")
                && (start..=start + end + 2).contains(&at)
            {
                return Some(Symbol::Name(kind, &rest[prefix.len()..end]));
            }
        }
    }

    // `#0106`, also inside `name="#0106"`.
    let is_id = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '#';
    let start = line[..at].rfind(|c| !is_id(c)).map_or(0, |i| i + 1);
    let end = line[at..].find(|c| !is_id(c)).map_or(line.len(), |i| at + i);
    let word = line[start..end].trim_end_matches('$');
    word.starts_with('#').then_some(Symbol::Pointer(word))
}

/// Finds `token` not followed by more id chars, so `#010` does not match in `#0105`.
fn find_token(text: &str, token: &str) -> Option<usize> {
    let is_id = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    text.match_indices(token).map(|(at, _)| at).find(|at| !text[at + token.len()..].starts_with(is_id))
}

/// 0-based line of the byte offset `at`.
fn line_of(text: &str, at: usize) -> usize {
    text.as_bytes()[..at].iter().filter(|&&b| b == b'\n').count()
}

/// Field name -> the enum / flag values it has in the template (sorted).
pub(crate) type FieldValues = HashMap<String, Vec<String>>;

/// The names and values to complete with.
#[derive(Debug)]
pub(crate) struct NemesisXml<'a> {
    pub(crate) events: &'a [String],
    pub(crate) variables: &'a [String],
    pub(crate) values: &'a FieldValues,
    /// Every object's id and description, for `#` completion and hovers.
    pub(crate) graph: &'a Graph,
    /// The class around the cursor and its XML fields (class schema).
    pub(crate) fields: Option<(&'a str, &'a [&'static FieldSchema])>,
    /// Every object, for go-to-definition and find-references.
    pub(crate) behavior: &'a Behavior,
}

impl Language for NemesisXml<'_> {
    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(Range<usize>, Color32)> {
        Xml.highlight(line, visuals)
    }

    /// Up to three paragraphs, all that apply:
    /// ```text
    /// **eventNames[12]** of 279        an array element
    /// **$eventID[moveStart]$**: Event #12     or     **#0106**: hkbModifierList GetUpModList
    /// **hkbClipGenerator.mode**: `Enum(Int8)`   a field of the class around that line
    /// ```
    fn hover_at(&self, text: &str, line_no: usize, column: usize) -> Option<String> {
        let line = text.split('\n').nth(line_no)?;
        let mut doc = String::new();
        if let Some(element) = schema::array_element(text, line_no, column) {
            let _ = write!(doc, "**{}[{}]**", element.field, element.index);
            if let Some(len) = element.len {
                let _ = write!(doc, " of {len}");
            }
            doc.push_str("\n\n");
        }
        match symbol_at(line, column) {
            Some(Symbol::Pointer(word)) => {
                if let Some(node) = self.graph.node_of(word) {
                    let _ = write!(doc, "**{word}**: {}\n\n", self.graph.doc(node));
                }
            }
            Some(Symbol::Name(kind, name)) => {
                let prefix = kind.prefix();
                match self.names_of(kind).iter().position(|n| n.eq_ignore_ascii_case(name)) {
                    Some(index) => {
                        let label = match kind {
                            Kind::Event => "Event",
                            Kind::Variable => "Variable",
                        };
                        let _ = write!(doc, "**{prefix}{name}]$**: {label} #{index}\n\n");
                    }
                    None => {
                        let _ = write!(doc, "**{prefix}{name}]$**: not found\n\n");
                    }
                }
            }
            None => {}
        }
        // The class around the hovered line (`self.fields` is the one around the cursor).
        if let Some((class, fields)) = schema::field_context(text, line_no)
            && let Some(name) = field_name_of(line)
            && let Some(field) = fields.iter().find(|field| field.name == name)
        {
            doc.push_str(&field_doc(&class, field));
        }
        let doc = doc.trim_end();
        (!doc.is_empty()).then(|| doc.to_owned())
    }

    /// `#0106`: that object. `$eventID[x]$`: the line of `x` in the names object.
    fn definition(&self, text: &str, line: usize, column: usize) -> Option<Location<'_>> {
        let behavior = self.behavior;
        match symbol_at(text.split('\n').nth(line)?, column)? {
            Symbol::Pointer(word) => {
                let object = behavior.texts().find(|object| object.id == word)?;
                Some(Location { document: object.id.into(), line: 0, detail: object.class.into() })
            }
            Symbol::Name(kind, name) => {
                let id = behavior.object_of_class("hkbBehaviorGraphStringData")?;
                let line = vars::name_line(behavior.text(id)?, kind.names_field(), name)?;
                Some(Location { document: id.into(), line, detail: kind.names_field().into() })
            }
        }
    }

    /// Every other object mentioning the `#id` (its first mention), or every line using the
    /// `$eventID[x]$`. Template objects use plain indices for events and variables, so only
    /// `$..$` uses (new or edited text) can be found.
    fn references(&self, text: &str, line: usize, column: usize) -> Option<References<'_>> {
        let mut locations: Vec<Location<'_>> = Vec::new();
        let symbol =
            match symbol_at(text.split('\n').nth(line)?, column)? {
                Symbol::Pointer(word) => {
                    for object in self.behavior.texts().filter(|object| object.id != word) {
                        if let Some(at) = find_token(object.text, word) {
                            locations.push(Location {
                                document: object.id.into(),
                                line: line_of(object.text, at),
                                detail: object.class.into(),
                            });
                        }
                    }
                    Cow::Owned(word.to_owned())
                }
                Symbol::Name(kind, name) => {
                    let token = format!("{}{name}]$", kind.prefix()).to_ascii_lowercase();
                    for object in self.behavior.texts() {
                        let lines = object.text.lines().enumerate();
                        locations.extend(lines.filter(|(_, l)| l.to_ascii_lowercase().contains(&token)).map(
                            |(line, _)| Location { document: object.id.into(), line, detail: object.class.into() },
                        ));
                    }
                    Cow::Owned(token)
                }
            };
        Some(References { symbol, locations })
    }

    /// `hkbClipGenerator.mode: String` for the field of the cursor line.
    fn signature_help(&self, line: &str, _column: usize) -> Option<egui_code_editor::SignatureHelp> {
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
        &['[', '|', '#', '>', '<', '"', '$']
    }

    fn complete(&self, line: &str, column: usize) -> CompletionList {
        let before: String = line.chars().take(column).collect();
        self.nemesis_variable(line, &before, column)
            .or_else(|| self.field_name(line, &before, column))
            .or_else(|| self.pointer_value(&before, column))
            .or_else(|| self.integer_value(&before, column))
            .or_else(|| self.pointer(&before, column))
            .or_else(|| self.field_value(&before, column))
            .unwrap_or_default()
    }
}

impl NemesisXml<'_> {
    const fn names_of(&self, kind: Kind) -> &[String] {
        match kind {
            Kind::Event => self.events,
            Kind::Variable => self.variables,
        }
    }

    /// In the value of an integer field: `$eventID[name]$` and `$variableID[name]$` of every name.
    ///
    /// e.g., `<hkparam name="id">$mov|` lists `$eventID[moveStart]$` (doc: `Event #12`), ...
    /// `typed` is matched against the name, so typing `$`, `$mov` or `mov` all work.
    fn integer_value(&self, before: &str, column: usize) -> Option<CompletionList> {
        let (name, value) = before.rsplit_once("<hkparam name=\"")?.1.split_once('"')?;
        let value = value.split_once('>')?.1;
        let field = self.fields?.1.iter().find(|f| f.name == name)?;
        let is_integer = matches!(
            field.kind,
            TypeKind::Int8
                | TypeKind::Uint8
                | TypeKind::Int16
                | TypeKind::Uint16
                | TypeKind::Int32
                | TypeKind::Uint32
                | TypeKind::Int64
                | TypeKind::Uint64
        );
        if !is_integer || value.contains('<') {
            return None;
        }
        let typed = value.trim_start();
        let lower = typed.trim_start_matches('$').to_lowercase();
        let items = [(Kind::Event, "⚡", "Event"), (Kind::Variable, "𝑥", "Variable")]
            .into_iter()
            .flat_map(|(kind, icon, label)| {
                let lower = &lower;
                self.names_of(kind).iter().enumerate().filter(move |(_, name)| name.to_lowercase().contains(lower)).map(
                    move |(index, name)| {
                        let insert = format!("{}{name}]$", kind.prefix());
                        item(&insert, icon, insert.clone(), format!("{label} #{index}"))
                    },
                )
            })
            .collect();
        Some(CompletionList { replace: column - typed.chars().count()..column, items })
    }

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
        let name = field_name_of(line)?;
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
        use nemesis_diff::schema::is_subclass;
        let open = before.rfind("<hkparam name=\"")?;
        let rest = &before[open + "<hkparam name=\"".len()..];
        let (name, value) = rest.split_once('"')?;
        let value = value.split_once('>')?.1;
        let field = self.fields?.1.iter().find(|f| f.name == name)?;
        if value.contains('<') || (field.kind != TypeKind::Pointer && field.sub_kind != TypeKind::Pointer) {
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
        let graph = self.graph;
        let items = null
            .into_iter()
            .chain(
                graph
                    .nodes()
                    .filter(|&node| fits(graph.doc(node)) && graph.id(node).to_lowercase().starts_with(&typed_lower))
                    .take(200)
                    .map(|node| pointer_item(graph, node)),
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
        let graph = self.graph;
        let items = graph
            .nodes()
            .filter(|&node| {
                graph.id(node).to_lowercase().starts_with(&typed_lower)
                    || (typed.len() > 1 && graph.doc(node).to_lowercase().contains(&typed_lower[1..]))
            })
            .take(200)
            .map(|node| pointer_item(graph, node))
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
            None => (self.values.get(field)?.iter().map(String::as_str).collect(), "used in the template"),
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

/// `<hkparam name="mode">X</hkparam>` -> `mode`
fn field_name_of(line: &str) -> Option<&str> {
    line.split_once("<hkparam name=\"")?.1.split('"').next()
}

/// `**hkbClipGenerator.mode**: Enum(Int8)` + its values.
fn field_doc(class: &str, field: &FieldSchema) -> String {
    let mut doc = format!("**{class}.{}**: `{}`", field.name, type_label(field));
    if let Some(target) = field.class {
        doc.push_str(&format!(" → `{target}`"));
    }
    if !field.values.is_empty() {
        let sep = if field.kind == TypeKind::Flags { " | " } else { ", " };
        doc.push_str(&format!("\n\n{}", field.values.join(sep)));
    }
    doc
}

/// `Real`, `Array<Pointer>`, `Enum(Int8)`.
fn type_label(field: &FieldSchema) -> String {
    match field.kind {
        TypeKind::Array | TypeKind::SimpleArray => {
            format!("{:?}<{:?}>", field.kind, field.sub_kind)
        }
        TypeKind::Enum | TypeKind::Flags => format!("{:?}({:?})", field.kind, field.sub_kind),
        kind => format!("{kind:?}"),
    }
}

/// A `#id` candidate, described by its class and name.
fn pointer_item(graph: &Graph, node: super::tree::Node) -> CompletionItem {
    let id = graph.id(node);
    item(id, "→", id.to_owned(), graph.doc(node).to_owned())
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
                    && token.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                    && token.contains('_');
                if is_enum {
                    values.entry(field.to_string()).or_default().insert(token.to_string());
                }
            }
        }
    }
    values.into_iter().map(|(field, set)| (field, set.into_iter().collect())).collect()
}

/// The value between `>` and `</hkparam>` of a one-line snippet (selected after inserting).
fn value_range(insert: &str) -> Option<Range<usize>> {
    let start = insert.find('>')? + 1;
    let end = insert[start..].find("</hkparam>")? + start;
    // Char indices (the editor selects by char).
    Some(insert[..start].chars().count()..insert[..end].chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_symbols() {
        let line = "\t\t#0105 #0109\t$eventID[moveStart]$";
        assert!(matches!(symbol_at(line, 4), Some(Symbol::Pointer("#0105"))));
        assert!(matches!(symbol_at(line, 9), Some(Symbol::Pointer("#0109"))));
        assert!(matches!(symbol_at(line, 20), Some(Symbol::Name(Kind::Event, "moveStart"))));
        assert!(symbol_at("<hkparam name=\"a\">1</hkparam>", 3).is_none());
        // Inside the name attribute of the object itself.
        assert!(matches!(symbol_at("<hkobject name=\"#mod$0\">", 18), Some(Symbol::Pointer("#mod$0"))));

        assert_eq!(find_token("#0105 #01090 #0109\n", "#0109"), Some(13));
        assert_eq!(find_token("#01090", "#0109"), None);
        assert_eq!(line_of("a\nb\nc", 2), 1);
    }
}

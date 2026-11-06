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
use egui_code_editor::{CompletionItem, CompletionList, Language, Location, QuickFix, References, languages::Xml};
use fast_hash::HashMap;
use nemesis_diff::{
    schema::{self, FieldSchema, TypeKind},
    vars,
};
use stdx::text::{contains_ignore_case, push_joined, starts_with_ignore_case};

use crate::patch::{tree::Graph, workspace::Behavior};

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
    /// `LspOptions::code_action`
    pub(in crate::patch) code_action: bool,
    pub(crate) events: &'a [String],
    pub(crate) variables: &'a [String],
    /// The type of each variable by index, e.g. `REAL`.
    pub(crate) variable_types: &'a [Box<str>],
    pub(crate) values: &'a FieldValues,
    /// Every object's id and description, for `#` completion and hovers.
    pub(crate) graph: &'a Graph,
    /// The class around the cursor and its XML fields (class schema).
    pub(crate) fields: Option<(&'a str, &'a [&'static FieldSchema])>,
    /// The array of pointers the cursor line is an element of: what its objects have to be.
    pub(crate) pointer_field: Option<&'static FieldSchema>,
    /// The hover of a push to an empty array that keeps its line (translated).
    pub(crate) kept_array_hover: &'a str,
    /// The cursor line is in an array of strings: its elements are `<hkcstring>`.
    pub(crate) in_string_array: bool,
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
                    let (_, class, name, links) = self.graph.label_parts(node);
                    let _ = write!(doc, "**{word}**: ⟦class|{}⟧ ⟦name|{}⟧{links}\n\n", class.trim(), name.trim());
                } else if let Some((code, _)) = word.trim_start_matches('#').split_once('$') {
                    let _ = write!(doc, "**{word}**: a new object of the mod `{code}`, which is not referenced.\n\n");
                }
            }
            Some(Symbol::Name(kind, name)) => {
                let prefix = kind.prefix();
                match self.names_of(kind).iter().position(|n| n.eq_ignore_ascii_case(name)) {
                    Some(index) => {
                        let _ = write!(doc, "**{prefix}{name}]$**: {}\n\n", self.name_doc(kind, index));
                    }
                    None => {
                        let _ = write!(doc, "**{prefix}{name}]$**: not found\n\n");
                    }
                }
            }
            None => {}
        }
        // `<hkobject>`, whose class may only be known from the field it is in.
        if line.trim_start().starts_with("<hkobject")
            && let Some((class, _)) = schema::field_context(text, line_no + 1)
        {
            doc.push_str(&class_hover(class));
        }
        // The class around the hovered line (`self.fields` is the one around the cursor).
        if let Some((class, fields)) = schema::field_context(text, line_no)
            && let Some(name) = field_name_of(line)
            && let Some(field) = fields.iter().find(|field| field.name == name)
        {
            doc.push_str(&field_hover(class, field));
        }
        if nemesis_diff::is_in_kept_array(text, line_no) {
            if !doc.is_empty() {
                doc.push_str("\n\n");
            }
            doc.push_str(self.kept_array_hover);
        }
        let doc = doc.trim_end();
        (!doc.is_empty()).then(|| doc.to_owned())
    }

    /// As the templates are.
    fn indent(&self) -> egui_code_editor::Indent {
        Xml.indent()
    }

    /// Indents as the templates do, so that it adds nothing to the patch.
    fn format(&self, text: &str) -> Option<String> {
        nemesis_diff::format_object(text)
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
                        locations.extend(lines.filter(|(_, l)| contains_ignore_case(l, &token)).map(|(line, _)| {
                            Location { document: object.id.into(), line, detail: object.class.into() }
                        }));
                    }
                    Cow::Owned(token)
                }
            };
        Some(References { symbol, locations })
    }

    /// At the `<hkobject>` line: its signature. Anywhere: the fields the object lacks.
    fn quick_fixes(&self, text: &str, line: usize, _column: usize) -> Vec<QuickFix> {
        let mut fixes = Vec::new();
        if !self.code_action {
            return fixes;
        }
        if let Some(source) = text.split('\n').nth(line)
            && let Some(new_text) = schema::line_with_signature(source)
        {
            let columns = 0..source.chars().count();
            fixes.push(QuickFix { title: "Fix the class signature".to_owned(), line, columns, new_text });
        }
        fixes.extend(missing_fields_fix(text));
        fixes
    }

    /// `hkbClipGenerator.mode: String` for the field of the cursor line.
    fn signature_help(&self, line: &str, _column: usize) -> Option<egui_code_editor::SignatureHelp> {
        let (class, field) = self.field_of_line(line)?;
        Some(egui_code_editor::SignatureHelp {
            label: format!("{class}.{}: {}", field.name, schema::type_label(field)),
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
            .or_else(|| self.in_string_array.then(|| string_element(&before, column)).flatten())
            .or_else(|| self.field_name(line, &before, column))
            .or_else(|| self.pointer_value(&before, column))
            .or_else(|| self.integer_value(&before, column))
            .or_else(|| self.pointer(&before, column))
            .or_else(|| self.field_value(&before, column))
            .unwrap_or_default()
    }
}

impl NemesisXml<'_> {
    /// e.g., `Event #12`, `Variable #3: REAL`
    fn name_doc(&self, kind: Kind, index: usize) -> String {
        match kind {
            Kind::Event => format!("Event #{index}"),
            Kind::Variable => match self.variable_types.get(index) {
                Some(kind) => format!("Variable #{index}: {kind}"),
                None => format!("Variable #{index}"),
            },
        }
    }

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
        let wanted = typed.trim_start_matches('$');
        let items = [(Kind::Event, "⚡"), (Kind::Variable, "𝑥")]
            .into_iter()
            .flat_map(|(kind, icon)| {
                self.names_of(kind).iter().enumerate().filter(move |(_, name)| contains_ignore_case(name, wanted)).map(
                    move |(index, name)| {
                        let insert = format!("{}{name}]$", kind.prefix());
                        item(insert.clone(), icon, insert, self.name_doc(kind, index))
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
            let typed = &before[at + OPEN.len()..];
            let items = fields
                .iter()
                .filter(|field| contains_ignore_case(field.name, typed))
                .map(|field| {
                    let mut insert = schema::field_snippet(field, depth);
                    insert.drain(..OPEN.len());
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
        let closed = line.chars().skip(column).take(2).eq("]$".chars());
        let end = if closed { column + 2 } else { column };

        let (names, icon) = match kind {
            Kind::Event => (self.events, "⚡"),
            Kind::Variable => (self.variables, "𝑥"),
        };
        let items = names
            .iter()
            .enumerate()
            .filter(|(_, name)| contains_ignore_case(name, typed))
            .map(|(index, name)| item(name.clone(), icon, format!("{name}]$"), self.name_doc(kind, index)))
            .collect();
        Some(CompletionList { replace: start..end, items })
    }

    /// In the value of a pointer field: `null` and the objects of the field's class (or a subclass).
    fn pointer_value(&self, before: &str, column: usize) -> Option<CompletionList> {
        let open = before.rfind("<hkparam name=\"")?;
        let rest = &before[open + "<hkparam name=\"".len()..];
        let (name, value) = rest.split_once('"')?;
        let value = value.split_once('>')?.1;
        let field = self.fields?.1.iter().find(|f| f.name == name)?;
        if value.contains('<') || (field.kind != TypeKind::Pointer && field.sub_kind != TypeKind::Pointer) {
            return None;
        }
        let typed = value.rsplit(char::is_whitespace).next().unwrap_or_default();
        let fits = |doc: &str| schema::pointer_accepts(field, class_of_doc(doc));
        let null = (field.kind == TypeKind::Pointer && starts_with_ignore_case("null", typed))
            .then(|| item("null", "∅", "null", "No object"));
        let graph = self.graph;
        let items = null
            .into_iter()
            .chain(
                graph
                    .nodes()
                    .filter(|&node| fits(graph.doc(node)) && starts_with_ignore_case(graph.id(node), typed))
                    .take(200)
                    .map(|node| pointer_item(graph, node)),
            )
            .collect();
        Some(CompletionList { replace: column - typed.chars().count()..column, items })
    }

    /// A `#...` token being typed. On a line of an array of pointers, only what the array can hold.
    fn pointer(&self, before: &str, column: usize) -> Option<CompletionList> {
        let token_start = before.rfind('#')?;
        let typed = &before[token_start..];
        if !typed[1..].chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') {
            return None;
        }
        let graph = self.graph;
        let fits = |doc: &str| self.pointer_field.is_none_or(|field| schema::pointer_accepts(field, class_of_doc(doc)));
        let items = graph
            .nodes()
            .filter(|&node| fits(graph.doc(node)))
            .filter(|&node| {
                starts_with_ignore_case(graph.id(node), typed)
                    || (typed.len() > 1 && contains_ignore_case(graph.doc(node), &typed[1..]))
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
        let schema_field = self.fields.and_then(|(_, fields)| fields.iter().find(|f| f.name == field));
        let schema_values = schema_field
            .map(|f| if f.kind == TypeKind::Bool { BOOL_VALUES } else { f.values })
            .filter(|values| !values.is_empty());
        let (values, doc): (Cow<'_, [&str]>, &str) = match schema_values {
            Some(values) => (Cow::Borrowed(values), "class schema"),
            None => (Cow::Owned(self.values.get(field)?.iter().map(String::as_str).collect()), "used in the template"),
        };
        // The token after the last `|` (flags) or the start of the value.
        let typed = value.rsplit('|').next().unwrap_or_default().trim_start();
        let items = values
            .iter()
            .filter(|v| contains_ignore_case(v, typed))
            .map(|v| item((*v).to_owned(), "◆", (*v).to_owned(), format!("`{field}` value ({doc})")))
            .collect();
        Some(CompletionList { replace: column - typed.chars().count()..column, items })
    }
}

/// `<hkparam name="mode">X</hkparam>` -> `mode`
fn field_name_of(line: &str) -> Option<&str> {
    line.split_once("<hkparam name=\"")?.1.split('"').next()
}

/// What a field of the type `Bool` can be.
const BOOL_VALUES: &[&str] = &["true", "false"];

/// Hovering `<hkobject>`: the class, what it inherits and its signature.
///
/// e.g., `hkbModifierList : hkbModifier : hkbBindable`, `signature 0xa4180ca1`
fn class_hover(class: &str) -> String {
    let mut doc = format!("⟦class|{class}⟧");
    for parent in schema::class_parents(class) {
        let _ = write!(doc, " : ⟦class|{parent}⟧");
    }
    if let Some(signature) = schema::class_signature(class) {
        let _ = write!(doc, "\n\n`signature` `{signature}`");
    }
    doc.push_str("\n\n");
    doc
}

/// Hovering a field: what it is. What it holds is on the line, and the values it could hold are for
/// the completion.
///
/// e.g., `hkbClipGenerator.flags: Flags(Int8)`
fn field_hover(class: &str, field: &FieldSchema) -> String {
    let mut doc = format!("⟦class|{class}⟧.**{}**: `{}`", field.name, schema::type_label(field));
    if let Some(target) = field.class {
        let _ = write!(doc, " → ⟦class|{target}⟧");
    }
    doc
}

/// Adds the fields the object lacks before its closing tag, at their defaults.
fn missing_fields_fix(text: &str) -> Option<QuickFix> {
    let missing = schema::missing_fields(text);
    if missing.is_empty() {
        return None;
    }
    let class = text.lines().next().and_then(schema::class_of_line)?;
    let lines = text.lines().enumerate().filter(|(_, line)| line.trim_start().starts_with("</hkobject>"));
    let line = lines.last()?.0;
    let mut new_text = String::new();
    for field in schema::xml_fields(class).filter(|field| missing.contains(&field.name)) {
        let _ = writeln!(new_text, "\t\t\t{}", schema::field_snippet(field, 3));
    }
    Some(QuickFix { title: format!("Add the missing fields: {}", missing.join(", ")), line, columns: 0..0, new_text })
}

/// `**hkbClipGenerator.mode**: Enum(Int8)` + its values.
fn field_doc(class: &str, field: &FieldSchema) -> String {
    let mut doc = format!("**{class}.{}**: `{}`", field.name, schema::type_label(field));
    if let Some(target) = field.class {
        let _ = write!(doc, " → `{target}`");
    }
    if !field.values.is_empty() {
        let sep = if field.kind == TypeKind::Flags { " | " } else { ", " };
        doc.push_str("\n\n");
        push_joined(&mut doc, field.values.iter().copied(), sep);
    }
    doc
}

/// The trigger names containing what is typed, up to the `:` before the time (typed by hand).
///
/// e.g., `sound|` -> `SoundPlay.Foo:`; after `SoundPlay.Foo:0` there is nothing to offer.
pub(in crate::patch) fn trigger_completion(names: &[String], before: &str, column: usize) -> CompletionList {
    let typed = before.trim_start();
    if typed.contains(':') {
        return CompletionList::default();
    }
    let items = names
        .iter()
        .filter(|name| contains_ignore_case(name, typed))
        .take(200)
        .map(|name| item(name.clone(), "⚡", format!("{name}:"), ""))
        .collect();
    CompletionList { replace: column - typed.chars().count()..column, items }
}

/// A `#id` candidate, described by its class and name.
/// The class in an object's text for completion. e.g. `hkbModifierList  GetUpModList` -> `hkbModifierList`
fn class_of_doc(doc: &str) -> &str {
    doc.split_whitespace().next().unwrap_or_default()
}

fn pointer_item(graph: &Graph, node: crate::patch::tree::Node) -> CompletionItem {
    let id = graph.id(node);
    item(id.to_owned(), "→", id.to_owned(), graph.doc(node).to_owned())
}

/// On an empty line of an array of strings, or `<`, `<hk`, ...: the tag of its elements, with the
/// cursor where the string goes.
///
/// e.g., `\t\t\t\t<hk|` -> `\t\t\t\t<hkcstring>|</hkcstring>`
fn string_element(before: &str, column: usize) -> Option<CompletionList> {
    const OPEN: &str = "<hkcstring>";
    let typed = before.trim_start();
    if !OPEN.starts_with(typed) && !OPEN[1..].starts_with(typed) {
        return None;
    }
    let mut element = item("hkcstring", "▪", "<hkcstring></hkcstring>", "An element of the array");
    element.select = Some(OPEN.len()..OPEN.len());
    Some(CompletionList { replace: column - typed.chars().count()..column, items: vec![element] })
}

pub(in crate::patch) fn item(
    label: impl Into<Cow<'static, str>>,
    icon: &'static str,
    insert: impl Into<Cow<'static, str>>,
    documentation: impl Into<Cow<'static, str>>,
) -> CompletionItem {
    CompletionItem {
        label: label.into(),
        icon,
        insert: insert.into(),
        select: None,
        documentation: documentation.into(),
    }
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
    fn completes_the_tag_of_a_string_element() {
        for before in ["\t\t\t\t", "\t\t\t\t<", "\t\t\t\t<hk", "\t\t\t\thkc", "\t\t\t\t<hkcstring"] {
            let column = before.chars().count();
            let list = string_element(before, column).unwrap();
            assert_eq!(list.replace, 4..column, "{before:?}");
            assert_eq!(list.items[0].insert, "<hkcstring></hkcstring>");
            assert_eq!(list.items[0].select, Some(11..11), "between the tags");
        }
        assert!(string_element("\t\t\t\t<hkcstring>a", 16).is_none(), "in the string");
        assert!(string_element("\t\t\t\t<hkparam", 12).is_none());
    }

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

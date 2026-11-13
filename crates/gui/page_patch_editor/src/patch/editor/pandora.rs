//! Editing a Pandora patch file: what is special about it, over what an object's text has.
//!
//! | Where                       | Completion                  | Hover          | `F12`, `Ctrl+click`, `gd`  |
//! | --------------------------- | --------------------------- | -------------- | -------------------------- |
//! | `path="#`                   | object ids                  | its class      | that object                |
//! | `path="#0106/`              | what the object has there   | the field type | the line of that field     |
//! | `<` on an empty line        | the operations              |                |                            |
//! | anywhere else               | as in an object's text ([`NemesisXml`])                                   |
//!
//! e.g. `<Replace path="#2802/transitions/Element0/interval">`

use std::{borrow::Cow, ops::Range};

use egui::Color32;
use egui_code_editor::{CompletionItem, CompletionList, Diagnostic, Language, Location, References, Severity};
use nemesis_diff::{
    open_kept_arrays,
    schema::{self, FieldSchema},
};
use pandora_patch::{
    apply::{apply_ops, has_edit},
    object_ops,
    path::{PathAt, doc_problems, line_of_element, path_at, path_of_line, resolve, split_path},
    tree::parse,
};
use stdx::text::contains_ignore_case;

use super::{
    language::{NemesisXml, item},
    preview::Preview,
};
use crate::patch::workspace::Behavior;

/// The preview of object `id` when the export is Pandora's: its operations, once it is seen that they
/// give the object as edited. `Err`: why not, which also keeps it out of the file.
pub(in crate::patch) fn op_preview(behavior: &Behavior, id: &str) -> Result<Preview, String> {
    let (Some(template), Some(edited)) = (behavior.template_object(id), behavior.edits.get(id)) else {
        return Ok(Preview::of_pandora_ops(&[]));
    };
    let edited = open_kept_arrays(edited);
    let ops = object_ops(&template.text, &edited).map_err(|problem| problem.to_string())?;
    let same = parse(&template.text)
        .zip(parse(&edited))
        .is_some_and(|(mut object, edited)| apply_ops(&mut object, &ops).is_ok() && has_edit(&object, &edited));
    if same { Ok(Preview::of_pandora_ops(&ops)) } else { Err("the operations don't give this object".to_owned()) }
}

/// What a Pandora patch can do, for the completion of `<`.
const OPERATIONS: [(&str, &str, &str); 4] = [
    (
        "Replace",
        "<Replace path=\"\">\n</Replace>",
        "Puts what it holds in place of the element at `path`.\n\n```xml\n<Replace path=\"#2773/flags\">\n  <hkparam name=\"flags\">0</hkparam>\n</Replace>\n```",
    ),
    (
        "Append",
        "<Append path=\"\">\n</Append>",
        "Adds what it holds to the end of the list at `path`.\n\n```xml\n<Append path=\"#0106/variableNames\">\n  <hkcstring>myVariable</hkcstring>\n</Append>\n```",
    ),
    (
        "Insert",
        "<Insert path=\"\">\n</Insert>",
        "Adds what it holds before the element at `path`. With an empty `path`: new objects.\n\n```xml\n<Insert path=\"\">\n  <hkobject name=\"#mine$0\" class=\"hkbClipGenerator\" signature=\"0x333b85b9\"></hkobject>\n</Insert>\n```",
    ),
    (
        "Remove",
        "<Remove path=\"\" />",
        "Removes the element at `path`.\n\n```xml\n<Remove path=\"#0367/transitions/Element0\" />\n```",
    ),
];

/// See the module docs.
#[derive(Debug)]
pub(crate) struct PandoraXml<'a> {
    pub(crate) inner: &'a NemesisXml<'a>,
}

impl PandoraXml<'_> {
    const fn behavior(&self) -> &Behavior {
        self.inner.behavior
    }

    /// The ids of the objects. `part`: what is typed of it.
    fn complete_id(&self, part: &str) -> Vec<CompletionItem> {
        let graph = self.inner.graph;
        graph
            .nodes()
            .filter(|node| contains_ignore_case(graph.id(*node), part))
            .take(200)
            .map(|node| {
                let id = graph.id(node).to_owned();
                item(id.clone(), "→", format!("{id}/"), graph.doc(node).to_owned())
            })
            .collect()
    }

    /// What the object has at `parent`. e.g. `#0106/transitions` -> `Element0`, `Element1`
    fn complete_key(&self, parent: &str, part: &str) -> Vec<CompletionItem> {
        let (id, keys) = split_path(parent);
        let Some(text) = self.behavior().text(id) else { return Vec::new() };
        let Some(object) = parse(text) else { return Vec::new() };
        let keys: Vec<&str> = keys.collect();
        let Some(element) = resolve(&object, keys.iter().copied()) else { return Vec::new() };
        let owner = object.attribute("class").map(|class| owner_of(class, &keys));
        let owner = owner.flatten();
        element
            .child_keys()
            .into_iter()
            .filter(|key| contains_ignore_case(key, part))
            .map(|key| {
                let field = owner.and_then(|class| schema::xml_fields(class).find(|field| field.name == key));
                let documentation = field.map_or_else(String::new, schema::type_label);
                item(key.clone().into_owned(), "ƒ", key.into_owned(), documentation)
            })
            .collect()
    }

    /// The hover of a path. `None`: it is not over one, for [`NemesisXml`] to say.
    fn path_hover(&self, at: &PathAt<'_>) -> Option<String> {
        let (id, keys) = split_path(at.up_to_part());
        let keys: Vec<&str> = keys.collect();
        let Some(text) = self.behavior().text(id) else { return Some(format!("No object has the id `{id}`.")) };
        let object = parse(text)?;
        let class = object.attribute("class")?;
        let Some(element) = resolve(&object, keys.iter().copied()) else {
            return Some(format!("`{id}` ⟦class|{class}⟧ has nothing at `{}`.", at.up_to_part()));
        };
        let Some(key) = keys.last() else { return Some(format!("**{id}**: ⟦class|{class}⟧")) };
        let mut doc = key_hover(class, &keys, key);
        let value = element.words.join(" ");
        if !value.is_empty() {
            doc.push_str("\n\n```\n");
            doc.push_str(&value);
            doc.push_str("\n```");
        }
        Some(doc)
    }

    /// Where `at` leads to.
    fn path_location(&self, at: &PathAt<'_>) -> Option<Location<'_>> {
        let (id, keys) = split_path(at.up_to_part());
        let object_id = self.behavior().texts().find(|object| object.id == id)?;
        let object = parse(object_id.text)?;
        let line = resolve(&object, keys).and_then(|element| line_of_element(object_id.text, element)).unwrap_or(0);
        Some(Location { document: object_id.id.into(), line, detail: object_id.class.into() })
    }
}

/// The class that holds what `keys` lead to: an `Element` is of the class of its list.
///
/// e.g. `hkbStateMachine`, `transitions`, `Element0` -> `hkbStateMachineTransitionInfo`
fn owner_of(class: &str, keys: &[&str]) -> Option<&'static str> {
    let mut owner = schema::class_names().into_iter().find(|name| *name == class)?;
    for key in keys.iter().filter(|key| !is_element(key)) {
        let field = schema::xml_fields(owner).find(|field| field.name == *key)?;
        owner = field.class?;
    }
    Some(owner)
}

fn is_element(key: &str) -> bool {
    key.strip_prefix("Element").is_some_and(|index| index.parse::<usize>().is_ok())
}

/// e.g. `⟦class|hkbClipGenerator⟧.**flags**: `Flags(Int8)``, or for an element of a list
/// `Element 3 of **transitions**`.
fn key_hover(class: &str, keys: &[&str], key: &str) -> String {
    let parent = &keys[..keys.len() - 1];
    if is_element(key) {
        let list = parent.iter().rev().find(|key| !is_element(key)).copied().unwrap_or_default();
        return format!("Element {} of **{list}**", &key["Element".len()..]);
    }
    let field = owner_of(class, parent).and_then(|owner| {
        let field = schema::xml_fields(owner).find(|field| field.name == key)?;
        Some((owner, field))
    });
    field.map_or_else(|| format!("**{key}**"), |(owner, field)| field_doc(owner, field))
}

fn field_doc(owner: &str, field: &FieldSchema) -> String {
    let target = field.class.map_or_else(String::new, |target| format!(" → ⟦class|{target}⟧"));
    format!("⟦class|{owner}⟧.**{}**: `{}`{target}", field.name, schema::type_label(field))
}

/// The operations, on a line with only what is being typed of a tag. e.g. `  <Re`
fn complete_operation(line: &str, column: usize) -> Option<CompletionList> {
    let at = line.char_indices().nth(column).map_or(line.len(), |(at, _)| at);
    let typed = line[..at].trim_start().strip_prefix('<')?;
    if !typed.chars().all(char::is_alphanumeric) {
        return None;
    }
    let start = line[..at].chars().count() - typed.chars().count() - 1;
    let items = OPERATIONS
        .iter()
        .filter(|(name, ..)| contains_ignore_case(name, typed))
        .map(|&(name, insert, documentation)| {
            let mut item = item(name, "◆", insert, documentation);
            let inside_quotes = insert.find("\"\"").map_or(0, |at| at + 1);
            item.select = Some(inside_quotes..inside_quotes);
            item
        })
        .collect();
    Some(CompletionList { replace: start..column, items })
}

impl Language for PandoraXml<'_> {
    fn comment(&self) -> Option<egui_code_editor::Comment> {
        Some(egui_code_editor::Comment::Block("<!--", "-->"))
    }

    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(Range<usize>, Color32)> {
        self.inner.highlight(line, visuals)
    }

    fn complete(&self, line: &str, column: usize) -> CompletionList {
        if let Some(at) = path_at(line, column) {
            let part = at.typed.rsplit('/').next().unwrap_or_default();
            let items = at.parent().map_or_else(|| self.complete_id(part), |parent| self.complete_key(parent, part));
            return CompletionList { replace: at.part_columns, items };
        }
        let operations = complete_operation(line, column);
        let mut list = self.inner.complete(line, column);
        if let Some(operations) = operations {
            if list.items.is_empty() {
                return operations;
            }
            list.replace = operations.replace;
            list.items.splice(0..0, operations.items);
        }
        list
    }

    fn completion_triggers(&self) -> &[char] {
        &['<', '"', '#', '/', '$', '[', '|']
    }

    fn hover_at(&self, text: &str, line_no: usize, column: usize) -> Option<String> {
        let line = text.split('\n').nth(line_no)?;
        match path_at(line, column) {
            Some(at) if !at.path.is_empty() => self.path_hover(&at),
            _ => self.inner.hover_at(text, line_no, column),
        }
    }

    fn definition(&self, text: &str, line_no: usize, column: usize) -> Option<Location<'_>> {
        let line = text.split('\n').nth(line_no)?;
        match path_at(line, column) {
            Some(at) if !at.path.is_empty() => self.path_location(&at),
            _ => self.inner.definition(text, line_no, column),
        }
    }

    fn references(&self, text: &str, line: usize, column: usize) -> Option<References<'_>> {
        self.inner.references(text, line, column)
    }

    fn indent(&self) -> egui_code_editor::Indent {
        self.inner.indent()
    }
}

/// The class and fields of what the lines around `cursor_line` of a patch file write, for the
/// completion of `<hkparam name="`: what the closest operation above the cursor points to.
///
/// e.g. in `<Replace path="#0106/enable">`, the fields of the class of `#0106`; in
/// `<Append path="#0106/transitions">`, those of the elements of `transitions`.
pub(in crate::patch) fn fields_at(
    behavior: &Behavior,
    doc: &str,
    cursor_line: usize,
) -> Option<(&'static str, Vec<&'static FieldSchema>)> {
    let (line, path) = doc
        .lines()
        .take(cursor_line + 1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .find_map(|line| path_of_line(line).filter(|path| !path.is_empty()).map(|path| (line, path)))?;
    let (id, keys) = split_path(path);
    let text = behavior.text(id)?;
    let class = parse(text)?.attribute("class")?.to_owned();
    let keys: Vec<&str> = keys.collect();
    // A `Replace` writes the field itself: its parent has the fields. The others write into it.
    let of = if line.trim_start().starts_with("<Replace") { &keys[..keys.len().saturating_sub(1)] } else { &keys[..] };
    let owner = owner_of(&class, of)?;
    Some((owner, schema::xml_fields(owner).collect()))
}

/// The editor's underlines for a patch file.
pub(in crate::patch) fn diagnostics(doc: &str, behavior: &Behavior) -> Vec<Diagnostic> {
    doc_problems(doc, |id| behavior.text(id))
        .into_iter()
        .map(|problem| Diagnostic {
            line: problem.line,
            columns: problem.columns,
            severity: Severity::Warning,
            message: problem.kind.to_string(),
        })
        .collect()
}

/// A comment for the head of a new patch file: what the work has changed that it can't say, so that it
/// isn't thought to be in it. Nothing if there is nothing to tell.
pub(in crate::patch) fn draft_comment(problems: &[String]) -> Cow<'static, str> {
    if problems.is_empty() {
        return Cow::Borrowed("");
    }
    let mut comment = String::from("<!-- Not written, as Pandora has no operation for them:\n");
    for problem in problems {
        comment.push_str(&problem.replace("--", "- -"));
        comment.push('\n');
    }
    comment.push_str("-->\n");
    Cow::Owned(comment)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::patch::{tree::Graph, workspace::Behavior};

    /// A template of a state machine with a list of transitions, kept until the end of the test.
    fn behavior() -> (Behavior, PathBuf) {
        // Both: `cargo test` has its tests in threads of one process, nextest each in a process of its own,
        // where the thread id is the same in all of them.
        let unique = format!("{}_{:?}", std::process::id(), std::thread::current().id());
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_language_{unique}"));
        let meshes = dir.join("meshes/actors/character/behaviors");
        std::fs::create_dir_all(&meshes).unwrap();
        let path = meshes.join("0_master.xml");
        let template = "<hkpackfile><hksection name=\"__data__\">\n\
<hkobject name=\"#0001\" class=\"hkbStateMachine\" signature=\"0x816c1dcb\">\n\
\t<hkparam name=\"wrapAroundStateId\">false</hkparam>\n\
\t<hkparam name=\"states\" numelements=\"1\">\n\
\t\t#0002\n\
\t</hkparam>\n\
\t<hkparam name=\"list\" numelements=\"2\">\n\
\t\t<hkobject>\n\
\t\t\t<hkparam name=\"inner\">1</hkparam>\n\
\t\t</hkobject>\n\
\t\t<hkobject>\n\
\t\t\t<hkparam name=\"inner\">2</hkparam>\n\
\t\t</hkobject>\n\
\t</hkparam>\n\
</hkobject>\n\
</hksection></hkpackfile>\n";
        std::fs::write(&path, template).unwrap();
        (Behavior::open(&path).unwrap(), dir)
    }

    /// Runs `check` with the language of a patch file of [`behavior`].
    fn with_language(check: impl FnOnce(&PandoraXml<'_>)) {
        let (behavior, dir) = behavior();
        let (graph, values) = (Graph::new(&behavior), Default::default());
        let inner = NemesisXml {
            code_action: false,
            events: &[],
            variables: &[],
            variable_types: &[],
            values: &values,
            graph: &graph,
            fields: None,
            pointer_field: None,
            kept_array_hover: "",
            in_string_array: false,
            behavior: &behavior,
        };
        check(&PandoraXml { inner: &inner });
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn labels(list: &CompletionList) -> Vec<&str> {
        list.items.iter().map(|item| item.label.as_ref()).collect()
    }

    #[test]
    fn a_path_completes_with_what_the_object_has_there() {
        with_language(|language| {
            let at = |line: &str| language.complete(line, line.chars().count());

            let ids = at("<Replace path=\"#00");
            assert_eq!(labels(&ids), ["#0001"]);
            assert_eq!(ids.items[0].insert, "#0001/", "ready for what comes next");
            assert_eq!(ids.replace, 15..18);

            let fields = at("<Replace path=\"#0001/");
            assert_eq!(labels(&fields), ["wrapAroundStateId", "states", "list"]);
            let typed = at("<Replace path=\"#0001/sta");
            assert_eq!(labels(&typed), ["wrapAroundStateId", "states"], "anywhere in the name");
            assert_eq!(typed.replace, 21..24);

            assert_eq!(labels(&at("<Replace path=\"#0001/list/")), ["Element0", "Element1"]);
            assert_eq!(labels(&at("<Replace path=\"#0001/list/Element1/")), ["inner"]);
            assert!(labels(&at("<Replace path=\"#9999/")).is_empty());
        });
    }

    #[test]
    fn a_path_hovers_with_what_it_leads_to() {
        with_language(|language| {
            let hover = |line: &str, on: &str| {
                let column = line[..line.find(on).unwrap()].chars().count() + 1;
                language.hover_at(line, 0, column)
            };
            let line = "<Replace path=\"#0001/list/Element1/inner\">";
            assert!(hover(line, "#0001").unwrap().contains("⟦class|hkbStateMachine⟧"));
            assert_eq!(hover(line, "Element1").unwrap(), "Element 1 of **list**");
            assert!(hover(line, "inner").unwrap().contains("\n```\n2\n```"), "its value");

            let field = hover("<Replace path=\"#0001/wrapAroundStateId\">", "wrap").unwrap();
            assert!(field.contains("**wrapAroundStateId**: `"), "its type: {field}");

            assert!(hover("<Replace path=\"#9999/a\">", "#9999").unwrap().contains("No object"));
            assert!(hover("<Replace path=\"#0001/none\">", "none").unwrap().contains("nothing at"));
        });
    }

    #[test]
    fn a_path_goes_to_the_line_of_what_it_leads_to() {
        with_language(|language| {
            let go = |line: &str, on: &str| {
                let column = line[..line.find(on).unwrap()].chars().count() + 1;
                let location = language.definition(line, 0, column)?;
                Some((location.document.into_owned(), location.line))
            };
            let line = "<Replace path=\"#0001/list/Element1/inner\">";
            assert_eq!(go(line, "#0001"), Some(("#0001".to_owned(), 0)));
            assert_eq!(go(line, "list"), Some(("#0001".to_owned(), 3 + 2)));
            assert_eq!(go(line, "Element1"), Some(("#0001".to_owned(), 9)));
            assert_eq!(go(line, "inner"), Some(("#0001".to_owned(), 10)));
            assert_eq!(go("<Replace path=\"#9999/a\">", "#9999"), None);
        });
    }

    #[test]
    fn what_a_file_writes_completes_with_the_fields_of_its_place() {
        let (behavior, dir) = behavior();
        let fields = |doc: &str, line: usize| {
            fields_at(&behavior, doc, line)
                .map(|(class, fields)| (class, fields.iter().any(|field| field.name == "wrapAroundStateId")))
        };
        let doc = "<m>\n<Replace path=\"#0001/wrapAroundStateId\">\n\t\n</Replace>\n</m>";
        assert_eq!(fields(doc, 2), Some(("hkbStateMachine", true)), "the field is one of the object's");
        assert_eq!(fields("<m>\n\t\n</m>", 1), None, "no operation around it");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_problems_of_a_file_are_underlined() {
        let (behavior, dir) = behavior();
        let problems = diagnostics("<m>\n<Remove path=\"#0001/none\" />\n</m>", &behavior);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].line, 1);
        assert_eq!(problems[0].severity, Severity::Warning);
    }

    #[test]
    fn the_operations_are_offered_on_a_line_with_only_a_tag() {
        let list = complete_operation("  <Re", 5).unwrap();
        let names: Vec<_> = list.items.iter().map(|item| item.label.as_ref()).collect();
        assert_eq!(names, ["Replace", "Remove"]);
        assert_eq!(list.replace, 2..5);
        assert_eq!(list.items[0].insert, "<Replace path=\"\">\n</Replace>");
        // The cursor goes into the path.
        assert_eq!(list.items[0].select, Some(15..15));

        assert_eq!(complete_operation("<hkparam name=\"", 15), None);
        assert_eq!(complete_operation("text", 4), None);
    }

    #[test]
    fn an_element_is_of_the_class_of_its_list() {
        let owner = owner_of("hkbStateMachine", &["wildcardTransitions"]);
        assert!(owner.is_some());
        assert_eq!(owner_of("hkbStateMachine", &["none"]), None);
        assert_eq!(owner_of("noSuchClass", &[]), None);
        assert_eq!(owner_of("hkbStateMachine", &[]), Some("hkbStateMachine"));
    }

    #[test]
    fn a_hover_says_what_the_key_is() {
        assert_eq!(
            key_hover("hkbStateMachine", &["transitions", "Element3"], "Element3"),
            "Element 3 of **transitions**"
        );
        let doc = key_hover("hkbStateMachine", &["wrapAroundStateId"], "wrapAroundStateId");
        assert!(doc.starts_with("⟦class|hkbStateMachine⟧.**wrapAroundStateId**: `"), "{doc}");
        assert_eq!(key_hover("noSuchClass", &["a"], "a"), "**a**");
    }

    #[test]
    fn what_a_draft_can_not_say_stays_in_a_comment() {
        assert_eq!(draft_comment(&[]), "");
        let comment = draft_comment(&["0_master/#0001: x -- y".to_owned()]);
        assert!(comment.starts_with("<!--") && comment.trim_end().ends_with("-->"));
        assert_eq!(comment.matches("--").count(), 2, "{comment}");
    }
}

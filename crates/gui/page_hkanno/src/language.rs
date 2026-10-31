//! Connects `hkanno_lsp` to the generic [`CodeEditor`](egui_code_editor::CodeEditor).
//!
//! This is the only hkanno-specific part of the editor: it translates `hkanno_lsp` results
//! into the editor's language-neutral types (colors, plain-text completions, ...).

use std::ops::Range;

use egui::Color32;
use egui_code_editor::{
    CompletionItem, CompletionList, Diagnostic, Language, QuickFix, Severity, SignatureHelp,
};
use hkanno_lsp::features::{self, completion::CompletionKind, highlight::TokenKind};

/// hkanno for the editor.
pub(super) struct HkannoLanguage {
    /// LSP option "semantic tokens": parser-based coloring on top of the basic one.
    pub(super) semantic: bool,
    /// LSP option "code action": quick fixes (`Ctrl+.`).
    pub(super) code_action: bool,
}

impl Language for HkannoLanguage {
    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(Range<usize>, Color32)> {
        features::highlight::highlight_line(line, self.semantic)
            .into_iter()
            .map(|(span, kind)| (span.start..span.end, token_color(kind, visuals.dark_mode)))
            .collect()
    }

    fn complete(&self, line: &str, column: usize) -> CompletionList {
        let list = features::completion::complete(line, column);
        let items = list
            .items
            .into_iter()
            .map(|item| {
                // The editor inserts plain text: expand `${1:placeholder}` here.
                let expanded = hkanno_lsp::snippet::expand(&item.insert);
                CompletionItem {
                    label: item.label.to_string(),
                    icon: kind_icon(item.kind),
                    insert: expanded.text,
                    select: expanded.first_placeholder,
                    documentation: item.documentation.to_string(),
                }
            })
            .collect();
        CompletionList { replace: list.replace.start..list.replace.end, items }
    }

    fn completion_triggers(&self) -> &[char] {
        &[' ', '.', '@', '$', '!']
    }

    fn hover(&self, line: &str, column: usize) -> Option<String> {
        features::hover::hover(line, column)
    }

    fn signature_help(&self, line: &str, column: usize) -> Option<SignatureHelp> {
        let help = features::signature::signature_help(line, column)?;
        Some(SignatureHelp {
            label: help.label,
            params: help.params,
            active: help.active,
            documentation: help.documentation.map(ToString::to_string),
        })
    }

    fn inlay_hints(&self, line: &str) -> Vec<String> {
        features::inlay_hint::inlay_hints(line).iter().map(|h| h.label.to_string()).collect()
    }

    fn format(&self, text: &str) -> Option<String> {
        Some(features::format::format_document(text))
    }

    fn quick_fixes(&self, text: &str, line: usize, column: usize) -> Vec<QuickFix> {
        if !self.code_action {
            return Vec::new();
        }
        let mut on_line: Vec<_> = features::diagnostics::diagnostics(text)
            .into_iter()
            .filter(|d| d.range.line == line)
            .collect();
        // The problem under the cursor first.
        on_line.sort_by_key(|d| !(d.range.span.start..=d.range.span.end).contains(&column));
        on_line
            .iter()
            .filter_map(|d| features::code_action::code_action(text, d))
            .map(|action| QuickFix {
                title: action.title,
                line: action.edit.range.line,
                columns: action.edit.range.span.start..action.edit.range.span.end,
                new_text: action.edit.new_text,
            })
            .collect()
    }
}

/// `hkanno_lsp` diagnostics -> editor diagnostics.
pub(super) fn to_editor_diagnostics(
    diagnostics: &[features::diagnostics::Diagnostic],
) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .map(|d| Diagnostic {
            line: d.range.line,
            columns: d.range.span.start..d.range.span.end,
            severity: match d.severity {
                features::diagnostics::Severity::Error => Severity::Error,
                features::diagnostics::Severity::Warning => Severity::Warning,
            },
            message: d.message.clone(),
        })
        .collect()
}

/// One Dark / One Light palette (the same colors as the former Monaco theme).
const fn token_color(kind: TokenKind, dark: bool) -> Color32 {
    let (dark_rgb, light_rgb) = match kind {
        TokenKind::Comment => ((127, 132, 142), (160, 161, 167)),
        TokenKind::Number => ((209, 154, 102), (152, 104, 1)),
        TokenKind::String => ((152, 195, 121), (80, 161, 79)),
        TokenKind::Identifier => ((97, 175, 239), (64, 120, 242)),
        TokenKind::Variable => ((224, 108, 117), (228, 86, 73)),
        TokenKind::Function => ((229, 192, 123), (193, 132, 1)),
        TokenKind::Keyword => ((198, 120, 221), (166, 38, 164)),
        TokenKind::Operator => ((86, 182, 194), (1, 132, 188)),
    };
    let (r, g, b) = if dark { dark_rgb } else { light_rgb };
    Color32::from_rgb(r, g, b)
}

/// Small glyph shown before each completion candidate.
const fn kind_icon(kind: CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Keyword => "🔑",
        CompletionKind::Value => "#",
        CompletionKind::Function => "ƒ",
        CompletionKind::Snippet => "✂",
        CompletionKind::EnumMember => "◆",
    }
}

//! Zed: the "Velure" theme, written to `~/.config/zed/themes/velure.json`. Zed reloads
//! it when the file changes.
//!
//! Every colour names a token, so the decisions made in `tokens` reach the editor intact:
//! code kinds use the `syntax_*` tokens, which are kept apart from each other and from text,
//! and reach the APCA contrast below which Zed would lighten them on its own.

use serde_json::{Map, Value, json};

use super::Paint::{self, Fixed, Tok};
use crate::tokens::Tokens;

const STYLE: &[(&str, Paint)] = &[
    ("background", Tok("bg", None)),
    ("border", Tok("border", None)),
    ("border.variant", Tok("surface_high", None)),
    ("border.focused", Tok("primary", None)),
    ("border.selected", Tok("primary", None)),
    ("border.transparent", Fixed("#00000000")),
    ("border.disabled", Tok("surface_high", None)),
    ("elevated_surface.background", Tok("surface", None)),
    ("surface.background", Tok("bg", None)),
    ("element.background", Tok("surface_high", None)),
    ("element.hover", Tok("overlay", None)),
    ("element.active", Tok("overlay", None)),
    ("element.selected", Tok("primary_container", None)),
    ("element.disabled", Tok("surface", None)),
    ("drop_target.background", Tok("primary", Some("33"))),
    ("ghost_element.background", Fixed("#00000000")),
    ("ghost_element.hover", Tok("text", Some("14"))),
    ("ghost_element.active", Tok("text", Some("1f"))),
    ("ghost_element.selected", Tok("primary_container", None)),
    ("ghost_element.disabled", Fixed("#00000000")),
    ("text", Tok("text", None)),
    ("text.muted", Tok("subtext", None)),
    ("text.placeholder", Tok("muted", None)),
    ("text.disabled", Tok("outline", None)),
    ("text.accent", Tok("primary", None)),
    ("icon", Tok("text", None)),
    ("icon.muted", Tok("subtext", None)),
    ("icon.disabled", Tok("outline", None)),
    ("icon.placeholder", Tok("muted", None)),
    ("icon.accent", Tok("primary", None)),
    ("status_bar.background", Tok("bg", None)),
    ("title_bar.background", Tok("bg", None)),
    ("title_bar.inactive_background", Tok("bg", None)),
    ("toolbar.background", Tok("bg", None)),
    ("tab_bar.background", Tok("bg", None)),
    ("tab.inactive_background", Tok("bg", None)),
    ("tab.active_background", Tok("bg", None)),
    ("search.match_background", Tok("primary", Some("40"))),
    ("panel.background", Tok("bg", None)),
    ("panel.focused_border", Tok("primary", None)),
    ("pane.focused_border", Tok("primary", None)),
    ("pane_group.border", Tok("border", None)),
    ("scrollbar.thumb.background", Tok("text", Some("26"))),
    ("scrollbar.thumb.hover_background", Tok("text", Some("40"))),
    ("scrollbar.thumb.border", Fixed("#00000000")),
    ("scrollbar.track.background", Fixed("#00000000")),
    ("scrollbar.track.border", Fixed("#00000000")),
    ("editor.foreground", Tok("text", None)),
    ("editor.background", Tok("bg", None)),
    ("editor.gutter.background", Tok("bg", None)),
    ("editor.subheader.background", Tok("surface", None)),
    ("editor.active_line.background", Tok("text", Some("0d"))),
    (
        "editor.highlighted_line.background",
        Tok("primary", Some("1a")),
    ),
    ("editor.line_number", Tok("outline", None)),
    ("editor.active_line_number", Tok("text", None)),
    ("editor.invisible", Tok("border", None)),
    ("editor.wrap_guide", Tok("border", Some("80"))),
    ("editor.active_wrap_guide", Tok("border", None)),
    ("editor.indent_guide", Tok("border", Some("80"))),
    ("editor.indent_guide_active", Tok("outline", None)),
    (
        "editor.document_highlight.read_background",
        Tok("primary", Some("1f")),
    ),
    (
        "editor.document_highlight.write_background",
        Tok("primary", Some("33")),
    ),
    (
        "editor.document_highlight.bracket_background",
        Tok("primary", Some("26")),
    ),
    ("link_text.hover", Tok("primary", None)),
    ("search.active_match_background", Tok("primary", Some("66"))),
    ("panel.overlay_background", Tok("surface", None)),
    ("panel.indent_guide", Tok("border", Some("80"))),
    ("panel.indent_guide_active", Tok("outline", None)),
    ("panel.indent_guide_hover", Tok("outline", None)),
    ("minimap.thumb.background", Tok("text", Some("1f"))),
    ("minimap.thumb.hover_background", Tok("text", Some("33"))),
    ("minimap.thumb.active_background", Tok("text", Some("40"))),
    ("minimap.thumb.border", Fixed("#00000000")),
    ("version_control.added", Tok("success", None)),
    ("version_control.deleted", Tok("error", None)),
    ("version_control.modified", Tok("info", None)),
    ("version_control.renamed", Tok("tertiary", None)),
    ("version_control.conflict", Tok("warning", None)),
    ("version_control.ignored", Tok("muted", None)),
    ("version_control.word_added", Tok("success", Some("33"))),
    ("version_control.word_deleted", Tok("error", Some("33"))),
    (
        "version_control.conflict_marker.ours",
        Tok("success", Some("26")),
    ),
    (
        "version_control.conflict_marker.theirs",
        Tok("info", Some("26")),
    ),
    ("terminal.background", Tok("bg", None)),
    ("terminal.foreground", Tok("text", None)),
    ("terminal.bright_foreground", Tok("text", None)),
    ("terminal.dim_foreground", Tok("subtext", None)),
    ("terminal.ansi.black", Tok("ansi_0", None)),
    ("terminal.ansi.bright_black", Tok("ansi_8", None)),
    ("terminal.ansi.dim_black", Tok("surface_high", None)),
    ("terminal.ansi.white", Tok("ansi_7", None)),
    ("terminal.ansi.bright_white", Tok("ansi_15", None)),
    ("terminal.ansi.dim_white", Tok("outline", None)),
    ("terminal.ansi.red", Tok("ansi_1", None)),
    ("terminal.ansi.bright_red", Tok("ansi_9", None)),
    ("terminal.ansi.dim_red", Tok("ansi_1", Some("99"))),
    ("terminal.ansi.green", Tok("ansi_2", None)),
    ("terminal.ansi.bright_green", Tok("ansi_10", None)),
    ("terminal.ansi.dim_green", Tok("ansi_2", Some("99"))),
    ("terminal.ansi.yellow", Tok("ansi_3", None)),
    ("terminal.ansi.bright_yellow", Tok("ansi_11", None)),
    ("terminal.ansi.dim_yellow", Tok("ansi_3", Some("99"))),
    ("terminal.ansi.blue", Tok("ansi_4", None)),
    ("terminal.ansi.bright_blue", Tok("ansi_12", None)),
    ("terminal.ansi.dim_blue", Tok("ansi_4", Some("99"))),
    ("terminal.ansi.magenta", Tok("ansi_5", None)),
    ("terminal.ansi.bright_magenta", Tok("ansi_13", None)),
    ("terminal.ansi.dim_magenta", Tok("ansi_5", Some("99"))),
    ("terminal.ansi.cyan", Tok("ansi_6", None)),
    ("terminal.ansi.bright_cyan", Tok("ansi_14", None)),
    ("terminal.ansi.dim_cyan", Tok("ansi_6", Some("99"))),
    ("error", Tok("error", None)),
    ("error.background", Tok("error", Some("1a"))),
    ("error.border", Tok("error", Some("4d"))),
    ("warning", Tok("warning", None)),
    ("warning.background", Tok("warning", Some("1a"))),
    ("warning.border", Tok("warning", Some("4d"))),
    ("info", Tok("info", None)),
    ("info.background", Tok("info", Some("1a"))),
    ("info.border", Tok("info", Some("4d"))),
    ("hint", Tok("muted", None)),
    ("hint.background", Tok("muted", Some("1a"))),
    ("hint.border", Tok("muted", Some("4d"))),
    ("success", Tok("success", None)),
    ("success.background", Tok("success", Some("1a"))),
    ("success.border", Tok("success", Some("4d"))),
    ("created", Tok("success", None)),
    ("created.background", Tok("success", Some("1a"))),
    ("created.border", Tok("success", Some("4d"))),
    ("deleted", Tok("error", None)),
    ("deleted.background", Tok("error", Some("1a"))),
    ("deleted.border", Tok("error", Some("4d"))),
    ("modified", Tok("info", None)),
    ("modified.background", Tok("info", Some("1a"))),
    ("modified.border", Tok("info", Some("4d"))),
    ("conflict", Tok("warning", None)),
    ("conflict.background", Tok("warning", Some("1a"))),
    ("conflict.border", Tok("warning", Some("4d"))),
    ("renamed", Tok("tertiary", None)),
    ("renamed.background", Tok("tertiary", Some("1a"))),
    ("renamed.border", Tok("tertiary", Some("4d"))),
    ("predictive", Tok("muted", None)),
    ("predictive.background", Tok("muted", Some("1a"))),
    ("predictive.border", Tok("muted", Some("4d"))),
    ("ignored", Tok("muted", None)),
    ("ignored.background", Tok("muted", Some("1a"))),
    ("ignored.border", Tok("muted", Some("4d"))),
    ("hidden", Tok("muted", None)),
    ("hidden.background", Tok("muted", Some("1a"))),
    ("hidden.border", Tok("muted", Some("4d"))),
    ("unreachable", Tok("muted", None)),
    ("unreachable.background", Tok("muted", Some("1a"))),
    ("unreachable.border", Tok("muted", Some("4d"))),
];

const PLAYERS: &[[(&str, Paint); 3]] = &[
    [
        ("cursor", Tok("primary", None)),
        ("background", Tok("primary", None)),
        ("selection", Tok("primary", Some("3d"))),
    ],
    [
        ("cursor", Tok("secondary", None)),
        ("background", Tok("secondary", None)),
        ("selection", Tok("secondary", Some("3d"))),
    ],
    [
        ("cursor", Tok("tertiary", None)),
        ("background", Tok("tertiary", None)),
        ("selection", Tok("tertiary", Some("3d"))),
    ],
    [
        ("cursor", Tok("syntax_string", None)),
        ("background", Tok("syntax_string", None)),
        ("selection", Tok("syntax_string", Some("3d"))),
    ],
];

/// Bracket pairs and indent guides when Zed colours them (`colorize_brackets`): without
/// them it would use its own palette. The code colours and not the accents, which can fall
/// short of the contrast at which Zed would change them.
const ACCENTS: &[Paint] = &[
    Tok("syntax_keyword", None),
    Tok("syntax_function", None),
    Tok("syntax_type", None),
    Tok("syntax_string", None),
    Tok("syntax_number", None),
    Tok("syntax_property", None),
];

/// Syntax kind, colour, font style and font weight.
const SYNTAX: &[(&str, Paint, Option<&str>, Option<u16>)] = &[
    ("attribute", Tok("syntax_keyword", None), None, None),
    ("boolean", Tok("syntax_number", None), None, None),
    ("comment", Tok("syntax_comment", None), Some("italic"), None),
    ("comment.doc", Tok("subtext", None), Some("italic"), None),
    ("constant", Tok("syntax_number", None), None, None),
    ("constructor", Tok("syntax_type", None), None, None),
    ("embedded", Tok("text", None), None, None),
    (
        "emphasis",
        Tok("syntax_keyword", None),
        Some("italic"),
        None,
    ),
    (
        "emphasis.strong",
        Tok("syntax_keyword", None),
        None,
        Some(700),
    ),
    ("enum", Tok("syntax_type", None), None, None),
    ("function", Tok("syntax_function", None), None, None),
    ("hint", Tok("syntax_comment", None), None, None),
    ("keyword", Tok("syntax_keyword", None), None, None),
    ("label", Tok("syntax_keyword", None), None, None),
    (
        "link_text",
        Tok("syntax_keyword", None),
        Some("italic"),
        None,
    ),
    ("link_uri", Tok("emphasis", None), None, None),
    ("number", Tok("syntax_number", None), None, None),
    ("operator", Tok("subtext", None), None, None),
    (
        "predictive",
        Tok("syntax_comment", None),
        Some("italic"),
        None,
    ),
    ("preproc", Tok("syntax_keyword", None), None, None),
    ("primary", Tok("text", None), None, None),
    ("property", Tok("syntax_property", None), None, None),
    ("punctuation", Tok("subtext", None), None, None),
    ("punctuation.bracket", Tok("subtext", None), None, None),
    ("punctuation.delimiter", Tok("subtext", None), None, None),
    (
        "punctuation.list_marker",
        Tok("syntax_keyword", None),
        None,
        None,
    ),
    (
        "punctuation.special",
        Tok("syntax_keyword", None),
        None,
        None,
    ),
    ("string", Tok("syntax_string", None), None, None),
    ("string.escape", Tok("emphasis", None), None, None),
    ("string.regex", Tok("emphasis", None), None, None),
    ("string.special", Tok("emphasis", None), None, None),
    ("string.special.symbol", Tok("emphasis", None), None, None),
    ("tag", Tok("syntax_property", None), None, None),
    ("text.literal", Tok("syntax_string", None), None, None),
    ("title", Tok("syntax_keyword", None), None, Some(700)),
    ("type", Tok("syntax_type", None), None, None),
    ("variable", Tok("text", None), None, None),
    ("variable.special", Tok("syntax_keyword", None), None, None),
    ("variant", Tok("syntax_type", None), None, None),
];

pub fn render(tokens: &Tokens) -> String {
    let paint = |p: &Paint| json!(p.resolve(tokens));
    let mut style: Map<String, Value> = STYLE
        .iter()
        .map(|(key, p)| ((*key).to_string(), paint(p)))
        .collect();
    let players: Vec<Value> = PLAYERS
        .iter()
        .map(|player| {
            Value::Object(
                player
                    .iter()
                    .map(|(key, p)| ((*key).to_string(), paint(p)))
                    .collect(),
            )
        })
        .collect();
    style.insert("players".into(), Value::Array(players));
    style.insert(
        "accents".into(),
        Value::Array(ACCENTS.iter().map(paint).collect()),
    );
    let syntax: Map<String, Value> = SYNTAX
        .iter()
        .map(|(kind, p, font_style, font_weight)| {
            (
                (*kind).to_string(),
                json!({ "color": paint(p), "font_style": font_style, "font_weight": font_weight }),
            )
        })
        .collect();
    style.insert("syntax".into(), Value::Object(syntax));
    let theme = json!({
        "$schema": "https://zed.dev/schema/themes/v0.2.0.json",
        "name": "Velure",
        "author": "velure",
        "themes": [{ "name": "Velure", "appearance": "dark", "style": style }],
    });
    let mut out = serde_json::to_string_pretty(&theme).expect("JSON de Zed");
    out.push('\n');
    out
}

/// Every token the theme reads, once per use.
pub fn usage() -> Vec<&'static str> {
    STYLE
        .iter()
        .map(|(_, p)| p)
        .chain(PLAYERS.iter().flatten().map(|(_, p)| p))
        .chain(ACCENTS)
        .chain(SYNTAX.iter().map(|(_, p, _, _)| p))
        .filter_map(Paint::token)
        .collect()
}

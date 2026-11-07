//! fish: `~/.config/fish/velure-colors.fish`, the colours of what is typed, completed and
//! selected. `config.fish` sources it and sources it again on SIGUSR1.
//!
//! Highlighting is code, so it uses the `syntax_*` tokens: they stay apart from each
//! other and from text even when the accents are nearly grey.

use super::Paint::{self, Tok};
use crate::tokens::Tokens;

/// Variable, colour, background and extra flags.
type Setting = (&'static str, Option<Paint>, Option<Paint>, &'static str);

const fn fg(name: &'static str, token: &'static str) -> Setting {
    (name, Some(Tok(token, None)), None, "")
}

const SECTIONS: &[(&str, &[Setting])] = &[
    (
        "Lo que se escribe",
        &[
            fg("fish_color_normal", "text"),
            fg("fish_color_command", "syntax_function"),
            fg("fish_color_keyword", "syntax_keyword"),
            fg("fish_color_param", "text"),
            fg("fish_color_option", "text"),
            fg("fish_color_quote", "syntax_string"),
            fg("fish_color_operator", "syntax_type"),
            fg("fish_color_redirection", "syntax_type"),
            fg("fish_color_escape", "syntax_number"),
            fg("fish_color_end", "syntax_type"),
            fg("fish_color_error", "error"),
            fg("fish_color_comment", "muted"),
            fg("fish_color_autosuggestion", "muted"),
            ("fish_color_valid_path", None, None, "--underline"),
            fg("fish_color_cancel", "error"),
            ("fish_color_history_current", None, None, "--bold"),
        ],
    ),
    (
        "Selección y coincidencias de la búsqueda: el primario en profundo",
        &[
            (
                "fish_color_selection",
                Some(Tok("on_primary_container", None)),
                Some(Tok("primary_container", None)),
                "",
            ),
            (
                "fish_color_search_match",
                Some(Tok("on_primary_container", None)),
                Some(Tok("primary_container", None)),
                "",
            ),
        ],
    ),
    (
        "Prompt de fish (sin Starship) y estado",
        &[
            fg("fish_color_cwd", "primary"),
            fg("fish_color_cwd_root", "error"),
            fg("fish_color_user", "subtext"),
            fg("fish_color_host", "subtext"),
            fg("fish_color_host_remote", "warning"),
            fg("fish_color_status", "error"),
        ],
    ),
    (
        "Menú de completado (Tab)",
        &[
            (
                "fish_pager_color_prefix",
                Some(Tok("primary", None)),
                None,
                "--bold",
            ),
            fg("fish_pager_color_completion", "text"),
            fg("fish_pager_color_description", "subtext"),
            (
                "fish_pager_color_progress",
                Some(Tok("on_primary_container", None)),
                Some(Tok("primary_container", None)),
                "",
            ),
            (
                "fish_pager_color_selected_background",
                None,
                Some(Tok("primary_container", None)),
                "",
            ),
            (
                "fish_pager_color_selected_prefix",
                Some(Tok("on_primary_container", None)),
                None,
                "--bold",
            ),
            fg(
                "fish_pager_color_selected_completion",
                "on_primary_container",
            ),
            fg(
                "fish_pager_color_selected_description",
                "on_primary_container",
            ),
        ],
    ),
];

pub fn render(tokens: &Tokens) -> String {
    let mut out = String::from(
        "# Generado por velure desde el fondo de pantalla. No editar a mano: se\n\
         # sobrescribe cada vez que cambia el fondo.\n",
    );
    for (comment, settings) in SECTIONS {
        out.push_str(&format!("\n# {comment}\n"));
        for (name, color, background, flags) in *settings {
            let mut line = format!("set -g {name}");
            if let Some(c) = color {
                line.push_str(&format!(" '{}'", c.resolve(tokens)));
            }
            if let Some(b) = background {
                line.push_str(&format!(" --background='{}'", b.resolve(tokens)));
            }
            if !flags.is_empty() {
                line.push(' ');
                line.push_str(flags);
            }
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

/// Every token the theme reads, once per use.
pub fn usage() -> Vec<&'static str> {
    SECTIONS
        .iter()
        .flat_map(|(_, settings)| settings.iter())
        .flat_map(|(_, c, b, _)| [c, b])
        .flatten()
        .filter_map(Paint::token)
        .collect()
}

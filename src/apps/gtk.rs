//! GTK: the colours of the "Velure" GTK3 theme (`colors.css`, imported by the theme on top
//! of adw-gtk3-dark) and of libadwaita apps (`~/.config/gtk-4.0/gtk.css`).
//!
//! Windows and lists sit on the shell's background, cards, menus and dialogs one step up
//! like the shell's panels, and each state fill carries its own text colour.

use super::Paint::{self, Fixed, Tok};
use crate::tokens::Tokens;

/// Named colours, in the order and groups of the file.
const GROUPS: &[(&str, &[(&str, Paint)])] = &[
    (
        "Acento",
        &[
            ("accent_color", Tok("primary", None)),
            ("accent_bg_color", Tok("primary", None)),
            ("accent_fg_color", Tok("on_primary", None)),
        ],
    ),
    (
        "Estados, cada uno con su texto",
        &[
            ("destructive_color", Tok("error", None)),
            ("destructive_bg_color", Tok("error", None)),
            ("destructive_fg_color", Tok("on_error", None)),
            ("success_color", Tok("success", None)),
            ("success_bg_color", Tok("success", None)),
            ("success_fg_color", Tok("on_success", None)),
            ("warning_color", Tok("warning", None)),
            ("warning_bg_color", Tok("warning", None)),
            ("warning_fg_color", Tok("on_warning", None)),
            ("error_color", Tok("error", None)),
            ("error_bg_color", Tok("error", None)),
            ("error_fg_color", Tok("on_error", None)),
        ],
    ),
    (
        "Ventana y contenido (la lista de archivos): el fondo de la shell",
        &[
            ("window_bg_color", Tok("bg", None)),
            ("window_fg_color", Tok("text", None)),
            ("view_bg_color", Tok("bg", None)),
            ("view_fg_color", Tok("text", None)),
        ],
    ),
    (
        "Barra de título",
        &[
            ("headerbar_bg_color", Tok("bg", None)),
            ("headerbar_fg_color", Tok("text", None)),
            ("headerbar_border_color", Tok("border", None)),
            ("headerbar_backdrop_color", Tok("bg", None)),
            ("headerbar_shade_color", Fixed("rgba(0, 0, 0, 0.36)")),
            ("headerbar_darker_shade_color", Fixed("rgba(0, 0, 0, 0.9)")),
        ],
    ),
    (
        "Barra lateral: como la ventana, separada por un borde",
        &[
            ("sidebar_bg_color", Tok("bg", None)),
            ("sidebar_fg_color", Tok("text", None)),
            ("sidebar_backdrop_color", Tok("bg", None)),
            ("sidebar_border_color", Tok("border", None)),
            ("sidebar_shade_color", Fixed("rgba(0, 0, 0, 0.25)")),
        ],
    ),
    (
        "Tarjetas, diálogos y menús: como los paneles de la shell",
        &[
            ("card_bg_color", Tok("surface", None)),
            ("card_fg_color", Tok("text", None)),
            ("card_shade_color", Fixed("rgba(0, 0, 0, 0.36)")),
            ("dialog_bg_color", Tok("surface", None)),
            ("dialog_fg_color", Tok("text", None)),
            ("popover_bg_color", Tok("surface", None)),
            ("popover_fg_color", Tok("text", None)),
            ("popover_shade_color", Fixed("rgba(0, 0, 0, 0.25)")),
            ("thumbnail_bg_color", Tok("surface", None)),
            ("thumbnail_fg_color", Tok("text", None)),
        ],
    ),
    (
        "Varios",
        &[
            ("shade_color", Fixed("rgba(0, 0, 0, 0.36)")),
            ("scrollbar_outline_color", Fixed("rgba(0, 0, 0, 0.5)")),
            ("borders", Tok("border", None)),
        ],
    ),
];

/// GTK3 and old libadwaita read `@define-color`; libadwaita 1.6 and later read CSS
/// variables, so the GTK4 file carries both.
pub fn render(tokens: &Tokens, variables: bool) -> String {
    let mut out = String::from(
        "/* Generado por velure desde el fondo de pantalla. No editar a mano: se\n   \
         sobrescribe cada vez que cambia el fondo. */\n",
    );
    for (comment, colors) in GROUPS {
        out.push_str(&format!("\n/* {comment} */\n"));
        for (name, paint) in *colors {
            out.push_str(&format!(
                "@define-color {name} {};\n",
                paint.resolve(tokens)
            ));
        }
    }
    if variables {
        out.push_str("\n/* libadwaita 1.6 y posteriores */\n:root {\n");
        for (name, paint) in GROUPS.iter().flat_map(|(_, colors)| colors.iter()) {
            let variable = if *name == "borders" {
                "border-color".to_string()
            } else {
                name.replace('_', "-")
            };
            out.push_str(&format!("  --{variable}: {};\n", paint.resolve(tokens)));
        }
        out.push_str("}\n");
    }
    out
}

/// Every token the theme reads, once per use.
pub fn usage() -> Vec<&'static str> {
    GROUPS
        .iter()
        .flat_map(|(_, colors)| colors.iter())
        .filter_map(|(_, p)| p.token())
        .collect()
}

/// `(foreground, background)` pairs that GTK draws one on the other.
pub const READABLE_PAIRS: &[(&str, &str)] = &[
    ("on_primary", "primary"),
    ("on_error", "error"),
    ("on_success", "success"),
    ("on_warning", "warning"),
    ("text", "bg"),
    ("text", "surface"),
    ("primary", "bg"),
];

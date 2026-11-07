//! Starship: the `velure` palette, written to `~/.config/starship/palette-velure.toml`.
//! Starship has no includes, so a hook joins it with the user's `starship.toml`; the
//! prompt refers to these names (`primary`, `ok`…) instead of colours.

use super::Paint::{self, Tok};
use crate::tokens::Tokens;

/// Palette name, token and what the prompt uses it for.
const PALETTE: &[(&str, Paint, &str)] = &[
    (
        "primary",
        Tok("primary", None),
        "Carpeta actual, rama de git, lenguajes",
    ),
    // `syntax_type` and not `tertiary`: on sober themes the tertiary can be as grey as
    // the muted text right next to it.
    (
        "accent",
        Tok("syntax_type", None),
        "Estado de git y otros detalles",
    ),
    ("accent_alt", Tok("secondary", None), "El segundo acento"),
    ("ok", Tok("success", None), "Estados con significado"),
    ("warn", Tok("warning", None), ""),
    ("err", Tok("error", None), ""),
    (
        "muted",
        Tok("muted", None),
        "Lo secundario (duración, tareas en segundo plano)",
    ),
];

pub fn render(tokens: &Tokens) -> String {
    let mut out = String::from(
        "# Generado por velure desde el fondo de pantalla. No editar a mano: se\n\
         # sobrescribe cada vez que cambia el fondo.\n[palettes.velure]\n",
    );
    for (name, paint, comment) in PALETTE {
        if !comment.is_empty() {
            out.push_str(&format!("# {comment}\n"));
        }
        out.push_str(&format!("{name} = \"{}\"\n", paint.resolve(tokens)));
    }
    out
}

/// Every token the theme reads, once per use.
pub fn usage() -> Vec<&'static str> {
    PALETTE.iter().filter_map(|(_, p, _)| p.token()).collect()
}

//! kitty: `~/.config/kitty/velure.conf`, loaded with `globinclude velure.conf` from
//! `kitty.conf`. Open windows reload it on SIGUSR1.

use super::Paint::{self, Tok};
use crate::tokens::Tokens;

/// Groups of settings, each with the comment that explains it.
const SECTIONS: &[(&str, &[(&str, Paint)])] = &[
    (
        "Fondo y texto: los de la barra y los paneles",
        &[
            ("foreground", Tok("text", None)),
            ("background", Tok("bg", None)),
        ],
    ),
    (
        "Selección: el primario en profundo, como lo seleccionado en la shell",
        &[
            ("selection_foreground", Tok("on_primary_container", None)),
            ("selection_background", Tok("primary_container", None)),
        ],
    ),
    (
        "Cursor y enlaces: los acentos",
        &[
            ("cursor", Tok("primary", None)),
            ("cursor_text_color", Tok("on_primary", None)),
            ("url_color", Tok("secondary", None)),
        ],
    ),
    (
        "Bordes de ventanas (splits): el activo en el primario, el resto discreto",
        &[
            ("active_border_color", Tok("primary", None)),
            ("inactive_border_color", Tok("border", None)),
            ("bell_border_color", Tok("warning", None)),
        ],
    ),
    (
        "Pestañas: la activa en el primario en profundo, las demás sobre el fondo",
        &[
            ("tab_bar_background", Tok("bg", None)),
            ("active_tab_foreground", Tok("on_primary_container", None)),
            ("active_tab_background", Tok("primary_container", None)),
            ("inactive_tab_foreground", Tok("subtext", None)),
            ("inactive_tab_background", Tok("surface", None)),
        ],
    ),
    (
        "Marcas (kitty +kitten hints, búsquedas)",
        &[
            ("mark1_foreground", Tok("on_primary", None)),
            ("mark1_background", Tok("primary", None)),
            ("mark2_foreground", Tok("on_primary", None)),
            ("mark2_background", Tok("secondary", None)),
            ("mark3_foreground", Tok("on_primary", None)),
            ("mark3_background", Tok("tertiary", None)),
        ],
    ),
];

/// The 16 terminal colours keep their meaning: red is red, magenta is magenta.
const ANSI_NAMES: [&str; 8] = [
    "negro", "rojo", "verde", "amarillo", "azul", "magenta", "cian", "blanco",
];

pub fn render(tokens: &Tokens) -> String {
    let mut out = String::from(
        "# Generado por velure desde el fondo de pantalla. No editar a mano: se\n\
         # sobrescribe cada vez que cambia el fondo.\n",
    );
    for (comment, settings) in SECTIONS {
        out.push_str(&format!("\n# {comment}\n"));
        for (key, paint) in *settings {
            out.push_str(&format!("{key:<24}{}\n", paint.resolve(tokens)));
        }
    }
    out.push_str(
        "\n# Los 16 colores: cada uno conserva su significado, con el croma contenido por\n\
         # velure. El brillante es el mismo tono algo más claro.\n",
    );
    for (i, name) in ANSI_NAMES.iter().enumerate() {
        out.push_str(&format!("# {name}\n"));
        for n in [i, i + 8] {
            let color = Tok(ANSI[n], None).resolve(tokens);
            out.push_str(&format!("{:<8}{color}\n", format!("color{n}")));
        }
    }
    out
}

const ANSI: [&str; 16] = [
    "ansi_0", "ansi_1", "ansi_2", "ansi_3", "ansi_4", "ansi_5", "ansi_6", "ansi_7", "ansi_8",
    "ansi_9", "ansi_10", "ansi_11", "ansi_12", "ansi_13", "ansi_14", "ansi_15",
];

/// Every token the theme reads, once per use.
pub fn usage() -> Vec<&'static str> {
    SECTIONS
        .iter()
        .flat_map(|(_, settings)| settings.iter().map(|(_, p)| p))
        .filter_map(Paint::token)
        .chain(ANSI)
        .collect()
}

/// `(foreground, background)` pairs that kitty draws one on the other.
pub const READABLE_PAIRS: &[(&str, &str)] = &[
    ("text", "bg"),
    ("on_primary_container", "primary_container"),
    ("on_primary", "primary"),
    ("subtext", "surface"),
    ("on_primary", "secondary"),
    ("on_primary", "tertiary"),
];

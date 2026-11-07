//! eza: `~/.config/eza/theme.yml`, the colours of `ls`. Icons take the colour of their
//! file kind.
//!
//! Colour is kept for what means something. A long listing repeats permissions on every
//! row, so painting each bit would turn it into noise: permissions stay neutral except a
//! file anyone can write and setuid bits. File kinds use the `syntax_*` tokens, which stay
//! apart from each other and from text.

use super::Paint::Tok;
use crate::tokens::Tokens;

/// Key, token and style flags (`is_bold`, `is_underline`).
type Entry = (&'static str, &'static str, &'static str);

const SECTIONS: &[(&str, &[Entry])] = &[
    (
        "filekinds",
        &[
            ("normal", "text", ""),
            ("directory", "primary", "is_bold"),
            ("symlink", "syntax_type", ""),
            ("pipe", "muted", ""),
            ("block_device", "syntax_number", ""),
            ("char_device", "syntax_number", ""),
            ("socket", "muted", ""),
            ("special", "syntax_type", ""),
            ("executable", "success", "is_bold"),
            ("mount_point", "secondary", ""),
        ],
    ),
    (
        "file_type",
        &[
            ("image", "syntax_property", ""),
            ("video", "syntax_property", ""),
            ("music", "syntax_string", ""),
            ("lossless", "syntax_string", ""),
            ("crypto", "warning", ""),
            ("document", "syntax_number", ""),
            ("compressed", "syntax_function", ""),
            ("temp", "muted", ""),
            ("compiled", "muted", ""),
            ("build", "syntax_keyword", ""),
            ("source", "text", ""),
        ],
    ),
    (
        "perms",
        &[
            ("user_read", "subtext", ""),
            ("user_write", "subtext", ""),
            ("user_execute_file", "subtext", ""),
            ("user_execute_other", "subtext", ""),
            ("group_read", "muted", ""),
            ("group_write", "muted", ""),
            ("group_execute", "muted", ""),
            ("other_read", "muted", ""),
            ("other_write", "warning", ""),
            ("other_execute", "muted", ""),
            ("special_user_file", "warning", ""),
            ("special_other", "warning", ""),
            ("attribute", "muted", ""),
        ],
    ),
    (
        "size",
        &[
            ("major", "subtext", ""),
            ("minor", "subtext", ""),
            ("number_byte", "subtext", ""),
            ("number_kilo", "subtext", ""),
            ("number_mega", "text", ""),
            ("number_giga", "primary", ""),
            ("number_huge", "warning", ""),
            ("unit_byte", "muted", ""),
            ("unit_kilo", "muted", ""),
            ("unit_mega", "muted", ""),
            ("unit_giga", "muted", ""),
            ("unit_huge", "muted", ""),
        ],
    ),
    (
        "users",
        &[
            ("user_you", "subtext", ""),
            ("user_root", "warning", ""),
            ("user_other", "secondary", ""),
            ("group_yours", "subtext", ""),
            ("group_other", "secondary", ""),
            ("group_root", "warning", ""),
        ],
    ),
    (
        "links",
        &[("normal", "muted", ""), ("multi_link_file", "warning", "")],
    ),
    (
        "git",
        &[
            ("new", "success", ""),
            ("modified", "warning", ""),
            ("deleted", "error", ""),
            ("renamed", "info", ""),
            ("typechange", "info", ""),
            ("ignored", "muted", ""),
            ("conflicted", "error", ""),
        ],
    ),
    (
        "git_repo",
        &[
            ("branch_main", "primary", ""),
            ("branch_other", "syntax_type", ""),
            ("git_clean", "success", ""),
            ("git_dirty", "warning", ""),
        ],
    ),
    (
        "",
        &[
            ("punctuation", "muted", ""),
            ("date", "subtext", ""),
            ("inode", "muted", ""),
            ("blocks", "muted", ""),
            ("header", "text", "is_underline"),
            ("octal", "muted", ""),
            ("flags", "muted", ""),
            ("symlink_path", "syntax_type", ""),
            ("control_char", "error", ""),
            ("broken_symlink", "error", ""),
            ("broken_path_overlay", "error", "is_underline"),
        ],
    ),
];

fn style(tokens: &Tokens, token: &'static str, flags: &str) -> String {
    let color = Tok(token, None).resolve(tokens);
    if flags.is_empty() {
        format!("{{foreground: \"{color}\"}}")
    } else {
        format!("{{foreground: \"{color}\", {flags}: true}}")
    }
}

pub fn render(tokens: &Tokens) -> String {
    let mut out = String::from(
        "# Generado por velure desde el fondo de pantalla. No editar a mano: se\n\
         # sobrescribe cada vez que cambia el fondo.\ncolourful: true\n",
    );
    for (section, entries) in SECTIONS {
        out.push('\n');
        let indent = if section.is_empty() {
            ""
        } else {
            out.push_str(&format!("{section}:\n"));
            "  "
        };
        for (key, token, flags) in *entries {
            out.push_str(&format!("{indent}{key}: {}\n", style(tokens, token, flags)));
        }
    }
    out
}

/// Every token the theme reads, once per use.
pub fn usage() -> Vec<&'static str> {
    SECTIONS
        .iter()
        .flat_map(|(_, entries)| entries.iter().map(|(_, token, _)| *token))
        .collect()
}

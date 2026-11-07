//! Writers for each ricing target. Exporters produce strings and do no I/O, apart
//! from `write_all`, which only ever writes inside the chosen output directory.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use base64::Engine;
use clap::ValueEnum;

use crate::audit::Check;
use crate::color::Color;
use crate::palette::{Mode, Palette};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Json,
    Css,
    Gtk,
    Kitty,
    Hyprland,
    Quickshell,
    Starship,
    Fish,
    Nvim,
    Base16,
    Preview,
}

impl Format {
    pub fn file_name(self) -> &'static str {
        match self {
            Format::Json => "palette.json",
            Format::Css => "colors.css",
            Format::Gtk => "gtk.css",
            Format::Kitty => "kitty.conf",
            Format::Hyprland => "hyprland.conf",
            Format::Quickshell => "Palette.qml",
            Format::Starship => "starship.toml",
            Format::Fish => "velure.theme",
            Format::Nvim => "velure.lua",
            Format::Base16 => "base16.yaml",
            Format::Preview => "preview.html",
        }
    }
}

pub struct Context_<'a> {
    pub palette: &'a Palette,
    pub checks: &'a [Check],
    pub thumbnail_jpeg: &'a [u8],
}

pub fn render(format: Format, ctx: &Context_) -> Result<String> {
    let p = ctx.palette;
    Ok(match format {
        Format::Json => json(p, ctx.checks)?,
        Format::Css => css(p),
        Format::Gtk => gtk(p),
        Format::Kitty => kitty(p),
        Format::Hyprland => hyprland(p),
        Format::Quickshell => quickshell(p),
        Format::Starship => starship(p),
        Format::Fish => fish(p),
        Format::Nvim => nvim(p),
        Format::Base16 => base16(p),
        Format::Preview => preview(ctx),
    })
}

pub fn write_all(dir: &Path, formats: &[Format], ctx: &Context_) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("no se pudo crear {}", dir.display()))?;
    for f in formats {
        let path = dir.join(f.file_name());
        fs::write(&path, render(*f, ctx)?)
            .with_context(|| format!("no se pudo escribir {}", path.display()))?;
    }
    Ok(())
}

/// The palette plus its audit, which is what `--json` prints.
pub fn json(p: &Palette, checks: &[Check]) -> Result<String> {
    #[derive(serde::Serialize)]
    struct Output<'a> {
        #[serde(flatten)]
        palette: &'a Palette,
        audit: &'a [Check],
    }
    let output = Output {
        palette: p,
        audit: checks,
    };
    Ok(serde_json::to_string_pretty(&output)? + "\n")
}

fn header(p: &Palette, comment: &str) -> String {
    format!(
        "{comment} Generado por velure desde {} ({}, armonía {})\n",
        p.meta.source,
        p.meta.style.label(),
        p.meta.scheme.label(),
    )
}

fn css(p: &Palette) -> String {
    let mut s = format!("/* {} */\n:root {{\n", header(p, "").trim());
    for (name, c) in p.roles() {
        s += &format!("  --{}: {};\n", name.replace('_', "-"), c.hex());
    }
    s + "}\n"
}

fn gtk(p: &Palette) -> String {
    let mut s = format!("/* {} */\n", header(p, "").trim());
    for (name, c) in p.roles() {
        s += &format!("@define-color {} {};\n", name, c.hex());
    }
    let (n, a) = (&p.neutrals, &p.accents);
    for (name, c) in [
        ("accent_color", a.primary),
        ("accent_bg_color", a.primary),
        ("accent_fg_color", a.on_primary),
        ("window_bg_color", n.bg),
        ("window_fg_color", n.text),
        ("view_bg_color", n.bg),
        ("view_fg_color", n.text),
        ("headerbar_bg_color", n.crust),
        ("headerbar_fg_color", n.text),
        ("sidebar_bg_color", n.crust),
        ("sidebar_fg_color", n.text),
        ("card_bg_color", n.surface),
        ("card_fg_color", n.text),
        ("popover_bg_color", n.surface),
        ("popover_fg_color", n.text),
    ] {
        s += &format!("@define-color {} {};\n", name, c.hex());
    }
    s
}

fn kitty(p: &Palette) -> String {
    let (n, a) = (&p.neutrals, &p.accents);
    let mut s = header(p, "#");
    for (k, c) in [
        ("foreground", n.text),
        ("background", n.bg),
        ("selection_foreground", n.text),
        ("selection_background", a.primary_container),
        ("cursor", a.primary),
        ("cursor_text_color", a.on_primary),
        ("url_color", a.secondary),
        ("active_border_color", a.primary),
        ("inactive_border_color", n.border),
        ("bell_border_color", p.semantic.warning),
        ("active_tab_foreground", a.on_primary),
        ("active_tab_background", a.primary),
        ("inactive_tab_foreground", n.subtext),
        ("inactive_tab_background", n.surface),
        ("tab_bar_background", n.crust),
        ("mark1_foreground", a.on_primary),
        ("mark1_background", a.primary),
    ] {
        s += &format!("{k} {}\n", c.hex());
    }
    for (i, c) in p.ansi.iter().enumerate() {
        s += &format!("color{i} {}\n", c.hex());
    }
    s
}

fn hyprland(p: &Palette) -> String {
    let mut s = header(p, "#");
    s += "# Uso: source = ~/.config/hypr/velure-tema.conf y luego $primary, $border...\n";
    for (name, c) in p.roles() {
        s += &format!("${} = rgb({})\n", name, c.bare());
    }
    s
}

fn quickshell(p: &Palette) -> String {
    let mut s = format!(
        "// {}pragma Singleton\nimport Quickshell\nimport QtQuick\n\nSingleton {{\n",
        header(p, "").trim_start()
    );
    for (name, c) in p.roles() {
        s += &format!(
            "    readonly property color {}: \"{}\"\n",
            camel(name),
            c.hex()
        );
    }
    s += "    readonly property var ansi: [";
    s += &p
        .ansi
        .iter()
        .map(|c| format!("\"{}\"", c.hex()))
        .collect::<Vec<_>>()
        .join(", ");
    s + "]\n}\n"
}

fn camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in name.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn starship(p: &Palette) -> String {
    let mut s = header(p, "#");
    s += "# Copia estas líneas en ~/.config/starship.toml\npalette = \"velure-tema\"\n\n[palettes.velure-tema]\n";
    for (name, c) in p.roles() {
        s += &format!("{} = \"{}\"\n", name, c.hex());
    }
    s
}

fn fish(p: &Palette) -> String {
    let (n, a, m) = (&p.neutrals, &p.accents, &p.semantic);
    let mut s = header(p, "#");
    for (k, c) in [
        ("fish_color_normal", n.text),
        ("fish_color_command", a.primary),
        ("fish_color_keyword", a.secondary),
        ("fish_color_quote", m.success),
        ("fish_color_redirection", a.tertiary),
        ("fish_color_end", a.tertiary),
        ("fish_color_error", m.error),
        ("fish_color_param", n.subtext),
        ("fish_color_option", n.subtext),
        ("fish_color_comment", n.muted),
        ("fish_color_operator", a.secondary),
        ("fish_color_escape", a.tertiary),
        ("fish_color_autosuggestion", n.muted),
        ("fish_color_cwd", a.primary),
        ("fish_color_host", n.subtext),
        ("fish_color_user", a.secondary),
        ("fish_color_cancel", m.error),
        ("fish_pager_color_prefix", a.primary),
        ("fish_pager_color_completion", n.text),
        ("fish_pager_color_description", n.muted),
        ("fish_pager_color_progress", n.muted),
    ] {
        s += &format!("{k} {}\n", c.bare());
    }
    s + &format!(
        "fish_color_selection --background={}\n",
        a.primary_container.bare()
    )
}

fn nvim(p: &Palette) -> String {
    let (n, a, m) = (&p.neutrals, &p.accents, &p.semantic);
    let hl = |group: &str, fg: Option<Color>, bg: Option<Color>, extra: &str| {
        let mut parts = Vec::new();
        if let Some(c) = fg {
            parts.push(format!("fg = \"{}\"", c.hex()));
        }
        if let Some(c) = bg {
            parts.push(format!("bg = \"{}\"", c.hex()));
        }
        if !extra.is_empty() {
            parts.push(extra.to_string());
        }
        format!("hl(\"{group}\", {{ {} }})\n", parts.join(", "))
    };
    let background = match p.meta.mode {
        Mode::Dark => "dark",
        Mode::Light => "light",
    };
    let mut s = header(p, "--");
    s += &format!(
        "vim.cmd(\"highlight clear\")\nvim.o.background = \"{background}\"\nvim.g.colors_name = \"velure\"\nlocal function hl(g, o) vim.api.nvim_set_hl(0, g, o) end\n\n"
    );
    let (fg, bg) = (Some(n.text), Some(n.bg));
    let f = |c: Color| Some(c);
    for line in [
        hl("Normal", fg, bg, ""),
        hl("NormalFloat", fg, f(n.surface), ""),
        hl("FloatBorder", f(n.border), f(n.surface), ""),
        hl("CursorLine", None, f(n.surface), ""),
        hl("CursorLineNr", f(a.primary), None, "bold = true"),
        hl("LineNr", f(n.muted), None, ""),
        hl("SignColumn", None, bg, ""),
        hl("Visual", None, f(a.primary_container), ""),
        hl("Search", f(a.on_primary), f(a.secondary), ""),
        hl("IncSearch", f(a.on_primary), f(a.primary), ""),
        hl(
            "MatchParen",
            f(a.primary),
            None,
            "bold = true, underline = true",
        ),
        hl("Pmenu", f(n.subtext), f(n.surface), ""),
        hl("PmenuSel", f(a.on_primary), f(a.primary), ""),
        hl("StatusLine", f(n.subtext), f(n.crust), ""),
        hl("StatusLineNC", f(n.muted), f(n.crust), ""),
        hl("WinSeparator", f(n.border), None, ""),
        hl("Directory", f(a.primary), None, ""),
        hl("Title", f(a.primary), None, "bold = true"),
        hl("Comment", f(n.muted), None, "italic = true"),
        hl("Constant", f(a.tertiary), None, ""),
        hl("String", f(m.success), None, ""),
        hl("Identifier", fg, None, ""),
        hl("Function", f(a.secondary), None, ""),
        hl("Statement", f(a.primary), None, ""),
        hl("Keyword", f(a.primary), None, ""),
        hl("Operator", f(n.subtext), None, ""),
        hl("PreProc", f(a.tertiary), None, ""),
        hl("Type", f(a.tertiary), None, ""),
        hl("Special", f(a.secondary), None, ""),
        hl("Delimiter", f(n.subtext), None, ""),
        hl("Error", f(m.error), None, ""),
        hl("Todo", f(a.on_primary), f(a.primary), "bold = true"),
        hl("DiagnosticError", f(m.error), None, ""),
        hl("DiagnosticWarn", f(m.warning), None, ""),
        hl("DiagnosticInfo", f(m.info), None, ""),
        hl("DiagnosticHint", f(a.tertiary), None, ""),
        hl("DiffAdd", f(m.success), None, ""),
        hl("DiffDelete", f(m.error), None, ""),
        hl("DiffChange", f(m.warning), None, ""),
    ] {
        s += &line;
    }
    s += "\nvim.g.terminal_color_0, vim.g.terminal_color_15 = ";
    s += &format!("\"{}\", \"{}\"\n", p.ansi[0].hex(), p.ansi[15].hex());
    for (i, c) in p.ansi.iter().enumerate().take(15).skip(1) {
        s += &format!("vim.g.terminal_color_{i} = \"{}\"\n", c.hex());
    }
    s
}

fn base16(p: &Palette) -> String {
    let variant = match p.meta.mode {
        Mode::Dark => "dark",
        Mode::Light => "light",
    };
    let mut s = format!(
        "system: \"base16\"\nname: \"Velure\"\nauthor: \"velure\"\nvariant: \"{variant}\"\npalette:\n"
    );
    for (i, c) in p.base16.iter().enumerate() {
        s += &format!("  base{i:02X}: \"{}\"\n", c.hex());
    }
    s
}

fn preview(ctx: &Context_) -> String {
    let p = ctx.palette;
    let vars: String = p
        .roles()
        .iter()
        .map(|(name, c)| format!("--{}:{};", name.replace('_', "-"), c.hex()))
        .chain(
            p.ansi
                .iter()
                .enumerate()
                .map(|(i, c)| format!("--a{i}:{};", c.hex())),
        )
        .collect();
    let swatches: String = p
        .roles()
        .iter()
        .map(|(name, c)| {
            format!(
                "<div class=\"sw\"><i style=\"background:{h}\"></i><b>{name}</b><code>{h}</code></div>",
                h = c.hex()
            )
        })
        .collect();
    let ansi: String = p
        .ansi
        .iter()
        .map(|c| {
            format!(
                "<i style=\"background:{h}\" title=\"{h}\"></i>",
                h = c.hex()
            )
        })
        .collect();
    let checks: String = ctx
        .checks
        .iter()
        .map(|c| {
            format!(
                "<li class=\"{}\"><span>{}</span><code>{}</code></li>",
                if c.passed { "ok" } else { "ko" },
                esc(&c.name),
                esc(&c.value)
            )
        })
        .collect();
    let thumb = base64::engine::general_purpose::STANDARD.encode(ctx.thumbnail_jpeg);
    let origin = match (p.meta.secondary_from_image, p.meta.tertiary_from_image) {
        (true, true) => "acentos tomados de la imagen",
        (false, false) => "acentos derivados por armonía",
        _ => "acentos entre imagen y armonía",
    };
    include_str!("templates/preview.html")
        .replace("%%VARS%%", &vars)
        .replace("%%SOURCE%%", &esc(&p.meta.source))
        .replace("%%STYLE%%", p.meta.style.label())
        .replace("%%SCHEME%%", p.meta.scheme.label())
        .replace("%%ORIGIN%%", origin)
        .replace("%%THUMB%%", &thumb)
        .replace("%%SWATCHES%%", &swatches)
        .replace("%%ANSI%%", &ansi)
        .replace("%%CHECKS%%", &checks)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camel_case() {
        assert_eq!(camel("primary_container"), "primaryContainer");
        assert_eq!(camel("bg"), "bg");
    }

    #[test]
    fn html_is_escaped() {
        assert_eq!(esc("<a&\">"), "&lt;a&amp;&quot;&gt;");
    }
}

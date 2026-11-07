//! Velure's own app themes. Each app knows where its theme lives and which token goes
//! where, so its colour decisions are tested here instead of living in a template.
//!
//! `config.toml` (see `templates`) picks which apps to write and can move their output or
//! add a hook. Templates remain for what is personal to a desktop rather than to an app.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::tokens::Tokens;

pub mod eza;
pub mod fastfetch;
pub mod fish;
pub mod gtk;
pub mod icons;
pub mod kitty;
pub mod starship;
pub mod web;
pub mod zed;

/// A colour in an app theme: a token, optionally with an alpha suffix, or a fixed value.
pub enum Paint {
    Tok(&'static str, Option<&'static str>),
    Fixed(&'static str),
}

impl Paint {
    pub fn resolve(&self, tokens: &Tokens) -> String {
        match self {
            Paint::Tok(name, alpha) => {
                let color = tokens
                    .get(name)
                    .unwrap_or_else(|| panic!("token desconocido en un tema: {name}"));
                format!("{}{}", color.hex(), alpha.unwrap_or(""))
            }
            Paint::Fixed(value) => (*value).to_string(),
        }
    }

    pub fn token(&self) -> Option<&'static str> {
        match self {
            Paint::Tok(name, _) => Some(name),
            Paint::Fixed(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum App {
    Zed,
    Kitty,
    Fish,
    Starship,
    Fastfetch,
    Eza,
    Gtk3,
    Gtk4,
    Icons,
    Firefox,
    Tidal,
}

impl App {
    pub const ALL: [App; 11] = [
        App::Zed,
        App::Kitty,
        App::Fish,
        App::Starship,
        App::Fastfetch,
        App::Eza,
        App::Gtk3,
        App::Gtk4,
        App::Icons,
        App::Firefox,
        App::Tidal,
    ];

    pub fn name(self) -> &'static str {
        match self {
            App::Zed => "zed",
            App::Kitty => "kitty",
            App::Fish => "fish",
            App::Starship => "starship",
            App::Fastfetch => "fastfetch",
            App::Eza => "eza",
            App::Gtk3 => "gtk3",
            App::Gtk4 => "gtk4",
            App::Icons => "iconos",
            App::Firefox => "firefox",
            App::Tidal => "tidal",
        }
    }

    pub fn from_name(name: &str) -> Option<App> {
        App::ALL.into_iter().find(|a| a.name() == name)
    }

    /// Where the theme goes unless the config says otherwise, relative to the home.
    pub fn default_output(self) -> &'static str {
        match self {
            App::Zed => "~/.config/zed/themes/velure.json",
            App::Kitty => "~/.config/kitty/velure.conf",
            App::Fish => "~/.config/fish/velure-colors.fish",
            App::Starship => "~/.config/starship/palette-velure.toml",
            App::Fastfetch => "~/.config/fastfetch/config.jsonc",
            App::Eza => "~/.config/eza/theme.yml",
            App::Gtk3 => "~/.local/share/themes/Velure/gtk-3.0/colors.css",
            App::Gtk4 => "~/.config/gtk-4.0/gtk.css",
            App::Icons => "~/.local/share/icons/Velure",
            App::Firefox => "~/.cache/velure",
            App::Tidal => "~/.config/tidal-hifi/themes/velure.css",
        }
    }

    /// The theme file. `base` is the user's file that some apps build on (fastfetch).
    pub fn render(self, tokens: &Tokens, base: Option<&str>) -> Result<String> {
        Ok(match self {
            App::Zed => zed::render(tokens),
            App::Kitty => kitty::render(tokens),
            App::Fish => fish::render(tokens),
            App::Starship => starship::render(tokens),
            App::Eza => eza::render(tokens),
            App::Gtk3 => gtk::render(tokens, false),
            App::Gtk4 => gtk::render(tokens, true),
            App::Tidal => web::render(web::TIDAL, tokens)?,
            App::Icons | App::Firefox => {
                unreachable!("se escriben en una carpeta, con write")
            }
            App::Fastfetch => {
                let path = crate::templates::expand_home(base.unwrap_or(fastfetch::DEFAULT_BASE));
                let text = std::fs::read_to_string(&path)
                    .with_context(|| format!("no se pudo leer la base {}", path.display()))?;
                fastfetch::render(tokens, &text)?
            }
        })
    }

    /// Writes the theme to `output`, a file or, for icons, a directory. `base` is the
    /// user's file or directory that some apps build on.
    pub fn write(
        self,
        tokens: &Tokens,
        base: Option<&str>,
        output: &std::path::Path,
    ) -> Result<()> {
        if self == App::Icons {
            let source = crate::templates::expand_home(base.unwrap_or(icons::DEFAULT_SOURCE));
            return icons::write(tokens, &source, output);
        }
        if self == App::Firefox {
            return web::write_firefox(tokens, output);
        }
        let content = self.render(tokens, base)?;
        crate::templates::write_atomic(output, &content)
            .with_context(|| format!("no se pudo escribir {}", output.display()))
    }

    /// Token usage counts, for the desktop guide's table of which app uses what.
    pub fn usage(self) -> Vec<(&'static str, usize)> {
        let names = match self {
            App::Zed => zed::usage(),
            App::Kitty => kitty::usage(),
            App::Fish => fish::usage(),
            App::Starship => starship::usage(),
            App::Fastfetch => fastfetch::usage(),
            App::Eza => eza::usage(),
            App::Gtk3 | App::Gtk4 => gtk::usage(),
            App::Icons => icons::usage(),
            App::Firefox => web::FIREFOX
                .iter()
                .flat_map(|(_, css)| web::usage(css))
                .collect(),
            App::Tidal => web::usage(web::TIDAL),
        };
        let mut counts: Vec<(&'static str, usize)> = Vec::new();
        for name in names {
            match counts.iter_mut().find(|(n, _)| *n == name) {
                Some((_, count)) => *count += 1,
                None => counts.push((name, 1)),
            }
        }
        counts
    }
}

pub fn output_path(app: App, configured: Option<&str>) -> PathBuf {
    crate::templates::expand_home(configured.unwrap_or(app.default_output()))
}

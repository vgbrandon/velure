//! Icons: the "Velure" icon theme, Adwaita with its folders in the primary hue. It is
//! written to `~/.local/share/icons/Velure` and inherits everything else from Adwaita.
//!
//! Adwaita's folders are blue (OKLCH hue around 250°). Each blue part takes the primary
//! hue and keeps its own lightness, so the shading survives; its chroma is scaled by how
//! colourful the primary is, so a sober theme gets quietly tinted folders, not loud ones.

use std::path::Path;

use anyhow::{Context, Result};

use crate::color::{Color, Oklch};
use crate::tokens::Tokens;

pub const DEFAULT_SOURCE: &str = "/usr/share/icons/Adwaita/scalable";

/// Icons recoloured, relative to the scalable directory.
const ICONS: &[&str] = &[
    "places/folder.svg",
    "places/folder-documents.svg",
    "places/folder-download.svg",
    "places/folder-drag-accept.svg",
    "places/folder-music.svg",
    "places/folder-pictures.svg",
    "places/folder-publicshare.svg",
    "places/folder-remote.svg",
    "places/folder-templates.svg",
    "places/folder-videos.svg",
    "places/user-home.svg",
    "places/user-desktop.svg",
    "mimetypes/inode-directory.svg",
];

/// The hue range of Adwaita's folder blues, and the chroma of its main blue.
const BLUE_HUES: std::ops::RangeInclusive<f64> = 225.0..=280.0;
const ADWAITA_CHROMA: f64 = 0.13;

const INDEX: &str = "[Icon Theme]
Name=Velure
Comment=Adwaita con las carpetas en el tono del fondo de pantalla (generado por velure)
Inherits=Adwaita,hicolor
Directories=scalable/places,scalable/mimetypes

[scalable/places]
Context=Places
Size=128
MinSize=8
MaxSize=512
Type=Scalable

[scalable/mimetypes]
Context=MimeTypes
Size=128
MinSize=8
MaxSize=512
Type=Scalable
";

fn parse_hex(s: &str) -> Option<Color> {
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(Color {
        r: (v >> 16) as u8,
        g: (v >> 8) as u8,
        b: v as u8,
    })
}

/// The folder colour `original` in the theme's primary hue.
pub fn recolor(original: Color, primary: Oklch) -> Color {
    let c = original.to_lch();
    if c.c < 0.05 || !BLUE_HUES.contains(&c.h) {
        return original;
    }
    let scale = (primary.c / ADWAITA_CHROMA).clamp(0.35, 1.0);
    Oklch::new(c.l, c.c * scale, primary.h).to_color()
}

/// Every `#rrggbb` in an SVG passed through `recolor`.
pub fn recolor_svg(svg: &str, primary: Oklch) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find('#') {
        out.push_str(&rest[..i]);
        let candidate = rest.get(i + 1..i + 7);
        let next_is_hex = rest[i + 1..]
            .chars()
            .nth(6)
            .is_some_and(|c| c.is_ascii_hexdigit());
        match candidate.filter(|_| !next_is_hex).and_then(parse_hex) {
            Some(color) => {
                out.push_str(&recolor(color, primary).hex());
                rest = &rest[i + 7..];
            }
            None => {
                out.push('#');
                rest = &rest[i + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn write(tokens: &Tokens, source: &Path, output: &Path) -> Result<()> {
    let primary = tokens.get("primary").expect("token primary").to_lch();
    let mut written = 0;
    for icon in ICONS {
        let from = source.join(icon);
        let Ok(svg) = std::fs::read_to_string(&from) else {
            continue;
        };
        let to = output.join("scalable").join(icon);
        crate::templates::write_atomic(&to, &recolor_svg(&svg, primary))
            .with_context(|| format!("no se pudo escribir {}", to.display()))?;
        written += 1;
    }
    anyhow::ensure!(
        written > 0,
        "no hay iconos de Adwaita en {}",
        source.display()
    );
    crate::templates::write_atomic(&output.join("index.theme"), INDEX)
}

pub fn usage() -> Vec<&'static str> {
    vec!["primary"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_blue_parts_change() {
        let primary = Oklch::new(0.7, 0.12, 30.0);
        let svg = r##"<a fill="#62a0ea"/><b fill="#ffffff"/><c stop-color="#ababab"/><d href="#a"/><e fill="#62a0eaff"/>"##;
        let out = recolor_svg(svg, primary);
        assert!(!out.contains("#62a0ea\""), "el azul cambia: {out}");
        assert!(out.contains("#ffffff") && out.contains("#ababab") && out.contains("href=\"#a\""));
        assert!(
            out.contains("#62a0eaff"),
            "un color de 8 dígitos no se toca"
        );
    }

    #[test]
    fn folders_keep_their_shading_and_follow_the_theme() {
        let primary = Oklch::new(0.7, 0.12, 30.0);
        for hex in ["438de6", "62a0ea", "a4caee"] {
            let original = parse_hex(hex).unwrap();
            let new = recolor(original, primary).to_lch();
            assert!((new.l - original.to_lch().l).abs() < 0.02, "{hex}");
            assert!(
                crate::color::hue_diff(new.h, 30.0).abs() < 4.0,
                "{hex}: {}",
                new.h
            );
        }
    }

    #[test]
    fn a_sober_theme_gets_quiet_folders() {
        let sober = Oklch::new(0.7, 0.04, 30.0);
        let new = recolor(parse_hex("438de6").unwrap(), sober).to_lch();
        assert!(new.c < 0.06, "croma {}", new.c);
    }
}

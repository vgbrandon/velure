//! Firefox and Tidal: stylesheets for programs whose look lives in CSS. The CSS is part of
//! velure (`src/apps/firefox/*.css`, `src/apps/tidal.css`), so the colour decisions are
//! reviewed and tested here; the selectors only follow each program's structure.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use crate::templates::{fill, write_atomic};
use crate::tokens::Tokens;

/// Firefox's stylesheets and the names its scripts look for.
pub const FIREFOX: &[(&str, &str)] = &[
    (
        "firefox-userChrome.css",
        include_str!("firefox/userChrome.css"),
    ),
    (
        "firefox-userContent.css",
        include_str!("firefox/userContent.css"),
    ),
    (
        "firefox-translucent.css",
        include_str!("firefox/translucent.css"),
    ),
    (
        "firefox-translucent-content.css",
        include_str!("firefox/translucent-content.css"),
    ),
];

pub const TIDAL: &str = include_str!("tidal.css");

pub fn render(css: &str, tokens: &Tokens) -> Result<String> {
    let colors: HashMap<String, String> = tokens
        .iter()
        .map(|(k, v)| (k.to_string(), v.hex()))
        .collect();
    let (out, unknown) = fill(css, &colors, "");
    ensure!(
        unknown.is_empty(),
        "tokens desconocidos: {}",
        unknown.join(", ")
    );
    Ok(out)
}

/// Writes Firefox's four stylesheets into `dir`.
pub fn write_firefox(tokens: &Tokens, dir: &Path) -> Result<()> {
    for (name, css) in FIREFOX {
        let path = dir.join(name);
        write_atomic(&path, &render(css, tokens)?)
            .with_context(|| format!("no se pudo escribir {}", path.display()))?;
    }
    Ok(())
}

/// Every token a stylesheet reads, once per use.
pub fn usage(css: &'static str) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut rest = css;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        out.push(after[..end].trim());
        rest = &after[end + 2..];
    }
    out
}

/// `(foreground, background)` pairs that Tidal draws one on the other.
pub const TIDAL_READABLE: &[(&str, &str)] = &[
    ("text", "bg"),
    ("subtext", "bg"),
    ("text", "crust"),
    ("subtext", "crust"),
    ("text", "surface_high"),
    ("on_primary_container", "primary_container"),
    ("text", "primary_container"),
    ("secondary", "bg"),
    ("on_primary", "primary"),
    ("text", "overlay"),
];

/// `(foreground, background)` pairs that Firefox draws one on the other.
pub const FIREFOX_READABLE: &[(&str, &str)] = &[
    ("text", "bg"),
    ("subtext", "bg"),
    ("text", "surface"),
    ("text", "surface_high"),
    ("text", "overlay"),
    ("text", "border"),
    ("on_primary", "primary"),
    ("on_primary_container", "primary_container"),
    ("primary", "bg"),
];

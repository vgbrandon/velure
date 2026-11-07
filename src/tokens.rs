//! Velure's design tokens: one name per colour, each with a derivation rule. The audited
//! palette is the only input, so everything here inherits its colour decisions. The rules
//! only fill the steps the desktop needs and the palette doesn't have.
//!
//! Tokens are dark only: the desktop they feed assumes a dark theme.

use serde_json::{Value, json};

use crate::color::{Color, Oklch, hue_diff, norm_hue};
use crate::palette::{Palette, ensure_contrast, mix};

/// Below this chroma an image colour reads as grey and says nothing about hue.
const COLORFUL: f64 = 0.03;
/// A wider gradient would turn into a rainbow on complementary or triadic themes.
const GRADIENT_SPREAD: f64 = 35.0;
/// Hues for code colours when the image doesn't offer enough of its own.
const SYNTAX_ANCHORS: [f64; 8] = [150.0, 58.0, 200.0, 330.0, 255.0, 27.0, 95.0, 295.0];
/// Lightness and chroma of a code colour taken from a hue alone: quieter than an accent.
const SYNTAX_LIGHTNESS: f64 = 0.78;
const SYNTAX_CHROMA: f64 = 0.08;
/// Accents can be nearly grey on sober themes, but code needs a visible tint.
const SYNTAX_MIN_CHROMA: f64 = 0.06;
/// Below this OKLab distance two kinds of code read as the same colour.
const SYNTAX_MIN_DELTA: f64 = 0.05;

/// Minimum WCAG contrast that each `(foreground, background)` pair must reach.
pub const CONTRAST_RULES: &[(&str, &str, f64)] = &[
    ("text", "bg", 7.0),
    ("text", "overlay", 4.5),
    ("subtext", "bg", 4.5),
    ("subtext", "surface", 4.5),
    ("muted", "bg", 3.0),
    ("outline", "surface", 3.0),
    ("emphasis", "bg", 7.0),
    ("on_emphasis", "emphasis", 7.0),
    ("primary", "bg", 4.5),
    ("secondary", "bg", 4.5),
    ("tertiary", "bg", 4.5),
    ("on_primary", "primary", 4.5),
    ("on_primary_container", "primary_container", 7.0),
    ("text", "primary_container", 4.5),
    ("text", "error_container", 4.5),
    ("error", "bg", 4.5),
    ("warning", "bg", 4.5),
    ("success", "bg", 4.5),
    ("info", "bg", 4.5),
    ("error", "surface", 4.5),
    ("warning", "surface", 4.5),
    ("success", "surface", 4.5),
    ("info", "surface", 4.5),
    ("on_error", "error", 4.5),
    ("on_success", "success", 4.5),
    ("on_warning", "warning", 4.5),
    ("syntax_keyword", "bg", 4.5),
    ("syntax_function", "bg", 4.5),
    ("syntax_type", "bg", 4.5),
    ("syntax_string", "bg", 4.5),
    ("syntax_number", "bg", 4.5),
    ("syntax_property", "bg", 4.5),
];

pub struct Tokens(Vec<(String, Color)>);

impl Tokens {
    pub fn get(&self, name: &str) -> Option<Color> {
        self.0.iter().find(|(n, _)| n == name).map(|(_, c)| *c)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, Color)> {
        self.0.iter().map(|(n, c)| (n.as_str(), *c))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn push(&mut self, name: impl Into<String>, color: Color) {
        self.0.push((name.into(), color));
    }

    /// Every rule of `CONTRAST_RULES` that this set breaks, described for a person.
    pub fn contrast_failures(&self) -> Vec<String> {
        CONTRAST_RULES
            .iter()
            .filter_map(|(fg, bg, min)| {
                let ratio = self.get(fg)?.contrast(self.get(bg)?);
                (ratio < *min).then(|| format!("{fg} sobre {bg}: {ratio:.2} (mín {min})"))
            })
            .collect()
    }
}

fn lch(l: f64, c: f64, h: f64) -> Color {
    Oklch::new(l, c, h).to_color()
}

/// Text for a solid fill: the tinted dark or tinted light, whichever reads better.
fn on_fill(fill: Color) -> Color {
    let h = fill.to_lch().h;
    let (dark, light) = (lch(0.18, 0.03, h), lch(0.96, 0.02, h));
    if fill.contrast(dark) >= fill.contrast(light) {
        dark
    } else {
        light
    }
}

/// A deep, quiet version of a colour for backgrounds that hold text.
fn container(color: Color) -> Color {
    let c = color.to_lch();
    lch(0.34, (c.c * 0.55).clamp(0.03, 0.08), c.h)
}

fn on_container(container: Color) -> Color {
    let h = container.to_lch().h;
    ensure_contrast(Oklch::new(0.92, 0.03, h), container, 7.0).to_color()
}

/// Three hues for literals, far from the accents: image hues first, then neutral anchors.
/// Each pass relaxes the required distance, so a crowded wheel still yields three.
fn literal_hues(p: &Palette) -> [f64; 3] {
    let a = &p.accents;
    let family = [a.primary, a.secondary, a.tertiary].map(|c| c.to_lch().h);
    let gap = |h: f64, others: &[f64]| {
        others
            .iter()
            .map(|o| hue_diff(h, *o).abs())
            .fold(f64::MAX, f64::min)
    };
    let mut image: Vec<f64> = p
        .meta
        .image_colors
        .iter()
        .map(|c| c.color.to_lch())
        .filter(|c| c.c >= COLORFUL)
        .map(|c| c.h)
        .collect();
    image.sort_by(|x, y| gap(*y, &family).total_cmp(&gap(*x, &family)));

    let mut picked: Vec<f64> = Vec::with_capacity(3);
    for distance in [25.0, 12.0, 0.0] {
        for h in image.iter().chain(&SYNTAX_ANCHORS) {
            let clear_of_picked = picked.is_empty() || gap(*h, &picked) >= distance;
            if picked.len() < 3 && clear_of_picked && gap(*h, &family) >= distance {
                picked.push(*h);
            }
        }
    }
    [picked[0], picked[1], picked[2]]
}

/// Moves a colour the least needed to stay legible and apart from `others`: lightness first,
/// then hue, so it keeps as much of its origin as it can.
fn separated(start: Oklch, others: &[Color], bg: Color) -> Color {
    for turn in [0.0, 15.0, -15.0, 30.0, -30.0, 45.0, -45.0, 60.0, -60.0] {
        for shift in [0.0, 0.04, -0.04, 0.08, -0.08] {
            let color = lch(start.l + shift, start.c, norm_hue(start.h + turn));
            let apart = others.iter().all(|o| color.delta_e(*o) >= SYNTAX_MIN_DELTA);
            if apart && color.contrast(bg) >= 4.5 {
                return color;
            }
        }
    }
    start.to_color()
}

/// Keyword, function, type, string, number and property, in that order: each one is
/// settled against the text colours and the ones before it. The first three start from
/// the accents, so code still belongs to the theme.
fn syntax(p: &Palette) -> [Color; 6] {
    let (n, a) = (&p.neutrals, &p.accents);
    let cap = p.meta.style.chroma_cap();
    let tinted = |c: Color| {
        let c = c.to_lch();
        Oklch::new(c.l, c.c.max(SYNTAX_MIN_CHROMA.min(cap)), c.h)
    };
    let literal = |h: f64| Oklch::new(SYNTAX_LIGHTNESS, SYNTAX_CHROMA.min(cap), h);
    let [string, number, property] = literal_hues(p);
    let starts = [
        tinted(a.primary),
        tinted(a.secondary),
        tinted(a.tertiary),
        literal(string),
        literal(number),
        literal(property),
    ];
    let mut taken = vec![n.text, n.subtext, n.muted];
    starts.map(|start| {
        let color = separated(start, &taken, n.bg);
        taken.push(color);
        color
    })
}

/// Tertiary side → primary → secondary side, from deep to light, for visualisers.
fn gradient(p: &Palette) -> [Color; 8] {
    let primary = p.accents.primary.to_lch();
    let cap = p.meta.style.chroma_cap();
    let turn_to =
        |c: Color| hue_diff(primary.h, c.to_lch().h).clamp(-GRADIENT_SPREAD, GRADIENT_SPREAD);
    let (to_secondary, to_tertiary) = (turn_to(p.accents.secondary), turn_to(p.accents.tertiary));
    std::array::from_fn(|i| {
        let t = i as f64 / 7.0;
        let turn = if t < 0.5 {
            to_tertiary * (1.0 - t / 0.5)
        } else {
            to_secondary * ((t - 0.5) / 0.5)
        };
        let chroma = (primary.c.min(cap) * (0.8 + 0.4 * t)).min(cap);
        lch(0.60 + 0.22 * t, chroma, norm_hue(primary.h + turn))
    })
}

pub fn derive(p: &Palette) -> Tokens {
    let (n, a, s) = (&p.neutrals, &p.accents, &p.semantic);
    let mut t = Tokens(Vec::with_capacity(70));

    // Elevation: the palette's own steps, with midpoints that keep its rhythm.
    t.push("crust", n.crust);
    t.push("bg", n.bg);
    t.push("surface_low", mix(n.bg, n.surface, 0.5));
    t.push("surface", n.surface);
    t.push("surface_high", mix(n.surface, n.overlay, 0.5));
    t.push("overlay", n.overlay);

    // Lines: a quiet separator, and an outline strong enough to find a control by.
    t.push("border", n.border);
    let start = mix(n.border, n.muted, 0.5).to_lch();
    t.push("outline", ensure_contrast(start, n.surface, 3.0).to_color());

    t.push("text", n.text);
    t.push("subtext", n.subtext);
    t.push("muted", n.muted);

    // Emphasis without colour: active workspaces, checkboxes, badges.
    let emphasis = lch(0.88, 0.02, a.primary.to_lch().h);
    t.push("emphasis", emphasis);
    t.push("on_emphasis", on_fill(emphasis));

    t.push("primary", a.primary);
    t.push("on_primary", a.on_primary);
    t.push("primary_container", a.primary_container);
    t.push("on_primary_container", on_container(a.primary_container));
    t.push("secondary", a.secondary);
    t.push("tertiary", a.tertiary);

    t.push("error", s.error);
    t.push("on_error", on_fill(s.error));
    t.push("error_container", container(s.error));
    t.push("warning", s.warning);
    t.push("on_warning", on_fill(s.warning));
    t.push("success", s.success);
    t.push("on_success", on_fill(s.success));
    t.push("info", s.info);

    for (i, color) in p.ansi.iter().enumerate() {
        t.push(format!("ansi_{i}"), *color);
    }

    let [keyword, function, kind, string, number, property] = syntax(p);
    t.push("syntax_keyword", keyword);
    t.push("syntax_function", function);
    t.push("syntax_type", kind);
    t.push("syntax_string", string);
    t.push("syntax_number", number);
    t.push("syntax_property", property);

    for (i, color) in gradient(p).iter().enumerate() {
        t.push(format!("gradient_{i}"), *color);
    }
    t
}

/// The image colour a role most plausibly came from: the closest hue with real colour.
fn origin(image: &[Color], color: Color, from_image: bool) -> Option<Color> {
    if !from_image {
        return None;
    }
    let target = color.to_lch().h;
    image
        .iter()
        .filter(|c| c.to_lch().c >= COLORFUL)
        .min_by(|a, b| {
            let distance = |c: &Color| hue_diff(c.to_lch().h, target).abs();
            distance(a).total_cmp(&distance(b))
        })
        .copied()
}

/// The ten colours a guide shows, by presence. An accent often comes from a small detail
/// of the image, so its origin takes the place of the least present colours.
fn shown_colors(image: &[Color], origins: &[Option<Color>]) -> Vec<Color> {
    const SHOWN: usize = 10;
    let required: Vec<Color> = origins.iter().flatten().copied().collect();
    let mut kept: Vec<Color> = Vec::with_capacity(SHOWN);
    for c in image {
        let free = SHOWN - kept.len();
        let pending = required.iter().filter(|r| !kept.contains(r)).count();
        if required.contains(c) || free > pending {
            kept.push(*c);
        }
    }
    kept
}

pub fn to_json(tokens: &Tokens) -> Value {
    Value::Object(
        tokens
            .iter()
            .map(|(k, v)| (k.to_string(), json!(v.hex())))
            .collect(),
    )
}

/// What the desktop's colour guides display for an image.
pub fn guide(p: &Palette, image: &str) -> Value {
    let colors: Vec<Color> = p.meta.image_colors.iter().map(|c| c.color).collect();
    let origins = [
        origin(&colors, p.accents.primary, !p.meta.achromatic_source),
        origin(&colors, p.accents.secondary, p.meta.secondary_from_image),
        origin(&colors, p.accents.tertiary, p.meta.tertiary_from_image),
    ];
    let hex_or_null = |c: Option<Color>| c.map_or(Value::Null, |c| json!(c.hex()));
    let shown: Vec<String> = shown_colors(&colors, &origins)
        .iter()
        .map(|c| c.hex())
        .collect();
    json!({
        "image": image,
        "achromatic": p.meta.achromatic_source,
        "harmony": p.meta.scheme.label(),
        "image_colors": shown,
        "origins": {
            "primary": hex_or_null(origins[0]),
            "secondary": hex_or_null(origins[1]),
            "tertiary": hex_or_null(origins[2]),
        },
        "tokens": to_json(&derive(p)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(v: u8) -> Color {
        Color { r: v, g: v, b: v }
    }

    #[test]
    fn origins_from_small_details_are_still_shown() {
        let image: Vec<Color> = (0..14).map(|i| grey(i * 10)).collect();
        let (a, b) = (image[12], image[13]);
        let shown = shown_colors(&image, &[Some(a), Some(b), None]);
        assert_eq!(shown.len(), 10);
        assert!(shown.contains(&a) && shown.contains(&b));
        assert_eq!(shown[0], image[0], "se mantiene el orden por presencia");
    }

    #[test]
    fn short_lists_are_kept_whole() {
        let image: Vec<Color> = (0..4).map(|i| grey(i * 10)).collect();
        assert_eq!(shown_colors(&image, &[None, None, None]), image);
    }
}

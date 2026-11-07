//! Builds the theme: neutrals, accents, semantic colours, ANSI and Base16.
//!
//! The image supplies hue and character; lightness and chroma are decided here,
//! because those two are what separate an elegant theme from a loud one.

use clap::ValueEnum;
use serde::Serialize;

use crate::color::{Color, Cvd, Oklch, hue_diff, norm_hue};
use crate::extract::Cluster;
use crate::harmony::{Analysis, CHROMATIC_MIN, Fit, Scheme};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[value(name = "oscuro")]
    Dark,
    #[value(name = "claro")]
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[value(name = "sobrio")]
    Sober,
    #[value(name = "equilibrado")]
    Balanced,
    #[value(name = "expresivo")]
    Expressive,
}

impl Style {
    /// Maximum accent chroma: the main guard against a clownish result.
    pub fn chroma_cap(self) -> f64 {
        match self {
            Style::Sober => 0.075,
            Style::Balanced => 0.11,
            Style::Expressive => 0.145,
        }
    }

    /// The cap for a given mode. Light themes need dark accents, and the same chroma
    /// reads far duller at low lightness, so they get a proportional gain.
    pub fn chroma_cap_for(self, mode: Mode) -> f64 {
        self.chroma_cap() * light_gain(mode)
    }

    fn chroma_floor(self) -> f64 {
        match self {
            Style::Sober => 0.03,
            Style::Balanced => 0.04,
            Style::Expressive => 0.05,
        }
    }

    /// Neutral tint strength. It stops growing at the top style on purpose:
    /// stronger accents need a quieter field to stay balanced (Moon-Spencer).
    fn neutral_chroma(self) -> f64 {
        match self {
            Style::Sober => 0.010,
            Style::Balanced => 0.016,
            Style::Expressive => 0.016,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Style::Sober => "sobrio",
            Style::Balanced => "equilibrado",
            Style::Expressive => "expresivo",
        }
    }
}

/// Chroma gain of light themes, roughly the ratio between the accent lightness of
/// each mode. Without it, accents on a light background look dirty: brown reds and
/// olive yellows.
fn light_gain(mode: Mode) -> f64 {
    match mode {
        Mode::Dark => 1.0,
        Mode::Light => 1.25,
    }
}

/// Scales the chroma cap by hue. Pinks, limes and cyans read as neon at chroma
/// where a blue or an ochre still looks calm, so they get a lower ceiling.
pub fn neon_factor(h: f64) -> f64 {
    let bump = |center: f64, width: f64| (-(hue_diff(h, center) / width).powi(2)).exp();
    1.0 - 0.22 * bump(345.0, 35.0) - 0.24 * bump(130.0, 30.0) - 0.14 * bump(195.0, 20.0)
}

/// Hues differ in natural lightness (yellow is light, blue is dark). Following
/// that order makes a set of accents read as balanced at the same tone.
///
/// The extra dip around red follows Munsell, where red peaks in chroma at middle
/// values. A light, low-chroma red stops being red and turns into candy pink;
/// keeping it lower gives terracotta and dusty rose instead.
fn natural_lightness(h: f64) -> f64 {
    let red_dip = 0.05 * (-(hue_diff(h, 15.0) / 40.0).powi(2)).exp();
    0.05 * (h - 100.0).to_radians().cos() - red_dip
}

/// Helmholtz-Kohlrausch: saturated colours look brighter than their lightness
/// says, so strong chroma gives back a little L. A first-order approximation.
fn hk_compensation(c: f64) -> f64 {
    0.15 * (c - 0.08).max(0.0)
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Neutrals {
    pub crust: Color,
    pub bg: Color,
    pub surface: Color,
    pub overlay: Color,
    pub border: Color,
    pub muted: Color,
    pub subtext: Color,
    pub text: Color,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Accents {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub secondary: Color,
    pub tertiary: Color,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Semantic {
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,
}

#[derive(Clone, Debug, Serialize)]
pub struct Meta {
    pub source: String,
    pub mode: Mode,
    pub style: Style,
    pub scheme: Scheme,
    /// `None` when the harmony was forced by the user.
    pub template: Option<Fit>,
    pub secondary_from_image: bool,
    pub tertiary_from_image: bool,
    pub achromatic_source: bool,
    /// The clusters found in the image, largest first.
    pub image_colors: Vec<ImageColor>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct ImageColor {
    pub color: Color,
    pub weight: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Palette {
    pub meta: Meta,
    pub neutrals: Neutrals,
    pub accents: Accents,
    pub semantic: Semantic,
    pub ansi: [Color; 16],
    pub base16: [Color; 16],
}

impl Palette {
    /// Every named UI role, in display order. Exporters are built on this list.
    pub fn roles(&self) -> Vec<(&'static str, Color)> {
        let (n, a, s) = (&self.neutrals, &self.accents, &self.semantic);
        vec![
            ("crust", n.crust),
            ("bg", n.bg),
            ("surface", n.surface),
            ("overlay", n.overlay),
            ("border", n.border),
            ("muted", n.muted),
            ("subtext", n.subtext),
            ("text", n.text),
            ("primary", a.primary),
            ("on_primary", a.on_primary),
            ("primary_container", a.primary_container),
            ("secondary", a.secondary),
            ("tertiary", a.tertiary),
            ("error", s.error),
            ("warning", s.warning),
            ("success", s.success),
            ("info", s.info),
        ]
    }
}

pub struct Options {
    pub source: String,
    pub mode: Option<Mode>,
    pub style: Style,
    pub scheme: Option<Scheme>,
}

/// Walks lightness away from `bg` until `target` contrast is met, leaving hue
/// and chroma intent intact. If that direction runs out, it tries the other.
pub fn ensure_contrast(mut lch: Oklch, bg: Color, target: f64) -> Oklch {
    if lch.to_color().contrast(bg) >= target {
        return lch;
    }
    let up = lch.l >= bg.to_lch().l;
    for dir in [up, !up] {
        let step = if dir { 0.004 } else { -0.004 };
        let mut p = lch;
        while (0.0..=1.0).contains(&p.l) {
            if p.to_color().contrast(bg) >= target {
                return p;
            }
            p.l += step;
        }
    }
    lch.l = if up { 0.98 } else { 0.1 };
    lch
}

fn neutrals(a: &Analysis, mode: Mode, style: Style) -> Neutrals {
    let h = a.atmosphere_hue;
    // Tints follow the image's own intensity, so a muted photo gets a greyer base.
    let base = style.neutral_chroma() * (a.atmosphere_chroma / 0.06).clamp(0.4, 1.0);
    // The same chroma tints a light background far more visibly than a dark one.
    let base = match mode {
        Mode::Dark => base,
        Mode::Light => base * 0.5,
    };
    // Even steps in OKLab L are even to the eye, which is what makes layers legible.
    // Raised layers come towards the viewer, so they are lighter in dark themes and
    // whiter in light ones: light surfaces sit above the page, not below it.
    let ladder = match mode {
        Mode::Dark => [0.170, 0.210, 0.260, 0.310, 0.390, 0.650, 0.770, 0.900],
        Mode::Light => [0.935, 0.965, 0.990, 0.905, 0.840, 0.520, 0.420, 0.270],
    };
    // Tint fades along the text end: a tinted surface looks intentional, but
    // tinted body text just looks off-colour.
    let tint = [1.0, 1.0, 1.0, 1.0, 0.9, 0.8, 0.55, 0.35];
    let tone = |i: usize| Oklch::new(ladder[i], base * tint[i], h);
    let bg = tone(1).to_color();
    let surface = tone(2).to_color();
    Neutrals {
        crust: tone(0).to_color(),
        bg,
        surface,
        overlay: tone(3).to_color(),
        border: tone(4).to_color(),
        muted: ensure_contrast(tone(5), bg, 4.5).to_color(),
        subtext: ensure_contrast(tone(6), bg, 7.0).to_color(),
        text: ensure_contrast(ensure_contrast(tone(7), bg, 10.0), surface, 7.0).to_color(),
    }
}

struct Pulled {
    hue: f64,
    chroma: f64,
    from_image: bool,
}

/// Replaces a theoretical hue with a nearby one the image really contains, so
/// the palette looks derived from the picture and not from a colour wheel.
fn pull_to_image(target: f64, fallback_c: f64, clusters: &[Cluster], taken: &[f64]) -> Pulled {
    const REACH: f64 = 25.0;
    const MIN_SEPARATION: f64 = 20.0;
    let best = clusters
        .iter()
        .map(|c| (c.lch(), c.weight))
        .filter(|(lch, _)| lch.c >= CHROMATIC_MIN + 0.01)
        .filter(|(lch, _)| hue_diff(lch.h, target).abs() <= REACH)
        .filter(|(lch, _)| {
            taken
                .iter()
                .all(|t| hue_diff(lch.h, *t).abs() >= MIN_SEPARATION)
        })
        .map(|(lch, w)| {
            let score = w.sqrt() * lch.c / (1.0 + hue_diff(lch.h, target).abs() / 10.0);
            (lch, score)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1));
    match best {
        Some((lch, _)) => Pulled {
            hue: lch.h,
            chroma: lch.c,
            from_image: true,
        },
        None => Pulled {
            hue: norm_hue(target),
            chroma: fallback_c,
            from_image: false,
        },
    }
}

/// The most prominent hues the image really has, compatible with the fitted
/// template and apart from the key and from each other. Fixed offsets from the
/// key would ignore them: a steel-blue and red image would get an olive accent.
fn image_hues(a: &Analysis) -> Vec<Pulled> {
    const MIN_AREA: f64 = 0.01;
    const MIN_SEPARATION: f64 = 30.0;
    const TEMPLATE_SLACK: f64 = 8.0;
    let mut candidates: Vec<_> = a
        .clusters
        .iter()
        .map(|c| (c.lch(), c.weight))
        .filter(|(lch, w)| lch.c >= CHROMATIC_MIN + 0.01 && *w >= MIN_AREA)
        .filter(|(lch, _)| a.fit.distance(lch.h) <= TEMPLATE_SLACK)
        .collect();
    candidates.sort_by(|x, y| (y.1.sqrt() * y.0.c).total_cmp(&(x.1.sqrt() * x.0.c)));

    let mut taken = vec![a.key_hue];
    let mut out = Vec::new();
    for (lch, _) in candidates {
        if taken
            .iter()
            .all(|t| hue_diff(lch.h, *t).abs() >= MIN_SEPARATION)
        {
            taken.push(lch.h);
            out.push(Pulled {
                hue: lch.h,
                chroma: lch.c,
                from_image: true,
            });
        }
    }
    out
}

/// Picks secondary and tertiary. A detected harmony takes its hues from the image
/// first; theory fills any gap, and all of a harmony the user forced. Asymmetric
/// schemes can open towards either side of the wheel, so both are tried and the
/// one the image supports best wins.
fn accent_hues(a: &Analysis, scheme: Scheme, detected: bool) -> (Pulled, Pulled) {
    let (s, t) = scheme.offsets();
    let fallback = a.key_chroma * 0.8;
    if detected && scheme != Scheme::Mono {
        let mut found = image_hues(a).into_iter();
        if let Some(sec) = found.next() {
            let ter = found.next().unwrap_or_else(|| {
                // Prefer the theoretical slot that stays clear of the secondary.
                let slot = |off: f64| norm_hue(a.key_hue + off);
                let target = [t, -t, s, -s]
                    .into_iter()
                    .map(slot)
                    .max_by(|x, y| {
                        hue_diff(*x, sec.hue)
                            .abs()
                            .min(hue_diff(*x, a.key_hue).abs())
                            .total_cmp(
                                &hue_diff(*y, sec.hue)
                                    .abs()
                                    .min(hue_diff(*y, a.key_hue).abs()),
                            )
                    })
                    .expect("slot list is non-empty");
                pull_to_image(target, fallback, &a.clusters, &[a.key_hue, sec.hue])
            });
            return (sec, ter);
        }
    }
    let resolve = |sign: f64| {
        if scheme == Scheme::Mono {
            let hue = |off: f64| Pulled {
                hue: norm_hue(a.key_hue + off * sign),
                chroma: a.key_chroma,
                from_image: true,
            };
            return (hue(s), hue(t));
        }
        let sec = pull_to_image(a.key_hue + s * sign, fallback, &a.clusters, &[a.key_hue]);
        let ter = pull_to_image(
            a.key_hue + t * sign,
            fallback,
            &a.clusters,
            &[a.key_hue, sec.hue],
        );
        (sec, ter)
    };
    let support = |p: &(Pulled, Pulled)| p.0.from_image as u8 + p.1.from_image as u8;
    let (pos, neg) = (resolve(1.0), resolve(-1.0));
    if support(&neg) > support(&pos) {
        neg
    } else {
        pos
    }
}

fn base_accent_lightness(mode: Mode) -> f64 {
    match mode {
        Mode::Dark => 0.74,
        // Just under the 4.5:1 limit on the page: any darker turns muddy.
        Mode::Light => 0.54,
    }
}

/// How strongly hue-specific lightness applies. Light themes have little room
/// between the contrast limit and muddiness, so they use less of it.
fn natural_weight(mode: Mode) -> f64 {
    match mode {
        Mode::Dark => 1.0,
        Mode::Light => 0.5,
    }
}

/// Builds an accent tone. `role_scale` sets chroma hierarchy: one colour leads,
/// the rest support it, and that visible ranking is most of what reads as "calm".
fn accent(
    h: f64,
    source_c: f64,
    role_scale: f64,
    l_offset: f64,
    mode: Mode,
    style: Style,
) -> Oklch {
    let cap = style.chroma_cap_for(mode) * neon_factor(h);
    let gain = light_gain(mode);
    let floor = style.chroma_floor() * gain;
    let c = (source_c * 0.85 * gain).clamp(floor.min(cap), cap) * role_scale;
    Oklch::new(
        base_accent_lightness(mode) + natural_lightness(h) * natural_weight(mode) + l_offset
            - hk_compensation(c),
        c,
        h,
    )
}

/// Moves a colour's lightness away from the background until it is told apart
/// from `others`. Muted or yellow palettes can land accents almost on top of each
/// other, because gamut mapping removes the chroma that separated them. Moving
/// away from the background can only raise contrast, so this stays legible.
fn separate(color: Color, others: &[Color], sign: f64) -> Color {
    const MIN_GAP: f64 = 0.045;
    let mut lch = color.to_lch();
    let mut out = color;
    while others.iter().any(|o| out.delta_e(*o) < MIN_GAP) && (0.15..=0.92).contains(&lch.l) {
        lch.l += 0.006 * sign;
        out = lch.to_color();
    }
    out
}

fn accents(
    a: &Analysis,
    scheme: Scheme,
    detected: bool,
    mode: Mode,
    style: Style,
    n: &Neutrals,
) -> (Accents, bool, bool) {
    let (sec, ter) = accent_hues(a, scheme, detected);
    let sign = if mode == Mode::Dark { 1.0 } else { -1.0 };
    // A monochromatic scheme has barely any hue contrast to work with, so its
    // roles have to separate through lightness instead.
    let (sec_l, ter_l) = if scheme == Scheme::Mono {
        (0.06 * sign, -0.07 * sign)
    } else {
        (0.015 * sign, 0.03 * sign)
    };

    let finish = |p: Oklch| ensure_contrast(p, n.bg, 4.5).to_color();
    let primary = finish(accent(a.key_hue, a.key_chroma, 1.0, 0.0, mode, style));
    let secondary = finish(accent(sec.hue, sec.chroma, 0.8, sec_l, mode, style));
    let secondary = separate(secondary, &[primary], sign);
    let tertiary = finish(accent(ter.hue, ter.chroma, 0.65, ter_l, mode, style));
    let tertiary = separate(tertiary, &[primary, secondary], sign);

    let dark = Oklch::new(0.20, 0.02, a.key_hue).to_color();
    let light = Oklch::new(0.97, 0.01, a.key_hue).to_color();
    let on_primary = if primary.contrast(dark) >= primary.contrast(light) {
        dark
    } else {
        light
    };

    let plch = primary.to_lch();
    let (container_l, container_c) = match mode {
        Mode::Dark => (0.33, 0.5),
        Mode::Light => (0.925, 0.3),
    };
    let primary_container = ensure_contrast(
        Oklch::new(container_l, plch.c * container_c, plch.h),
        n.text,
        4.5,
    )
    .to_color();

    (
        Accents {
            primary,
            on_primary,
            primary_container,
            secondary,
            tertiary,
        },
        sec.from_image,
        ter.from_image,
    )
}

/// ANSI hue anchors. Terminals carry meaning here (red means failure), so these
/// must stay recognisable whatever the image looks like.
const RED: f64 = 27.0;
const GREEN: f64 = 150.0;
const YELLOW: f64 = 95.0;
/// A yellow dark enough to read on a light page is always olive. Moving towards
/// amber gives an ochre instead, which still reads as a warning.
const YELLOW_ON_LIGHT: f64 = 82.0;
/// Dark on a light page, a plain green and that ochre both look olive to a
/// deuteranope, and there is no lightness left to separate them. A bluish green
/// (as in the Okabe-Ito palette) looks greyish instead, so hue can tell them apart.
const GREEN_ON_LIGHT: f64 = 162.0;
const BLUE: f64 = 255.0;
const MAGENTA: f64 = 330.0;
const CYAN: f64 = 200.0;
const ORANGE: f64 = 58.0;

/// Nudges an anchor towards a nearby theme hue. The shift is bounded, which is
/// how the set stays in the theme's family without losing its meaning.
fn tune_anchor(anchor: f64, theme_hues: &[f64]) -> f64 {
    let nearest = theme_hues
        .iter()
        .map(|h| hue_diff(anchor, *h))
        .filter(|d| d.abs() <= 35.0)
        .min_by(|a, b| a.abs().total_cmp(&b.abs()));
    match nearest {
        Some(d) => norm_hue(anchor + (d * 0.5).clamp(-15.0, 15.0)),
        None => anchor,
    }
}

/// Keeps a status colour from being mistaken for a brand colour: when the theme's
/// primary is a red, a plain button must not read as an error. Finds the smallest
/// change, staying near the semantic anchor so that red still means red.
///
/// `rivals` are the status colours already settled, which this one must stay
/// distinct from under protanopia and deuteranopia. `grounds` are the backgrounds
/// it must be legible on. Both are part of the search rather than a later check,
/// because moving a colour away from the accents can break either of them.
fn detach_semantic(
    color: Color,
    anchor: f64,
    accents: &[Color],
    rivals: &[Color],
    grounds: &[Color],
    boost_limit: Option<f64>,
    mode: Mode,
) -> Color {
    let clear = |c: Color| {
        grounds.iter().all(|g| c.contrast(*g) >= STATUS_CONTRAST)
            && accents.iter().enumerate().all(|(i, a)| {
                let gap = if i == 0 {
                    SEMANTIC_GAP
                } else {
                    support_gap(mode)
                };
                c.delta_e(*a) >= gap
            })
            && rivals.iter().all(|r| {
                [Cvd::Protan, Cvd::Deutan]
                    .iter()
                    .all(|d| c.simulate(*d).delta_e(r.simulate(*d)) >= CVD_GAP)
            })
    };
    // A status may darken freely but barely lighten: raising lightness turns a red
    // into pastel pink and a green into near white, and neither reads as a status.
    // Hence the order of preference: within the limit and near the anchor, then
    // within the limit with a wider hue shift (a brick red still reads as an error,
    // a pale pink doesn't), and only then past the limit.
    const NARROW_HUE: f64 = 16.0;
    const MAX_LIGHTENING: f64 = 0.08;
    let ceiling = (color.to_lch().l + MAX_LIGHTENING).min(0.86);
    let natural = |c: Color| (0.38..=ceiling).contains(&c.to_lch().l);
    if clear(color) && natural(color) {
        return color;
    }

    let base = color.to_lch();
    let mut chromas = vec![base.c, base.c * 1.15, base.c * 1.3];
    if let Some(limit) = boost_limit {
        chromas.extend(
            [0.8, 0.9, 1.0]
                .map(|t| limit * t)
                .into_iter()
                .filter(|c| *c > base.c * 1.3),
        );
    }
    // With a boost allowed, chroma is the cheap way out: it keeps the colour's hue.
    let chroma_weight = if boost_limit.is_some() { 0.1 } else { 0.2 };

    let mut best: Option<(f64, Color)> = None;
    for dl in -18..=18 {
        for dh in [-20.0, -16.0, -8.0, 0.0, 8.0, 16.0, 20.0_f64] {
            for chroma in &chromas {
                let dl = dl as f64 * 0.02;
                let c = Oklch::new(base.l + dl, *chroma, anchor + dh).to_color();
                if !clear(c) {
                    continue;
                }
                let tier = if natural(c) { 0.0 } else { 2.0 }
                    + if dh.abs() > NARROW_HUE { 1.0 } else { 0.0 };
                let cost = tier * 10.0
                    + dl.abs()
                    + dh.abs() / 100.0
                    + (chroma / base.c - 1.0) * chroma_weight;
                if best.is_none_or(|b| cost < b.0) {
                    best = Some((cost, c));
                }
            }
        }
    }
    best.map_or(color, |b| b.1)
}

/// How far `error` may exceed the style's chroma cap when it has to stand apart
/// from a reddish theme. It is the one colour allowed to: a clear red matters more
/// there than restraint, and its small area keeps the theme calm overall.
fn error_boost_limit(style: Style) -> f64 {
    (style.chroma_cap() * 1.36).min(0.16)
}

/// Minimum ΔE between two status colours as seen by a dichromat. Slightly above
/// the audit's 0.05 so that 8-bit rounding can't drop a colour below it.
const CVD_GAP: f64 = 0.055;

/// Minimum contrast of a status colour on the backgrounds it appears on.
pub const STATUS_CONTRAST: f64 = 4.5;

/// Minimum ΔE between a status colour and the primary. A status that looks like the
/// main action is the costly confusion.
pub const SEMANTIC_GAP: f64 = 0.07;

/// Minimum ΔE between a status colour and the supporting accents. Looking a bit like
/// those matters less, and in a light theme there is too little room to demand more:
/// with an ochre secondary, a warning kept at 0.07 flees to near black.
pub fn support_gap(mode: Mode) -> f64 {
    match mode {
        Mode::Dark => SEMANTIC_GAP,
        Mode::Light => 0.045,
    }
}

fn yellow_anchor(mode: Mode) -> f64 {
    match mode {
        Mode::Dark => YELLOW,
        Mode::Light => YELLOW_ON_LIGHT,
    }
}

fn green_anchor(mode: Mode) -> f64 {
    match mode {
        Mode::Dark => GREEN,
        Mode::Light => GREEN_ON_LIGHT,
    }
}

struct Tones<'a> {
    mode: Mode,
    style: Style,
    bg: Color,
    theme_hues: &'a [f64],
}

impl Tones<'_> {
    /// Shared L and C make the set feel like one family instead of six unrelated
    /// colours, a main cause of the clown effect in terminals. Green sits lighter
    /// and red darker, because dichromats tell those two apart by lightness.
    fn semantic(&self, anchor: f64, bright: bool) -> Color {
        let h = tune_anchor(anchor, self.theme_hues);
        // Dichromats tell these apart by lightness, so each gets its own step. A light
        // theme only has about 0.15 of L between the contrast limit and muddiness. A
        // fixed ladder in natural order (deep red, mid green, ochre on top) leaves
        // each colour room; otherwise the last one settled gets pushed to near black.
        let (bright_shift, red_shift, green_shift) = match self.mode {
            Mode::Dark => (0.06, -0.05, 0.05),
            Mode::Light => (-0.04, -0.08, -0.02),
        };
        let l_shift = if anchor == green_anchor(self.mode) {
            green_shift
        } else if anchor == RED {
            red_shift
        } else {
            0.0
        };
        let mut c = self.base_chroma() * neon_factor(h);
        let mut l = base_accent_lightness(self.mode)
            + natural_lightness(h) * 0.8 * natural_weight(self.mode)
            + l_shift;
        if bright {
            c *= 1.1;
            l += bright_shift;
        }
        ensure_contrast(Oklch::new(l, c, h), self.bg, 4.5).to_color()
    }

    fn base_chroma(&self) -> f64 {
        (self.style.chroma_cap().max(0.075) * 0.9).max(0.065) * light_gain(self.mode)
    }

    fn mid_tone(&self, h: f64, c_scale: f64, l_shift: f64) -> Color {
        let c = self.base_chroma() * c_scale;
        let l = base_accent_lightness(self.mode) + l_shift;
        ensure_contrast(Oklch::new(l, c, h), self.bg, 4.5).to_color()
    }
}

pub fn build(a: &Analysis, o: &Options) -> Palette {
    let mode = o.mode.unwrap_or(if a.mean_lightness > 0.62 {
        Mode::Light
    } else {
        Mode::Dark
    });
    let scheme = o.scheme.unwrap_or(a.fit.scheme);
    let n = neutrals(a, mode, o.style);
    let (acc, sec_img, ter_img) = accents(a, scheme, o.scheme.is_none(), mode, o.style, &n);

    let hues = [
        acc.primary.to_lch().h,
        acc.secondary.to_lch().h,
        acc.tertiary.to_lch().h,
    ];
    let t = Tones {
        mode,
        style: o.style,
        bg: n.bg,
        theme_hues: &hues,
    };
    let yellow = yellow_anchor(mode);
    let green = green_anchor(mode);
    let order = [RED, green, yellow, BLUE, MAGENTA, CYAN];
    let normal = order.map(|h| t.semantic(h, false));
    let bright = order.map(|h| t.semantic(h, true));
    let bright_black = ensure_contrast(n.muted.to_lch(), n.bg, 3.0).to_color();

    let ansi = [
        n.overlay,
        normal[0],
        normal[1],
        normal[2],
        normal[3],
        normal[4],
        normal[5],
        n.subtext,
        bright_black,
        bright[0],
        bright[1],
        bright[2],
        bright[3],
        bright[4],
        bright[5],
        n.text,
    ];

    let orange = t.semantic(ORANGE, false);
    let brown = {
        let shift = if mode == Mode::Dark { -0.08 } else { 0.0 };
        t.mid_tone(tune_anchor(ORANGE, &hues), 0.65, shift)
    };
    let base16 = [
        n.bg,
        // base01 must step from the page towards the text. A light theme's surface
        // goes the other way, towards white.
        if mode == Mode::Dark {
            n.surface
        } else {
            n.crust
        },
        n.overlay,
        n.muted,
        n.subtext,
        n.text,
        mix(n.text, n.crust, if mode == Mode::Dark { 0.0 } else { 0.25 }),
        Oklch::new(
            if mode == Mode::Dark { 0.95 } else { 0.20 },
            0.008,
            a.atmosphere_hue,
        )
        .to_color(),
        normal[0],
        orange,
        normal[2],
        normal[1],
        normal[5],
        normal[3],
        normal[4],
        brown,
    ];

    Palette {
        meta: Meta {
            source: o.source.clone(),
            mode,
            style: o.style,
            scheme,
            template: o.scheme.is_none().then_some(a.fit),
            secondary_from_image: sec_img,
            tertiary_from_image: ter_img,
            achromatic_source: a.achromatic,
            image_colors: a
                .clusters
                .iter()
                .map(|c| ImageColor {
                    color: c.lch().to_color(),
                    weight: c.weight,
                })
                .collect(),
        },
        neutrals: n,
        accents: acc,
        semantic: {
            let brand = [acc.primary, acc.secondary, acc.tertiary];
            let grounds = [n.bg, n.surface];
            // Each colour is settled against the ones before it. Error and success
            // go first because confusing them has the worst consequences.
            let error = detach_semantic(
                normal[0],
                RED,
                &brand,
                &[],
                &grounds,
                Some(error_boost_limit(o.style)),
                mode,
            );
            let success = detach_semantic(normal[1], green, &brand, &[error], &grounds, None, mode);
            let warning = detach_semantic(
                normal[2],
                yellow,
                &brand,
                &[error, success],
                &grounds,
                None,
                mode,
            );
            let info = detach_semantic(
                normal[3],
                BLUE,
                &brand,
                &[error, success, warning],
                &grounds,
                None,
                mode,
            );
            Semantic {
                error,
                warning,
                success,
                info,
            }
        },
        ansi,
        base16,
    }
}

pub fn mix(a: Color, b: Color, t: f64) -> Color {
    let (x, y) = (a.to_lab(), b.to_lab());
    crate::color::Oklab {
        l: x.l + (y.l - x.l) * t,
        a: x.a + (y.a - x.a) * t,
        b: x.b + (y.b - x.b) * t,
    }
    .to_lch()
    .to_color()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neon_factor_lowers_only_the_loud_hues() {
        assert!(neon_factor(345.0) < 0.85);
        assert!(neon_factor(130.0) < 0.85);
        assert!(neon_factor(265.0) > 0.95);
    }

    #[test]
    fn anchors_stay_near_their_meaning() {
        assert_eq!(tune_anchor(RED, &[200.0]), RED);
        let tuned = tune_anchor(RED, &[50.0]);
        assert!(tuned > RED && tuned <= RED + 15.0);
    }

    #[test]
    fn ensure_contrast_reaches_target_on_both_grounds() {
        let dark = Color::new(20, 20, 24);
        let light = Color::new(245, 245, 240);
        for bg in [dark, light] {
            let c = ensure_contrast(Oklch::new(0.5, 0.1, 250.0), bg, 4.5).to_color();
            assert!(c.contrast(bg) >= 4.5, "{} on {}", c.hex(), bg.hex());
        }
    }
}

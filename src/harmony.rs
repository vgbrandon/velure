//! Colour-theory analysis of the image: Matsuda harmonic templates, key colour,
//! atmosphere hue and the hue targets of each classical scheme.

use clap::ValueEnum;
use serde::Serialize;

use crate::color::{hue_diff, norm_hue};
use crate::extract::Cluster;

/// Below this OKLCH chroma a colour reads as grey and carries no hue.
pub const CHROMATIC_MIN: f64 = 0.02;
/// Mean arc (degrees) a template may leave the image's hues outside its sectors
/// and still count as describing the image.
const FIT_TOLERANCE: f64 = 6.0;
/// Hue used when the image is truly greyscale: a slate, the least assertive tint.
const ACHROMATIC_FALLBACK_HUE: f64 = 250.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    #[value(name = "mono")]
    Mono,
    #[value(name = "analoga")]
    Analogous,
    #[value(name = "complementaria")]
    Complementary,
    #[value(name = "dividida")]
    Split,
    #[value(name = "ortogonal")]
    Orthogonal,
    #[value(name = "triadica")]
    Triadic,
}

impl Scheme {
    pub fn label(self) -> &'static str {
        match self {
            Scheme::Mono => "monocromática",
            Scheme::Analogous => "análoga",
            Scheme::Complementary => "complementaria",
            Scheme::Split => "complementaria dividida",
            Scheme::Orthogonal => "ortogonal",
            Scheme::Triadic => "triádica",
        }
    }

    /// Hue offsets from the primary for secondary and tertiary.
    pub fn offsets(self) -> (f64, f64) {
        match self {
            Scheme::Mono => (12.0, -12.0),
            Scheme::Analogous => (30.0, -30.0),
            Scheme::Complementary => (180.0, 30.0),
            Scheme::Split => (150.0, 210.0),
            Scheme::Orthogonal => (90.0, -30.0),
            Scheme::Triadic => (120.0, 240.0),
        }
    }
}

/// A Matsuda template: `(centre offset, width)` sectors on the hue wheel.
struct Template {
    name: &'static str,
    sectors: &'static [(f64, f64)],
    scheme: Scheme,
}

/// Ordered from narrowest to widest: the first that fits is the simplest
/// harmony that honestly describes the image.
const TEMPLATES: [Template; 7] = [
    Template {
        name: "i",
        sectors: &[(0.0, 18.0)],
        scheme: Scheme::Mono,
    },
    Template {
        name: "I",
        sectors: &[(0.0, 18.0), (180.0, 18.0)],
        scheme: Scheme::Complementary,
    },
    Template {
        name: "V",
        sectors: &[(0.0, 93.6)],
        scheme: Scheme::Analogous,
    },
    Template {
        name: "L",
        sectors: &[(0.0, 18.0), (90.0, 79.2)],
        scheme: Scheme::Orthogonal,
    },
    Template {
        name: "Y",
        sectors: &[(0.0, 93.6), (180.0, 18.0)],
        scheme: Scheme::Split,
    },
    Template {
        name: "T",
        sectors: &[(0.0, 180.0)],
        scheme: Scheme::Analogous,
    },
    Template {
        name: "X",
        sectors: &[(0.0, 93.6), (180.0, 93.6)],
        scheme: Scheme::Complementary,
    },
];

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Fit {
    pub template: &'static str,
    pub rotation: f64,
    /// Mean degrees of hue left outside the template, weighted by area × chroma.
    pub cost: f64,
    pub scheme: Scheme,
}

impl Fit {
    /// Degrees by which a hue falls outside the fitted template (0 if inside).
    pub fn distance(&self, h: f64) -> f64 {
        let sectors = TEMPLATES
            .iter()
            .find(|t| t.name == self.template)
            .map_or(&[][..], |t| t.sectors);
        sector_distance(h, self.rotation, sectors)
    }
}

/// Fits every template at every rotation (Cohen-Or et al., 2006) and returns the
/// narrowest within tolerance, or the lowest-cost one if none qualifies.
pub fn fit_template(clusters: &[Cluster]) -> Fit {
    let samples: Vec<(f64, f64)> = clusters
        .iter()
        .map(|c| (c.lch(), c.weight))
        .filter(|(lch, _)| lch.c >= CHROMATIC_MIN)
        .map(|(lch, w)| (lch.h, w * lch.c))
        .collect();
    let total: f64 = samples.iter().map(|s| s.1).sum();

    let mut best: Option<Fit> = None;
    for t in &TEMPLATES {
        let mut fit = Fit {
            template: t.name,
            rotation: 0.0,
            cost: f64::INFINITY,
            scheme: t.scheme,
        };
        for rot in 0..360 {
            let rot = rot as f64;
            let cost = if total > 0.0 {
                samples
                    .iter()
                    .map(|(h, w)| sector_distance(*h, rot, t.sectors) * w)
                    .sum::<f64>()
                    / total
            } else {
                0.0
            };
            if cost < fit.cost {
                fit.cost = cost;
                fit.rotation = rot;
            }
        }
        if fit.cost <= FIT_TOLERANCE {
            return fit;
        }
        if best.is_none_or(|b| fit.cost < b.cost) {
            best = Some(fit);
        }
    }
    best.expect("TEMPLATES is non-empty")
}

fn sector_distance(h: f64, rot: f64, sectors: &[(f64, f64)]) -> f64 {
    sectors
        .iter()
        .map(|(center, width)| (hue_diff(h, rot + center).abs() - width / 2.0).max(0.0))
        .fold(f64::INFINITY, f64::min)
}

#[derive(Clone, Debug)]
pub struct Analysis {
    pub clusters: Vec<Cluster>,
    /// Hue that gives the image its identity. Becomes the primary.
    pub key_hue: f64,
    pub key_chroma: f64,
    /// Hue that dominates by area. Tints the neutrals.
    pub atmosphere_hue: f64,
    pub atmosphere_chroma: f64,
    pub mean_lightness: f64,
    pub achromatic: bool,
    pub fit: Fit,
}

pub fn analyze(clusters: Vec<Cluster>) -> Analysis {
    let mean_lightness = clusters.iter().map(|c| c.lab.l * c.weight).sum();
    let fit = fit_template(&clusters);
    let chromatic_area: f64 = clusters
        .iter()
        .filter(|c| c.lch().c >= CHROMATIC_MIN)
        .map(|c| c.weight)
        .sum();
    let achromatic = chromatic_area < 0.02;

    // Favours colours that are visible and carry hue, near mid-lightness where
    // hue is best perceived. The sqrt keeps area from drowning out a small
    // accent, and chroma is capped so a neon speck doesn't beat the main subject.
    let key = clusters
        .iter()
        .max_by(|a, b| key_score(a).total_cmp(&key_score(b)));
    let (key_hue, key_chroma) = match key {
        Some(c) if !achromatic => (c.lch().h, c.lch().c),
        Some(c) if c.lch().c > 0.005 => (c.lch().h, 0.04),
        _ => (ACHROMATIC_FALLBACK_HUE, 0.04),
    };

    let (atmosphere_hue, atmosphere_chroma) = if achromatic {
        (key_hue, 0.01)
    } else {
        atmosphere(&clusters).unwrap_or((key_hue, key_chroma))
    };

    Analysis {
        clusters,
        key_hue,
        key_chroma,
        atmosphere_hue,
        atmosphere_chroma,
        mean_lightness,
        achromatic,
        fit,
    }
}

fn key_score(c: &Cluster) -> f64 {
    let lch = c.lch();
    c.weight.sqrt() * lch.c.min(0.16) * (-((lch.l - 0.62) / 0.30).powi(2)).exp()
}

/// Peak of a 30°-smoothed hue histogram. A circular mean would be meaningless
/// on opposing hues: orange and blue would average to a magenta not in the image.
fn atmosphere(clusters: &[Cluster]) -> Option<(f64, f64)> {
    let chromatic: Vec<_> = clusters
        .iter()
        .map(|c| (c.lch(), c.weight))
        .filter(|(lch, _)| lch.c >= CHROMATIC_MIN)
        .collect();
    let score = |h: f64| -> f64 {
        chromatic
            .iter()
            .filter(|(lch, _)| hue_diff(lch.h, h).abs() <= 15.0)
            .map(|(_, w)| *w)
            .sum()
    };
    let peak = chromatic
        .iter()
        .map(|(lch, _)| lch.h)
        .max_by(|a, b| score(*a).total_cmp(&score(*b)))?;
    let (mut sum, mut area) = (0.0, 0.0);
    for (lch, w) in &chromatic {
        if hue_diff(lch.h, peak).abs() <= 15.0 {
            sum += lch.c * w;
            area += w;
        }
    }
    Some((norm_hue(peak), sum / area))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Oklch;

    fn cluster(h: f64, c: f64, weight: f64) -> Cluster {
        let rad = h.to_radians();
        Cluster {
            lab: crate::color::Oklab {
                l: 0.6,
                a: c * rad.cos(),
                b: c * rad.sin(),
            },
            weight,
        }
    }

    #[test]
    fn single_hue_fits_monochromatic() {
        let f = fit_template(&[cluster(200.0, 0.1, 0.6), cluster(204.0, 0.08, 0.4)]);
        assert_eq!(f.template, "i");
        assert_eq!(f.scheme, Scheme::Mono);
    }

    #[test]
    fn opposing_hues_fit_complementary() {
        let f = fit_template(&[cluster(60.0, 0.1, 0.5), cluster(240.0, 0.1, 0.5)]);
        assert_eq!(f.template, "I");
        assert_eq!(f.scheme, Scheme::Complementary);
    }

    #[test]
    fn close_hues_fit_analogous() {
        let f = fit_template(&[
            cluster(100.0, 0.1, 0.4),
            cluster(130.0, 0.1, 0.3),
            cluster(160.0, 0.1, 0.3),
        ]);
        assert_eq!(f.template, "V");
        assert_eq!(f.scheme, Scheme::Analogous);
    }

    #[test]
    fn specks_do_not_break_a_mono_image() {
        // A tiny, faint speck of opposite hue must not change the harmony.
        let f = fit_template(&[cluster(30.0, 0.12, 0.995), cluster(210.0, 0.03, 0.005)]);
        assert_eq!(f.scheme, Scheme::Mono);
    }

    #[test]
    fn greyscale_falls_back_to_slate() {
        let a = analyze(vec![Cluster {
            lab: crate::color::Oklab {
                l: 0.5,
                a: 0.0,
                b: 0.0,
            },
            weight: 1.0,
        }]);
        assert!(a.achromatic);
        assert_eq!(a.key_hue, ACHROMATIC_FALLBACK_HUE);
    }

    #[test]
    fn atmosphere_is_a_real_hue_of_the_image() {
        let a = analyze(vec![cluster(60.0, 0.1, 0.6), cluster(240.0, 0.1, 0.4)]);
        assert!(hue_diff(a.atmosphere_hue, 60.0).abs() < 1.0);
    }

    #[test]
    fn oklch_constructor_normalises_hue() {
        assert_eq!(Oklch::new(0.5, 0.1, 370.0).h, 10.0);
    }
}

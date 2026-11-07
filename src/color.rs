//! Color spaces and perceptual metrics: sRGB, OKLab/OKLCH, WCAG 2, APCA and
//! colour-vision-deficiency simulation.

use serde::{Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

/// Cylindrical OKLab. `h` is in degrees, in `[0, 360)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

/// A final, displayable 8-bit sRGB colour. Every audit runs on this type, so
/// what gets checked is exactly what gets exported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

pub fn srgb_to_linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(v: f64) -> f64 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn linear_to_oklab([r, g, b]: [f64; 3]) -> Oklab {
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
    let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
    Oklab {
        l: 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        a: 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        b: 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    }
}

fn oklab_to_linear(c: Oklab) -> [f64; 3] {
    let l = c.l + 0.3963377774 * c.a + 0.2158037573 * c.b;
    let m = c.l - 0.1055613458 * c.a - 0.0638541728 * c.b;
    let s = c.l - 0.0894841775 * c.a - 1.2914855480 * c.b;
    let (l, m, s) = (l * l * l, m * m * m, s * s * s);
    [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ]
}

impl Oklab {
    pub fn to_lch(self) -> Oklch {
        Oklch {
            l: self.l,
            c: self.a.hypot(self.b),
            h: norm_hue(self.b.atan2(self.a).to_degrees()),
        }
    }

    pub fn delta_e(self, o: Oklab) -> f64 {
        ((self.l - o.l).powi(2) + (self.a - o.a).powi(2) + (self.b - o.b).powi(2)).sqrt()
    }
}

impl Oklch {
    pub fn new(l: f64, c: f64, h: f64) -> Self {
        Self {
            l,
            c,
            h: norm_hue(h),
        }
    }

    fn to_lab(self) -> Oklab {
        let (s, c) = self.h.to_radians().sin_cos();
        Oklab {
            l: self.l,
            a: self.c * c,
            b: self.c * s,
        }
    }

    fn in_gamut(self) -> bool {
        oklab_to_linear(self.to_lab())
            .iter()
            .all(|v| (-1e-4..=1.0 + 1e-4).contains(v))
    }

    /// Gamut-maps to sRGB by reducing chroma only, which preserves the
    /// lightness and hue decisions made upstream (CSS Color 4 style).
    pub fn to_color(self) -> Color {
        let mut p = Oklch {
            l: self.l.clamp(0.0, 1.0),
            ..self
        };
        if !p.in_gamut() {
            let (mut lo, mut hi) = (0.0, p.c);
            for _ in 0..24 {
                let mid = (lo + hi) / 2.0;
                if (Oklch { c: mid, ..p }).in_gamut() {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            p.c = lo;
        }
        let [r, g, b] = oklab_to_linear(p.to_lab());
        let q = |v: f64| (linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8;
        Color {
            r: q(r),
            g: q(g),
            b: q(b),
        }
    }
}

pub fn norm_hue(h: f64) -> f64 {
    h.rem_euclid(360.0)
}

/// Signed shortest arc from `a` to `b`, in `[-180, 180)`.
pub fn hue_diff(a: f64, b: f64) -> f64 {
    (b - a + 180.0).rem_euclid(360.0) - 180.0
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Hex without the leading `#`, as Hyprland and fish expect it.
    pub fn bare(self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    fn linear(self) -> [f64; 3] {
        [self.r, self.g, self.b].map(|v| srgb_to_linear(v as f64 / 255.0))
    }

    pub fn to_lab(self) -> Oklab {
        linear_to_oklab(self.linear())
    }

    pub fn to_lch(self) -> Oklch {
        self.to_lab().to_lch()
    }

    pub fn delta_e(self, o: Color) -> f64 {
        self.to_lab().delta_e(o.to_lab())
    }

    /// WCAG 2 relative luminance.
    pub fn luminance(self) -> f64 {
        let [r, g, b] = self.linear();
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// WCAG 2 contrast ratio, in `[1, 21]`.
    pub fn contrast(self, o: Color) -> f64 {
        let (a, b) = (self.luminance(), o.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// APCA 0.0.98G lightness contrast of `self` as text on background `bg`.
    /// Returns the absolute value of Lc, in `[0, ~108]`.
    pub fn apca(self, bg: Color) -> f64 {
        let y = |c: Color| {
            let f = |v: u8| (v as f64 / 255.0).powf(2.4);
            let raw = 0.2126729 * f(c.r) + 0.7151522 * f(c.g) + 0.0721750 * f(c.b);
            if raw < 0.022 {
                raw + (0.022 - raw).powf(1.414)
            } else {
                raw
            }
        };
        let (t, b) = (y(self), y(bg));
        if (b - t).abs() < 0.0005 {
            return 0.0;
        }
        let s = if b > t {
            (b.powf(0.56) - t.powf(0.57)) * 1.14
        } else {
            (b.powf(0.65) - t.powf(0.62)) * 1.14
        };
        if s.abs() < 0.1 {
            0.0
        } else {
            (s.abs() - 0.027) * 100.0
        }
    }

    /// Simulates dichromacy with the Machado et al. (2009) severity-1.0
    /// matrices, which are defined on linear RGB.
    pub fn simulate(self, cvd: Cvd) -> Color {
        let m = match cvd {
            Cvd::Protan => [
                [0.152286, 1.052583, -0.204868],
                [0.114503, 0.786281, 0.099216],
                [-0.003882, -0.048116, 1.051998],
            ],
            Cvd::Deutan => [
                [0.367322, 0.860646, -0.227968],
                [0.280085, 0.672501, 0.047413],
                [-0.011820, 0.042940, 0.968881],
            ],
        };
        let c = self.linear();
        let q = |row: [f64; 3]| {
            let v = row[0] * c[0] + row[1] * c[1] + row[2] * c[2];
            (linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8
        };
        Color::new(q(m[0]), q(m[1]), q(m[2]))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Cvd {
    Protan,
    Deutan,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklab_reference_values() {
        // Published reference values for sRGB red and white.
        let red = Color::new(255, 0, 0).to_lab();
        assert!((red.l - 0.62796).abs() < 1e-3);
        assert!((red.a - 0.22486).abs() < 1e-3);
        assert!((red.b - 0.12585).abs() < 1e-3);
        let white = Color::new(255, 255, 255).to_lab();
        assert!((white.l - 1.0).abs() < 1e-3 && white.a.abs() < 1e-3 && white.b.abs() < 1e-3);
    }

    #[test]
    fn roundtrip_is_exact_for_8_bit() {
        for c in [
            Color::new(12, 34, 56),
            Color::new(200, 100, 50),
            Color::new(255, 255, 0),
            Color::new(1, 1, 1),
        ] {
            assert_eq!(c.to_lch().to_color(), c);
        }
    }

    #[test]
    fn gamut_mapping_keeps_lightness_and_hue() {
        let out = Oklch::new(0.7, 0.4, 150.0).to_color().to_lch();
        assert!((out.l - 0.7).abs() < 0.01);
        assert!(hue_diff(out.h, 150.0).abs() < 2.0);
        assert!(out.c < 0.4);
    }

    #[test]
    fn wcag_extremes() {
        let (k, w) = (Color::new(0, 0, 0), Color::new(255, 255, 255));
        assert!((k.contrast(w) - 21.0).abs() < 1e-9);
        assert!((w.contrast(w) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn apca_reference_values() {
        // Reference pair from the APCA documentation: #888 on #fff is Lc 63.06.
        let (grey, white) = (Color::new(0x88, 0x88, 0x88), Color::new(255, 255, 255));
        assert!((grey.apca(white) - 63.06).abs() < 0.5);
        // Black text on white and white text on black, known ~106 and ~108.
        assert!((Color::new(0, 0, 0).apca(white) - 106.0).abs() < 1.0);
        assert!((white.apca(Color::new(0, 0, 0)) - 107.9).abs() < 1.0);
    }

    #[test]
    fn hue_diff_takes_the_short_way() {
        assert_eq!(hue_diff(350.0, 10.0), 20.0);
        assert_eq!(hue_diff(10.0, 350.0), -20.0);
    }

    #[test]
    fn deutan_collapses_pure_red_green_axis() {
        let (r, g) = (Color::new(200, 40, 40), Color::new(40, 170, 40));
        assert!(r.simulate(Cvd::Deutan).delta_e(g.simulate(Cvd::Deutan)) < r.delta_e(g));
    }
}

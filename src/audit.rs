//! Audits the final 8-bit palette against legibility and restraint criteria.

use serde::Serialize;

use crate::color::{Color, Cvd, hue_diff};
use crate::palette::{Palette, SEMANTIC_GAP, STATUS_CONTRAST, neon_factor, support_gap};

#[derive(Clone, Debug, Serialize)]
pub struct Check {
    pub name: String,
    pub value: String,
    pub passed: bool,
}

/// The classic 60-30-10 split: background, surfaces, accents.
const AREA_BG: f64 = 0.60;
const AREA_SURFACE: f64 = 0.30;
const AREA_ACCENT: f64 = 0.10;
/// Area-weighted chroma ceiling for a theme to stay calm in everyday use. It
/// applies the Moon-Spencer idea that small strong areas balance large weak ones.
const MAX_CHROMA_LOAD: f64 = 0.03;
/// Tolerance for the 8-bit rounding of a colour against its chroma cap.
const CAP_ROUNDING: f64 = 0.004;

fn contrast(name: &str, fg: Color, bg: Color, min: f64) -> Check {
    let ratio = fg.contrast(bg);
    Check {
        name: name.into(),
        value: format!("{ratio:.2}:1 (mín {min})"),
        passed: ratio >= min,
    }
}

pub fn run(p: &Palette) -> Vec<Check> {
    let (n, a, s) = (&p.neutrals, &p.accents, &p.semantic);
    let mut out = vec![
        contrast("Texto sobre fondo (AAA)", n.text, n.bg, 7.0),
        contrast("Texto sobre superficie (AAA)", n.text, n.surface, 7.0),
        contrast("Subtexto sobre fondo (AAA)", n.subtext, n.bg, 7.0),
        contrast("Texto atenuado sobre fondo (AA)", n.muted, n.bg, 4.5),
        contrast("Primario sobre fondo (AA)", a.primary, n.bg, 4.5),
        contrast("Secundario sobre fondo (AA)", a.secondary, n.bg, 4.5),
        contrast("Terciario sobre fondo (AA)", a.tertiary, n.bg, 4.5),
        contrast("Texto sobre primario (AA)", a.on_primary, a.primary, 4.5),
        contrast(
            "Texto sobre contenedor (AA)",
            n.text,
            a.primary_container,
            4.5,
        ),
    ];

    let lc = n.text.apca(n.bg);
    out.push(Check {
        name: "APCA del cuerpo de texto".into(),
        value: format!("Lc {lc:.0} (mín 75)"),
        passed: lc >= 75.0,
    });

    let worst_ansi = p
        .ansi
        .iter()
        .enumerate()
        .filter(|(i, _)| ![0, 8].contains(i))
        .map(|(_, c)| c.contrast(n.bg))
        .fold(f64::INFINITY, f64::min);
    out.push(Check {
        name: "Colores ANSI sobre fondo (AA)".into(),
        value: format!("peor {worst_ansi:.2}:1 (mín 4.5)"),
        passed: worst_ansi >= 4.5,
    });
    out.push(contrast(
        "ANSI gris brillante / comentarios",
        p.ansi[8],
        n.bg,
        3.0,
    ));

    let ladder = [
        n.crust, n.bg, n.surface, n.overlay, n.border, n.muted, n.subtext, n.text,
    ];
    let min_step = ladder
        .windows(2)
        .map(|w| w[0].to_lch().l - w[1].to_lch().l)
        .map(f64::abs)
        .fold(f64::INFINITY, f64::min);
    out.push(Check {
        name: "Capas de neutros distinguibles".into(),
        value: format!("paso mínimo ΔL {min_step:.3} (mín 0.015)"),
        passed: min_step >= 0.015,
    });

    let cap = p.meta.style.chroma_cap_for(p.meta.mode);
    let over_cap = [a.primary, a.secondary, a.tertiary]
        .iter()
        .map(|c| c.to_lch())
        .map(|l| l.c / (cap * neon_factor(l.h)))
        .fold(0.0, f64::max);
    out.push(Check {
        name: format!("Croma dentro del estilo {}", p.meta.style.label()),
        value: format!("{:.0}% del tope", over_cap * 100.0),
        passed: over_cap <= 1.0 + CAP_ROUNDING / cap,
    });

    let load = AREA_BG * n.bg.to_lch().c
        + AREA_SURFACE * n.surface.to_lch().c
        + AREA_ACCENT * a.primary.to_lch().c;
    out.push(Check {
        name: "Carga cromática 60-30-10".into(),
        value: format!("{load:.3} (máx {MAX_CHROMA_LOAD})"),
        passed: load <= MAX_CHROMA_LOAD,
    });

    let hue_families = hue_families(&[a.primary, a.secondary, a.tertiary]);
    out.push(Check {
        name: "Familias de tono en la interfaz".into(),
        value: format!("{hue_families} (máx 3)"),
        passed: hue_families <= 3,
    });

    let accent_gap = [
        (a.primary, a.secondary),
        (a.primary, a.tertiary),
        (a.secondary, a.tertiary),
    ]
    .iter()
    .map(|(x, y)| x.delta_e(*y))
    .fold(f64::INFINITY, f64::min);
    out.push(Check {
        name: "Acentos distinguibles entre sí".into(),
        value: format!("ΔE {accent_gap:.3} (mín 0.04)"),
        passed: accent_gap >= 0.04,
    });

    let all_states = [s.error, s.warning, s.success, s.info];
    let closest = |accents: &[Color]| {
        all_states
            .iter()
            .flat_map(|x| accents.iter().map(|y| x.delta_e(*y)))
            .fold(f64::INFINITY, f64::min)
    };
    let (primary_gap, supports_gap) = (closest(&[a.primary]), closest(&[a.secondary, a.tertiary]));
    let min_support = support_gap(p.meta.mode);
    // 8-bit rounding can land a colour just under a limit the search did meet.
    const ROUNDING: f64 = 0.002;
    out.push(Check {
        name: "Estados distintos del primario".into(),
        value: format!("ΔE {primary_gap:.3} (mín {SEMANTIC_GAP})"),
        passed: primary_gap >= SEMANTIC_GAP - ROUNDING,
    });
    out.push(Check {
        name: "Estados distintos de otros acentos".into(),
        value: format!("ΔE {supports_gap:.3} (mín {min_support})"),
        passed: supports_gap >= min_support - ROUNDING,
    });

    let states = [
        ("error", s.error),
        ("aviso", s.warning),
        ("éxito", s.success),
        ("info", s.info),
    ];
    for (ground_name, ground) in [("fondo", n.bg), ("superficie", n.surface)] {
        let (worst_name, worst) = states
            .iter()
            .map(|(name, c)| (*name, c.contrast(ground)))
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .expect("states is non-empty");
        out.push(Check {
            name: format!("Estados sobre {ground_name} (AA)"),
            value: format!("peor {worst_name} {worst:.2}:1 (mín {STATUS_CONTRAST})"),
            passed: worst >= STATUS_CONTRAST,
        });
    }

    for (name, cvd) in [
        ("Estados con protanopia", Cvd::Protan),
        ("Estados con deuteranopia", Cvd::Deutan),
    ] {
        let mut worst = ("", "", f64::INFINITY);
        for (i, (x_name, x)) in states.iter().enumerate() {
            for (y_name, y) in &states[i + 1..] {
                let gap = x.simulate(cvd).delta_e(y.simulate(cvd));
                if gap < worst.2 {
                    worst = (x_name, y_name, gap);
                }
            }
        }
        out.push(Check {
            name: name.into(),
            value: format!("peor {}/{} ΔE {:.3} (mín 0.05)", worst.0, worst.1, worst.2),
            passed: worst.2 >= 0.05,
        });
    }

    let extreme = p
        .roles()
        .iter()
        .any(|(_, c)| matches!((c.r, c.g, c.b), (0, 0, 0) | (255, 255, 255)));
    out.push(Check {
        name: "Sin negro ni blanco puros".into(),
        value: if extreme { "presente" } else { "ninguno" }.into(),
        passed: !extreme,
    });

    out
}

/// Counts hue clusters at least 20° apart among visibly chromatic colours.
fn hue_families(colors: &[Color]) -> usize {
    let mut hues: Vec<f64> = Vec::new();
    for c in colors {
        let lch = c.to_lch();
        if lch.c >= 0.03 && !hues.iter().any(|h| hue_diff(*h, lch.h).abs() < 20.0) {
            hues.push(lch.h);
        }
    }
    hues.len()
}

//! Runs the whole pipeline on deliberately difficult synthetic images and asserts
//! that every audit criterion holds across all modes, styles and harmonies.

use image::{DynamicImage, Rgba, RgbaImage};
use velure::audit;
use velure::color::{Color, hue_diff};
use velure::export::{self, Context_, Format};
use velure::extract;
use velure::harmony::{self, Scheme};
use velure::palette::{self, Mode, Options, Palette, Style};

use clap::ValueEnum;

fn stripes(colors: &[[u8; 3]]) -> DynamicImage {
    let n = colors.len() as u32;
    DynamicImage::ImageRgba8(RgbaImage::from_fn(240, 240, |x, _| {
        let [r, g, b] = colors[(x * n / 240) as usize];
        Rgba([r, g, b, 255])
    }))
}

/// Images chosen to break a naive generator.
fn hostile_images() -> Vec<(&'static str, DynamicImage)> {
    vec![
        ("gris", stripes(&[[128, 128, 128]])),
        ("negro", stripes(&[[0, 0, 0]])),
        ("blanco", stripes(&[[255, 255, 255]])),
        ("neón rosa", stripes(&[[255, 0, 200], [255, 20, 147]])),
        ("neón lima", stripes(&[[160, 255, 0], [50, 255, 20]])),
        (
            "payaso",
            stripes(&[
                [255, 0, 0],
                [255, 255, 0],
                [0, 255, 0],
                [0, 255, 255],
                [0, 0, 255],
                [255, 0, 255],
            ]),
        ),
        ("amarillo puro", stripes(&[[255, 255, 0]])),
        ("azul oscuro", stripes(&[[5, 10, 60], [10, 20, 90]])),
        (
            "sepia",
            stripes(&[[112, 66, 20], [160, 120, 80], [230, 210, 180]]),
        ),
        ("naranja y azul", stripes(&[[230, 120, 30], [30, 90, 200]])),
    ]
}

fn build(img: &DynamicImage, mode: Mode, style: Style, scheme: Option<Scheme>) -> Palette {
    let analysis = harmony::analyze(extract::clusters(img));
    palette::build(
        &analysis,
        &Options {
            source: "test".into(),
            mode: Some(mode),
            style,
            scheme,
        },
    )
}

#[test]
fn every_audit_passes_on_hostile_images() {
    let mut failures = Vec::new();
    for (name, img) in hostile_images() {
        for mode in Mode::value_variants() {
            for style in Style::value_variants() {
                let schemes = Scheme::value_variants().iter().map(|s| Some(*s));
                for scheme in std::iter::once(None).chain(schemes) {
                    let p = build(&img, *mode, *style, scheme);
                    for c in audit::run(&p).iter().filter(|c| !c.passed) {
                        failures.push(format!(
                            "{name} / {mode:?} / {style:?} / {scheme:?}: {} = {}",
                            c.name, c.value
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} fallos:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn neon_input_never_produces_neon_output() {
    for (name, img) in hostile_images() {
        let p = build(&img, Mode::Dark, Style::Balanced, None);
        for (role, c) in p.roles() {
            // Error alone may exceed the cap, to stand apart from a reddish theme.
            let limit = if role == "error" { 0.155 } else { 0.13 };
            assert!(
                c.to_lch().c <= limit,
                "{name}: {role} {} tiene croma {:.3}",
                c.hex(),
                c.to_lch().c
            );
        }
    }
}

#[test]
fn ansi_colours_keep_their_meaning() {
    let bands = [
        (1, 27.0, "rojo"),
        (2, 150.0, "verde"),
        (3, 95.0, "amarillo"),
        (4, 255.0, "azul"),
        (5, 330.0, "magenta"),
        (6, 200.0, "cian"),
    ];
    for (name, img) in hostile_images() {
        for mode in Mode::value_variants() {
            let p = build(&img, *mode, Style::Balanced, None);
            for (i, anchor, label) in bands {
                // In light themes the yellow is an ochre and the green a bluish
                // green, on purpose.
                let anchor = match (i, *mode) {
                    (2, Mode::Light) => 162.0,
                    (3, Mode::Light) => 82.0,
                    _ => anchor,
                };
                let d = hue_diff(anchor, p.ansi[i].to_lch().h).abs();
                assert!(
                    d <= 18.0,
                    "{name}/{mode:?}: ANSI {label} {} se desvía {d:.0}°",
                    p.ansi[i].hex()
                );
            }
        }
    }
}

#[test]
fn semantic_colours_keep_their_meaning() {
    for (name, img) in hostile_images() {
        for mode in Mode::value_variants() {
            let s = build(&img, *mode, Style::Balanced, None).semantic;
            for (label, c, anchor) in [
                ("error", s.error, 27.0),
                (
                    "aviso",
                    s.warning,
                    if *mode == Mode::Light { 82.0 } else { 95.0 },
                ),
                (
                    "éxito",
                    s.success,
                    if *mode == Mode::Light { 162.0 } else { 150.0 },
                ),
                ("info", s.info, 255.0),
            ] {
                let d = hue_diff(anchor, c.to_lch().h).abs();
                assert!(
                    d <= 22.0,
                    "{name}/{mode:?}: {label} {} se desvía {d:.0}°",
                    c.hex()
                );
            }
        }
    }
}

#[test]
fn error_stays_a_clear_red_when_the_primary_is_reddish() {
    let img = stripes(&[[30, 28, 30], [40, 36, 38], [205, 90, 85], [215, 95, 80]]);
    let p = build(&img, Mode::Dark, Style::Balanced, None);
    assert!(
        hue_diff(27.0, p.accents.primary.to_lch().h).abs() < 25.0,
        "la imagen debe dar un primario rojizo: {}",
        p.accents.primary.hex()
    );
    let error = p.semantic.error.to_lch();
    let label = p.semantic.error.hex();
    assert!(error.l <= 0.76, "{label} es demasiado pálido");
    assert!(error.c >= 0.09, "{label} está demasiado apagado");
    assert!(
        hue_diff(27.0, error.h).abs() <= 20.0,
        "{label} perdió el tono"
    );
    assert!(p.semantic.error.delta_e(p.accents.primary) >= 0.068);
}

#[test]
fn light_themes_are_clean_and_layered() {
    for (name, img) in hostile_images() {
        for style in Style::value_variants() {
            let p = build(&img, Mode::Light, *style, None);
            let (n, s) = (&p.neutrals, &p.semantic);
            let l = |c: Color| c.to_lch().l;

            // Raised layers are whiter than the page, and chrome sits below it.
            assert!(l(n.surface) > l(n.bg), "{name}: superficie bajo la página");
            assert!(l(n.crust) < l(n.bg), "{name}: crust sobre la página");
            // The page barely shows a tint.
            assert!(
                n.bg.to_lch().c <= 0.012,
                "{name}: fondo teñido {}",
                n.bg.hex()
            );
            // Base16 steps from the page towards the text.
            assert!(l(p.base16[1]) < l(p.base16[0]), "{name}: base01 invertido");

            for (label, c) in [
                ("error", s.error),
                ("aviso", s.warning),
                ("éxito", s.success),
                ("info", s.info),
            ] {
                // Single-hue images occupying a status hue leave little room, so this
                // guards against true black rather than asking for an ideal tone.
                assert!(
                    l(c) >= 0.30,
                    "{name}/{style:?}: {label} {} casi negro",
                    c.hex()
                );
            }
        }
    }
}

#[test]
fn accents_come_from_the_hues_the_image_really_has() {
    // A red subject on a steel-blue image: the second accent must be the blue,
    // not whatever a fixed offset from red happens to land on.
    let img = stripes(&[[40, 70, 110], [50, 85, 130], [60, 95, 145], [200, 40, 45]]);
    let p = build(&img, Mode::Dark, Style::Balanced, None);
    let hues = [p.accents.primary, p.accents.secondary].map(|c| c.to_lch().h);
    assert!(
        hue_diff(hues[0], 25.0).abs() < 20.0,
        "primario h={:.0}",
        hues[0]
    );
    assert!(
        hue_diff(hues[1], 250.0).abs() < 25.0,
        "secundario h={:.0}",
        hues[1]
    );
    assert!(p.meta.secondary_from_image);
}

#[test]
fn styles_are_ordered_by_intensity() {
    let img = stripes(&[[220, 60, 40], [40, 80, 200]]);
    let chroma = |s| build(&img, Mode::Dark, s, None).accents.primary.to_lch().c;
    assert!(chroma(Style::Sober) < chroma(Style::Balanced));
    assert!(chroma(Style::Balanced) < chroma(Style::Expressive));
}

#[test]
fn light_and_dark_invert_the_ladder() {
    let img = stripes(&[[90, 120, 160]]);
    let l = |c: Color| c.to_lch().l;
    let dark = build(&img, Mode::Dark, Style::Balanced, None).neutrals;
    let light = build(&img, Mode::Light, Style::Balanced, None).neutrals;
    assert!(l(dark.bg) < l(dark.text));
    assert!(l(light.bg) > l(light.text));
}

#[test]
fn json_carries_the_palette_and_its_audit() {
    let img = stripes(&[[90, 120, 160], [180, 120, 90]]);
    let p = build(&img, Mode::Dark, Style::Balanced, None);
    let checks = audit::run(&p);
    let parsed: serde_json::Value =
        serde_json::from_str(&export::json(&p, &checks).unwrap()).unwrap();
    assert_eq!(parsed["audit"].as_array().unwrap().len(), checks.len());
    assert_eq!(parsed["ansi"].as_array().unwrap().len(), 16);
    assert_eq!(parsed["neutrals"]["bg"], p.neutrals.bg.hex());
    let weights: f64 = parsed["meta"]["image_colors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["weight"].as_f64().unwrap())
        .sum();
    assert!((weights - 1.0).abs() < 1e-6);
}

#[test]
fn every_exporter_writes_real_colours() {
    let img = stripes(&[[90, 120, 160], [180, 120, 90]]);
    let p = build(&img, Mode::Dark, Style::Balanced, None);
    let checks = audit::run(&p);
    let ctx = Context_ {
        palette: &p,
        checks: &checks,
        thumbnail_jpeg: &[],
    };
    for f in Format::value_variants() {
        let s = export::render(*f, &ctx).unwrap();
        assert!(
            s.contains(&p.neutrals.text.bare()),
            "{f:?} no contiene el color de texto"
        );
        assert!(!s.contains("%%"), "{f:?} dejó marcadores sin reemplazar");
    }
}

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

mod desktop_tokens {
    use velure::color::{Color, hue_diff};
    use velure::harmony;
    use velure::palette::{self, Mode, Options, Palette, Style};
    use velure::tokens::{CONTRAST_RULES, Tokens, derive, guide};

    fn palette(colors: &[[u8; 3]], style: Style) -> Palette {
        let img = image::RgbImage::from_fn(colors.len() as u32 * 40, 40, |x, _| {
            image::Rgb(colors[x as usize / 40])
        });
        let clusters = velure::extract::clusters(&image::DynamicImage::ImageRgb8(img));
        palette::build(
            &harmony::analyze(clusters),
            &Options {
                source: "prueba".into(),
                mode: Some(Mode::Dark),
                style,
                scheme: None,
            },
        )
    }

    fn get(t: &Tokens, name: &str) -> Color {
        t.get(name).unwrap_or_else(|| panic!("falta {name}"))
    }

    const IMAGES: [&[[u8; 3]]; 5] = [
        &[[160, 140, 210], [230, 220, 245], [40, 35, 70]],
        &[[20, 70, 160], [220, 120, 30], [15, 20, 30]],
        &[
            [190, 40, 40],
            [40, 170, 70],
            [40, 60, 200],
            [240, 210, 60],
            [20, 20, 20],
        ],
        &[[128, 128, 128], [30, 30, 30], [230, 230, 230]],
        &[[250, 20, 20], [240, 240, 240], [5, 5, 5]],
    ];
    const STYLES: [Style; 3] = [Style::Sober, Style::Balanced, Style::Expressive];

    #[test]
    fn every_contrast_rule_holds() {
        for colors in IMAGES {
            for style in STYLES {
                let failures = derive(&palette(colors, style)).contrast_failures();
                assert!(failures.is_empty(), "{failures:?}");
            }
        }
    }

    fn light(colors: &[[u8; 3]], style: Style) -> Palette {
        let img = image::RgbImage::from_fn(colors.len() as u32 * 40, 40, |x, _| {
            image::Rgb(colors[x as usize / 40])
        });
        let clusters = velure::extract::clusters(&image::DynamicImage::ImageRgb8(img));
        palette::build(
            &harmony::analyze(clusters),
            &Options {
                source: "prueba".into(),
                mode: Some(Mode::Light),
                style,
                scheme: None,
            },
        )
    }

    #[test]
    fn light_tokens_keep_every_contrast_rule() {
        for colors in IMAGES {
            for style in STYLES {
                let t = derive(&light(colors, style));
                let failures = t.contrast_failures();
                assert!(failures.is_empty(), "{failures:?}");
                // Emphasis and containers follow the mode: dark emphasis, pale containers.
                assert!(get(&t, "emphasis").to_lch().l < 0.5);
                assert!(get(&t, "error_container").to_lch().l > get(&t, "error").to_lch().l);
            }
        }
    }

    #[test]
    fn light_code_colours_are_told_apart() {
        for colors in IMAGES {
            let t = derive(&light(colors, Style::Sober));
            let code = [
                "syntax_keyword",
                "syntax_function",
                "syntax_type",
                "syntax_string",
                "syntax_number",
                "syntax_property",
            ];
            for (i, name) in code.iter().enumerate() {
                for other in code[i + 1..].iter().chain(&["text", "subtext", "muted"]) {
                    let gap = get(&t, name).delta_e(get(&t, other));
                    assert!(gap >= 0.05, "{name} y {other} a {gap:.3}");
                }
            }
        }
    }

    #[test]
    fn contrast_rules_name_real_tokens() {
        let t = derive(&palette(IMAGES[0], Style::Balanced));
        for (fg, bg, _) in CONTRAST_RULES {
            get(&t, fg);
            get(&t, bg);
        }
    }

    #[test]
    fn each_name_appears_once() {
        let t = derive(&palette(IMAGES[0], Style::Balanced));
        let mut names: Vec<&str> = t.iter().map(|(n, _)| n).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total);
    }

    #[test]
    fn elevation_climbs_step_by_step() {
        for colors in IMAGES {
            for style in STYLES {
                let t = derive(&palette(colors, style));
                let ladder = [
                    "crust",
                    "bg",
                    "surface_low",
                    "surface",
                    "surface_high",
                    "overlay",
                    "border",
                ];
                for pair in ladder.windows(2) {
                    assert!(
                        get(&t, pair[1]).to_lch().l > get(&t, pair[0]).to_lch().l,
                        "{} no sube sobre {}",
                        pair[1],
                        pair[0]
                    );
                }
            }
        }
    }

    #[test]
    fn ansi_is_the_palette_itself() {
        let p = palette(IMAGES[1], Style::Balanced);
        let t = derive(&p);
        for (i, color) in p.ansi.iter().enumerate() {
            assert_eq!(get(&t, &format!("ansi_{i}")), *color);
        }
    }

    #[test]
    fn gradient_stays_in_one_family_on_contrasting_themes() {
        for colors in IMAGES {
            let p = palette(colors, Style::Expressive);
            let t = derive(&p);
            let h = p.accents.primary.to_lch().h;
            for i in 0..8 {
                let c = get(&t, &format!("gradient_{i}")).to_lch();
                assert!(hue_diff(h, c.h).abs() <= 40.0, "paso {i}: {} vs {h}", c.h);
            }
        }
    }

    #[test]
    fn kinds_of_code_are_told_apart() {
        let code = [
            "syntax_keyword",
            "syntax_function",
            "syntax_type",
            "syntax_string",
            "syntax_number",
            "syntax_property",
        ];
        for colors in IMAGES {
            for style in STYLES {
                let t = derive(&palette(colors, style));
                for (i, name) in code.iter().enumerate() {
                    let c = get(&t, name);
                    for other in code[i + 1..].iter().chain(&["text", "subtext", "muted"]) {
                        let gap = c.delta_e(get(&t, other));
                        assert!(gap >= 0.05, "{name} y {other} a {gap:.3}");
                    }
                }
            }
        }
    }

    #[test]
    fn code_keeps_the_accents_when_they_already_stand_out() {
        let p = palette(IMAGES[1], Style::Expressive);
        let t = derive(&p);
        assert_eq!(get(&t, "syntax_keyword"), p.accents.primary);
    }

    #[test]
    fn guide_names_the_closest_image_colour_as_origin() {
        let p = palette(IMAGES[1], Style::Balanced);
        let g = guide(&p, "/f.jpg");
        assert_eq!(g["image"], "/f.jpg");
        let base = g["origins"]["primary"]
            .as_str()
            .expect("el primario sale de la imagen");
        assert!(
            g["image_colors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c == base),
            "el origen debe aparecer entre los colores que muestra la guía"
        );
        let target = p.accents.primary.to_lch().h;
        let best = p
            .meta
            .image_colors
            .iter()
            .map(|c| c.color.to_lch())
            .filter(|c| c.c >= 0.03)
            .map(|c| hue_diff(c.h, target).abs())
            .fold(f64::MAX, f64::min);
        let shown = p
            .meta
            .image_colors
            .iter()
            .find(|c| c.color.hex() == base)
            .unwrap()
            .color;
        let got = hue_diff(shown.to_lch().h, target).abs();
        assert!(
            (got - best).abs() < 1e-9,
            "origen a {got:.0}°, el más cercano a {best:.0}°"
        );
    }
}

mod zed_theme {
    use velure::apps::{App, zed};
    use velure::color::Color;
    use velure::harmony;
    use velure::palette::{self, Mode, Options, Style};
    use velure::tokens::{Tokens, derive};

    const IMAGES: [&[[u8; 3]]; 4] = [
        &[[160, 140, 210], [230, 220, 245], [40, 35, 70]],
        &[[20, 70, 160], [220, 120, 30], [15, 20, 30]],
        &[[128, 128, 128], [30, 30, 30], [230, 230, 230]],
        &[[250, 20, 20], [240, 240, 240], [5, 5, 5]],
    ];

    fn tokens(colors: &[[u8; 3]], style: Style) -> Tokens {
        let img = image::RgbImage::from_fn(colors.len() as u32 * 40, 40, |x, _| {
            image::Rgb(colors[x as usize / 40])
        });
        let clusters = velure::extract::clusters(&image::DynamicImage::ImageRgb8(img));
        derive(&palette::build(
            &harmony::analyze(clusters),
            &Options {
                source: "prueba".into(),
                mode: Some(Mode::Dark),
                style,
                scheme: None,
            },
        ))
    }

    fn hex(s: &str) -> Color {
        let v = u32::from_str_radix(&s[1..7], 16).unwrap();
        Color {
            r: (v >> 16) as u8,
            g: (v >> 8) as u8,
            b: v as u8,
        }
    }

    /// `fg` drawn over `bg` with an `alpha` tint of `tint` in between.
    fn over(tint: Color, alpha: f64, bg: Color) -> Color {
        let mix =
            |a: u8, b: u8| (f64::from(a) * alpha + f64::from(b) * (1.0 - alpha)).round() as u8;
        Color {
            r: mix(tint.r, bg.r),
            g: mix(tint.g, bg.g),
            b: mix(tint.b, bg.b),
        }
    }

    #[test]
    fn writes_a_complete_zed_theme() {
        let theme: serde_json::Value =
            serde_json::from_str(&zed::render(&tokens(IMAGES[0], Style::Balanced))).unwrap();
        let style = &theme["themes"][0]["style"];
        assert_eq!(theme["themes"][0]["appearance"], "dark");
        assert!(style.as_object().unwrap().len() > 150);
        for (key, value) in style.as_object().unwrap() {
            if let Some(s) = value.as_str() {
                let ok = s.starts_with('#') && (s.len() == 7 || s.len() == 9);
                assert!(
                    ok && s[1..].chars().all(|c| c.is_ascii_hexdigit()),
                    "{key}: {s}"
                );
            }
        }
        assert_eq!(style["players"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn each_kind_of_code_has_its_own_token() {
        let t = tokens(IMAGES[1], Style::Balanced);
        let theme: serde_json::Value = serde_json::from_str(&zed::render(&t)).unwrap();
        let syntax = &theme["themes"][0]["style"]["syntax"];
        for (kind, token) in [
            ("keyword", "syntax_keyword"),
            ("function", "syntax_function"),
            ("type", "syntax_type"),
            ("string", "syntax_string"),
            ("number", "syntax_number"),
            ("property", "syntax_property"),
            ("comment", "muted"),
        ] {
            assert_eq!(syntax[kind]["color"], t.get(token).unwrap().hex(), "{kind}");
        }
    }

    #[test]
    fn code_stays_legible_when_selected() {
        let code = [
            "syntax_keyword",
            "syntax_function",
            "syntax_type",
            "syntax_string",
            "syntax_number",
            "syntax_property",
            "text",
        ];
        for colors in IMAGES {
            for style in [Style::Sober, Style::Balanced, Style::Expressive] {
                let t = tokens(colors, style);
                let theme: serde_json::Value = serde_json::from_str(&zed::render(&t)).unwrap();
                let selection = theme["themes"][0]["style"]["players"][0]["selection"]
                    .as_str()
                    .unwrap();
                let alpha = f64::from(u8::from_str_radix(&selection[7..9], 16).unwrap()) / 255.0;
                let ground = over(hex(selection), alpha, t.get("bg").unwrap());
                for name in code {
                    let ratio = t.get(name).unwrap().contrast(ground);
                    assert!(ratio >= 3.0, "{name} sobre la selección: {ratio:.2}");
                }
            }
        }
    }

    #[test]
    fn usage_names_real_tokens() {
        let t = tokens(IMAGES[0], Style::Balanced);
        for app in App::ALL {
            for (name, _) in app.usage() {
                assert!(t.get(name).is_some(), "{} usa {name}", app.name());
            }
        }
    }
}

mod kitty_theme {
    use velure::apps::kitty;
    use velure::harmony;
    use velure::palette::{self, Mode, Options, Style};
    use velure::tokens::{Tokens, derive};

    const IMAGES: [&[[u8; 3]]; 4] = [
        &[[160, 140, 210], [230, 220, 245], [40, 35, 70]],
        &[[20, 70, 160], [220, 120, 30], [15, 20, 30]],
        &[[128, 128, 128], [30, 30, 30], [230, 230, 230]],
        &[[250, 20, 20], [240, 240, 240], [5, 5, 5]],
    ];

    fn tokens(colors: &[[u8; 3]], style: Style) -> Tokens {
        let img = image::RgbImage::from_fn(colors.len() as u32 * 40, 40, |x, _| {
            image::Rgb(colors[x as usize / 40])
        });
        let clusters = velure::extract::clusters(&image::DynamicImage::ImageRgb8(img));
        derive(&palette::build(
            &harmony::analyze(clusters),
            &Options {
                source: "prueba".into(),
                mode: Some(Mode::Dark),
                style,
                scheme: None,
            },
        ))
    }

    #[test]
    fn every_setting_is_a_colour() {
        let conf = kitty::render(&tokens(IMAGES[0], Style::Balanced));
        let settings: Vec<&str> = conf
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        assert_eq!(settings.len(), 21 + 16);
        for line in settings {
            let value = line.split_whitespace().nth(1).unwrap();
            assert!(value.len() == 7 && value.starts_with('#'), "{line}");
        }
        for n in 0..16 {
            assert!(
                conf.lines()
                    .any(|l| l.split_whitespace().next() == Some(&format!("color{n}")))
            );
        }
    }

    #[test]
    fn what_kitty_draws_together_is_legible() {
        for colors in IMAGES {
            for style in [Style::Sober, Style::Balanced, Style::Expressive] {
                let t = tokens(colors, style);
                for (fg, bg) in kitty::READABLE_PAIRS {
                    let ratio = t.get(fg).unwrap().contrast(t.get(bg).unwrap());
                    assert!(ratio >= 4.5, "{fg} sobre {bg}: {ratio:.2}");
                }
            }
        }
    }

    #[test]
    fn terminal_colours_are_the_palette_ansi() {
        let t = tokens(IMAGES[1], Style::Balanced);
        let conf = kitty::render(&t);
        for n in 0..16 {
            let line = conf
                .lines()
                .find(|l| l.split_whitespace().next() == Some(&format!("color{n}")))
                .unwrap();
            assert!(line.ends_with(&t.get(&format!("ansi_{n}")).unwrap().hex()));
        }
    }
}

mod shell_apps {
    use velure::apps::{eza, fastfetch, fish, gtk, starship, web};
    use velure::harmony;
    use velure::palette::{self, Mode, Options, Style};
    use velure::tokens::{Tokens, derive};

    fn tokens(colors: &[[u8; 3]]) -> Tokens {
        let img = image::RgbImage::from_fn(colors.len() as u32 * 40, 40, |x, _| {
            image::Rgb(colors[x as usize / 40])
        });
        let clusters = velure::extract::clusters(&image::DynamicImage::ImageRgb8(img));
        derive(&palette::build(
            &harmony::analyze(clusters),
            &Options {
                source: "prueba".into(),
                mode: Some(Mode::Dark),
                style: Style::Sober,
                scheme: None,
            },
        ))
    }

    fn sober() -> Tokens {
        tokens(&[[128, 128, 128], [30, 30, 30], [230, 230, 230]])
    }

    #[test]
    fn fish_highlighting_uses_the_separated_code_colours() {
        let t = sober();
        let conf = fish::render(&t);
        let value = |var: &str| {
            conf.lines()
                .find(|l| l.split_whitespace().nth(2) == Some(var))
                .and_then(|l| l.split('\'').nth(1))
                .unwrap()
                .to_string()
        };
        assert_eq!(
            value("fish_color_command"),
            t.get("syntax_function").unwrap().hex()
        );
        assert_eq!(
            value("fish_color_operator"),
            t.get("syntax_type").unwrap().hex()
        );
        assert_eq!(value("fish_color_comment"), t.get("muted").unwrap().hex());
        assert!(conf.contains("set -g fish_color_valid_path --underline"));
    }

    #[test]
    fn starship_palette_is_valid_toml_with_every_name() {
        let t = sober();
        let parsed: toml::Table = toml::from_str(&starship::render(&t)).unwrap();
        let palette = parsed["palettes"]["velure"].as_table().unwrap();
        for name in [
            "primary",
            "accent",
            "accent_alt",
            "ok",
            "warn",
            "err",
            "muted",
        ] {
            assert!(palette[name].as_str().unwrap().starts_with('#'), "{name}");
        }
        assert_ne!(palette["accent"].as_str(), palette["muted"].as_str());
    }

    #[test]
    fn fastfetch_keeps_the_modules_and_sets_only_colours() {
        let t = sober();
        let base = r#"{
            // la base del usuario
            "logo": { "source": "arch.txt", "padding": { "right": 6 } },
            "modules": ["title", { "type": "os", "key": " ", "keyColor": "red", },],
        }"#;
        let out = fastfetch::render(&t, base).unwrap();
        let json: serde_json::Value = serde_json::from_str(&fastfetch::strip_jsonc(&out)).unwrap();
        let primary = t.get("primary").unwrap().hex();
        assert_eq!(json["logo"]["source"], "arch.txt");
        assert_eq!(json["logo"]["padding"]["right"], 6);
        assert_eq!(json["logo"]["color"]["1"], primary);
        assert_eq!(json["display"]["color"]["keys"], primary);
        assert_eq!(json["display"]["color"]["title"], primary);
        assert_eq!(json["modules"][1]["key"], " ");
        assert!(json["modules"][1].get("keyColor").is_none());
    }

    #[test]
    fn eza_keeps_long_listings_calm() {
        let t = sober();
        let theme = eza::render(&t);
        let perms = theme
            .split("perms:")
            .nth(1)
            .unwrap()
            .split("\n\n")
            .next()
            .unwrap();
        let neutral = [
            t.get("subtext").unwrap().hex(),
            t.get("muted").unwrap().hex(),
        ];
        let coloured: Vec<&str> = perms
            .lines()
            .filter(|l| l.contains("foreground"))
            .filter(|l| !neutral.iter().any(|n| l.contains(n.as_str())))
            .collect();
        assert_eq!(
            coloured.len(),
            3,
            "solo escritura para todos y setuid: {coloured:?}"
        );
        for line in theme.lines().filter(|l| l.contains("foreground")) {
            let value = line.split('"').nth(1).unwrap();
            assert!(value.len() == 7 && value.starts_with('#'), "{line}");
        }
    }

    #[test]
    fn gtk_fills_carry_their_own_text() {
        for colors in [
            &[[160, 140, 210], [230, 220, 245], [40, 35, 70]][..],
            &[[20, 70, 160], [220, 120, 30], [15, 20, 30]][..],
            &[[128, 128, 128], [30, 30, 30], [230, 230, 230]][..],
        ] {
            let t = tokens(colors);
            for (fg, bg) in gtk::READABLE_PAIRS {
                let ratio = t.get(fg).unwrap().contrast(t.get(bg).unwrap());
                assert!(ratio >= 4.5, "{fg} sobre {bg}: {ratio:.2}");
            }
        }
        let t = sober();
        let gtk3 = gtk::render(&t, false);
        let gtk4 = gtk::render(&t, true);
        assert!(gtk3.contains(&format!(
            "@define-color success_fg_color {};",
            t.get("on_success").unwrap().hex()
        )));
        assert!(!gtk3.contains(":root"));
        assert!(gtk4.contains(&format!(
            "--accent-bg-color: {};",
            t.get("primary").unwrap().hex()
        )));
        assert!(gtk4.contains("--border-color:"));
        assert_eq!(
            gtk4.matches("@define-color").count(),
            gtk3.matches("@define-color").count()
        );
    }

    #[test]
    fn firefox_and_tidal_name_only_real_tokens_and_read_well() {
        for colors in [
            &[[160, 140, 210], [230, 220, 245], [40, 35, 70]][..],
            &[[20, 70, 160], [220, 120, 30], [15, 20, 30]][..],
            &[[128, 128, 128], [30, 30, 30], [230, 230, 230]][..],
        ] {
            let t = tokens(colors);
            for (name, css) in web::FIREFOX {
                let out = web::render(css, &t).unwrap_or_else(|e| panic!("{name}: {e}"));
                assert!(!out.contains("{{"), "{name}");
            }
            let tidal = web::render(web::TIDAL, &t).unwrap();
            assert!(!tidal.contains("{{"));
            for (fg, bg) in web::TIDAL_READABLE.iter().chain(web::FIREFOX_READABLE) {
                let ratio = t.get(fg).unwrap().contrast(t.get(bg).unwrap());
                assert!(ratio >= 4.5, "{fg} sobre {bg}: {ratio:.2}");
            }
        }
    }

    #[test]
    fn firefox_keeps_the_alpha_its_translucency_script_rewrites() {
        // translucent.sh replaces "e0 !important" with the opacity chosen in the shell.
        let t = sober();
        let (_, css) = web::FIREFOX
            .iter()
            .find(|(n, _)| *n == "firefox-translucent.css")
            .unwrap();
        let out = web::render(css, &t).unwrap();
        assert!(out.contains(&format!("{}e0 !important", t.get("bg").unwrap().hex())));
    }

    #[test]
    fn tidal_marks_what_plays_on_the_selection_colour() {
        let t = sober();
        let tidal = web::render(web::TIDAL, &t).unwrap();
        let container = t.get("primary_container").unwrap().hex();
        assert!(tidal.contains(&format!("background-color: {container};")));
        assert!(!tidal.contains(&format!(
            "background-color: {};",
            t.get("outline").unwrap().hex()
        )));
    }
}

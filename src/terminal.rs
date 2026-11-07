//! Truecolor report for the terminal.

use crate::audit::Check;
use crate::color::Color;
use crate::harmony::Analysis;
use crate::palette::Palette;

/// Whether to emit ANSI colour. Honours the `NO_COLOR` convention.
pub struct Painter {
    pub color: bool,
}

impl Painter {
    fn swatch(&self, c: Color, width: usize) -> String {
        if self.color {
            format!(
                "\x1b[48;2;{};{};{}m{}\x1b[0m",
                c.r,
                c.g,
                c.b,
                " ".repeat(width)
            )
        } else {
            String::new()
        }
    }

    fn fg(&self, c: Color, s: &str) -> String {
        if self.color {
            format!("\x1b[38;2;{};{};{}m{s}\x1b[0m", c.r, c.g, c.b)
        } else {
            s.to_string()
        }
    }

    fn dim(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[2m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn bold(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[1m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
}

pub fn report(p: &Palette, a: &Analysis, checks: &[Check], paint: &Painter) -> String {
    let mut out = String::new();
    let mode = match p.meta.mode {
        crate::palette::Mode::Dark => "oscuro",
        crate::palette::Mode::Light => "claro",
    };
    out += &format!(
        "\n{}  {}\n",
        paint.bold("velure"),
        paint.dim(&format!(
            "{} · {mode} · {}",
            p.meta.source,
            p.meta.style.label()
        ))
    );

    let source = match (&p.meta.template, p.meta.achromatic_source) {
        (_, true) => "imagen sin color: se usa un tinte neutro".to_string(),
        (Some(f), _) => format!(
            "plantilla {} de Matsuda, desvío {:.1}°, rotación {:.0}°",
            f.template, f.cost, f.rotation
        ),
        (None, _) => "elegida a mano".to_string(),
    };
    out += &format!("\n  Armonía  {} ({source})\n", p.meta.scheme.label());
    out += &format!(
        "  Tono     clave {:.0}°, atmósfera {:.0}°, {} colores en la imagen\n",
        a.key_hue,
        a.atmosphere_hue,
        a.clusters.len()
    );
    let origin = |from_image: bool| if from_image { "imagen" } else { "armonía" };
    out += &format!(
        "  Acentos  primario imagen, secundario {}, terciario {}\n\n",
        origin(p.meta.secondary_from_image),
        origin(p.meta.tertiary_from_image)
    );

    for (name, c) in p.roles() {
        let lch = c.to_lch();
        out += &format!(
            "  {} {:<18} {}  {}\n",
            paint.swatch(c, 6),
            name,
            c.hex(),
            paint.dim(&format!(
                "L {:.2}  C {:.3}  h {:>3.0}°",
                lch.l, lch.c, lch.h
            ))
        );
    }

    if paint.color {
        out += "\n  ";
        for c in &p.ansi[..8] {
            out += &paint.swatch(*c, 5);
        }
        out += "  ansi\n  ";
        for c in &p.ansi[8..] {
            out += &paint.swatch(*c, 5);
        }
        out += "\n";
    }

    let passed = checks.iter().filter(|c| c.passed).count();
    out += &format!(
        "\n  {} {passed}/{} criterios\n",
        paint.bold("Auditoría"),
        checks.len()
    );
    for c in checks {
        let mark = if c.passed {
            paint.fg(p.semantic.success, "✓")
        } else {
            paint.fg(p.semantic.error, "✗")
        };
        out += &format!("  {mark} {:<38} {}\n", c.name, paint.dim(&c.value));
    }
    out
}

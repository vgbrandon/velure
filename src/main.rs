use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};

use velure::export::{self, Context_, Format};
use velure::harmony::{self, Scheme};
use velure::palette::{self, Mode, Options, Style};
use velure::terminal::{self, Painter};
use velure::{audit, extract};

/// Crea temas de ricing elegantes a partir de una imagen.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Imagen de origen (jpg, png, webp, gif, bmp o tiff)
    imagen: PathBuf,

    /// Modo del tema; por defecto se elige según la luminosidad de la imagen
    #[arg(short, long, value_enum)]
    modo: Option<Mode>,

    /// Intensidad del color; determina el tope de croma de los acentos
    #[arg(short, long, value_enum, default_value = "equilibrado")]
    estilo: Style,

    /// Fuerza una armonía; por defecto se detecta en la imagen
    #[arg(short, long, value_enum)]
    armonia: Option<Scheme>,

    /// Formatos a exportar, separados por comas, o "todo"
    #[arg(short = 'x', long, value_delimiter = ',', value_name = "FORMATOS")]
    exportar: Vec<String>,

    /// Carpeta de salida [por defecto: ./velure-<nombre de la imagen>]
    #[arg(short = 'o', long, value_name = "DIR")]
    salida: Option<PathBuf>,

    /// Imprime la paleta como JSON en stdout en lugar del informe
    #[arg(long)]
    json: bool,
}

fn parse_formats(raw: &[String]) -> Result<Vec<Format>> {
    let mut out = Vec::new();
    for name in raw {
        if name == "todo" {
            return Ok(Format::value_variants().to_vec());
        }
        match Format::from_str(name, true) {
            Ok(f) => out.push(f),
            Err(_) => {
                let valid: Vec<String> = Format::value_variants()
                    .iter()
                    .filter_map(|f| f.to_possible_value())
                    .map(|v| v.get_name().to_string())
                    .collect();
                bail!(
                    "formato desconocido «{name}». Disponibles: {}, todo",
                    valid.join(", ")
                );
            }
        }
    }
    Ok(out)
}

fn run(cli: Cli) -> Result<()> {
    let imagen = cli.imagen;
    let formats = parse_formats(&cli.exportar)?;
    let img = extract::load(&imagen)?;
    let clusters = extract::clusters(&img);
    if clusters.is_empty() {
        bail!("la imagen no tiene píxeles visibles");
    }

    let analysis = harmony::analyze(clusters);
    let source = imagen
        .file_name()
        .map_or_else(|| "imagen".into(), |n| n.to_string_lossy().into_owned());
    let palette = palette::build(
        &analysis,
        &Options {
            source,
            mode: cli.modo,
            style: cli.estilo,
            scheme: cli.armonia,
        },
    );
    let checks = audit::run(&palette);

    if cli.json {
        print!("{}", export::json(&palette, &checks)?);
    } else {
        let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
        print!(
            "{}",
            terminal::report(&palette, &analysis, &checks, &Painter { color })
        );
    }

    if !formats.is_empty() {
        let dir = cli.salida.unwrap_or_else(|| {
            let stem = imagen
                .file_stem()
                .map_or_else(|| "imagen".into(), |s| s.to_string_lossy().into_owned());
            PathBuf::from(format!("velure-{stem}"))
        });
        let thumbnail_jpeg = if formats.contains(&Format::Preview) {
            extract::thumbnail_jpeg(&img)?
        } else {
            Vec::new()
        };
        let ctx = Context_ {
            palette: &palette,
            checks: &checks,
            thumbnail_jpeg: &thumbnail_jpeg,
        };
        export::write_all(&dir, &formats, &ctx)?;
        eprintln!("\n  {} archivos en {}", formats.len(), dir.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

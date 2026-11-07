use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

use velure::apps::App;
use velure::export::{self, Context_, Format};
use velure::harmony::{self, Scheme};
use velure::palette::{self, Mode, Options, Style};
use velure::terminal::{self, Painter};
use velure::{audit, extract, templates, tokens};

/// Crea temas de ricing elegantes a partir de una imagen.
#[derive(Parser)]
#[command(
    version,
    about,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Cli {
    #[command(subcommand)]
    comando: Option<Comando>,

    /// Imagen de origen (jpg, png, webp, gif, bmp o tiff)
    #[arg(required = true)]
    imagen: Option<PathBuf>,

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

#[derive(Subcommand)]
enum Comando {
    /// Aplica la imagen como tema del escritorio: deriva los tokens, escribe los temas de
    /// las apps y las plantillas y ejecuta sus ganchos. Imprime los tokens como JSON
    /// (siempre en modo oscuro)
    Aplicar {
        /// Imagen de origen
        imagen: PathBuf,

        /// Intensidad del color
        #[arg(short, long, value_enum, default_value = "equilibrado")]
        estilo: Style,

        /// Fuerza una armonía; por defecto se detecta en la imagen
        #[arg(short, long, value_enum)]
        armonia: Option<Scheme>,

        /// Configuración de las plantillas
        #[arg(
            long,
            value_name = "ARCHIVO",
            default_value = "~/.config/velure/config.toml"
        )]
        config: String,

        /// Escribe los temas sin avisar a las apps
        #[arg(long)]
        sin_ganchos: bool,

        /// Solo imprime los tokens, sin escribir temas
        #[arg(long)]
        sin_plantillas: bool,
    },
    /// Qué tokens usa cada app incorporada y dónde escribe su tema
    Uso,
    /// Escribe los tokens y su origen, los datos que muestran las guías de color
    Tokens {
        /// Imagen de origen
        imagen: PathBuf,

        /// Intensidad del color
        #[arg(short, long, value_enum, default_value = "equilibrado")]
        estilo: Style,

        /// Fuerza una armonía; por defecto se detecta en la imagen
        #[arg(short, long, value_enum)]
        armonia: Option<Scheme>,

        /// Archivo de salida; por defecto, stdout
        #[arg(short = 'o', long, value_name = "ARCHIVO")]
        salida: Option<PathBuf>,
    },
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

/// The dark palette behind the desktop theme, and the image's absolute path.
fn desktop_palette(
    imagen: &Path,
    style: Style,
    scheme: Option<Scheme>,
) -> Result<(palette::Palette, String)> {
    let absolute = std::fs::canonicalize(imagen)
        .with_context(|| format!("no se pudo abrir {}", imagen.display()))?;
    let clusters = extract::clusters(&extract::load(&absolute)?);
    if clusters.is_empty() {
        bail!("la imagen no tiene píxeles visibles");
    }
    let source = absolute.to_string_lossy().into_owned();
    let palette = palette::build(
        &harmony::analyze(clusters),
        &Options {
            source: source.clone(),
            mode: Some(Mode::Dark),
            style,
            scheme,
        },
    );
    Ok((palette, source))
}

fn run(cli: Cli) -> Result<()> {
    match cli.comando {
        Some(Comando::Aplicar {
            imagen,
            estilo,
            armonia,
            config,
            sin_ganchos,
            sin_plantillas,
        }) => {
            let (palette, source) = desktop_palette(&imagen, estilo, armonia)?;
            let derived = tokens::derive(&palette);
            let failures = derived.contrast_failures();
            if !failures.is_empty() {
                eprintln!("velure: contraste insuficiente: {}", failures.join("; "));
            }
            if !sin_plantillas {
                let config = templates::expand_home(&config);
                if let Err(e) = templates::apply(&config, &derived, &source, !sin_ganchos) {
                    eprintln!("velure: {e:#}");
                }
            }
            // The colours are valid even if an app failed, so the shell must still get them.
            println!(
                "{}",
                serde_json::json!({
                    "image": source,
                    "achromatic": palette.meta.achromatic_source,
                    "tokens": tokens::to_json(&derived),
                })
            );
            return Ok(());
        }
        Some(Comando::Uso) => {
            let apps: Vec<serde_json::Value> = App::ALL
                .iter()
                .map(|app| {
                    let roles: serde_json::Map<String, serde_json::Value> = app
                        .usage()
                        .into_iter()
                        .map(|(token, count)| (token.to_string(), count.into()))
                        .collect();
                    serde_json::json!({
                        "app": app.name(),
                        "salida": app.default_output(),
                        "roles": roles,
                    })
                })
                .collect();
            println!("{}", serde_json::json!({ "apps": apps }));
            return Ok(());
        }
        Some(Comando::Tokens {
            imagen,
            estilo,
            armonia,
            salida,
        }) => {
            let (palette, source) = desktop_palette(&imagen, estilo, armonia)?;
            let text = serde_json::to_string_pretty(&tokens::guide(&palette, &source))?;
            match salida {
                Some(path) => {
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir)?;
                    }
                    std::fs::write(&path, text)
                        .with_context(|| format!("no se pudo escribir {}", path.display()))?;
                }
                None => println!("{text}"),
            }
            return Ok(());
        }
        None => {}
    }
    let imagen = cli.imagen.expect("clap exige la imagen sin subcomando");
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

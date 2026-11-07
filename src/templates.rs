//! Writes the desktop theme: velure's own app themes (`[apps.NAME]`) and user templates
//! (`[plantillas.NAME]`), as listed in a config file:
//!
//! ```toml
//! [apps.zed]                       # built in; `salida`, `gancho` and `base` are optional
//! gancho = 'sh ~/script.sh'
//!
//! [plantillas.firefox]             # a file with `{{primary}}` markers
//! entrada = '~/.config/velure/templates/firefox.css'
//! salida = '~/.cache/velure/firefox.css'
//! ```
//!
//! Apps go first, then templates, each group in file order: some hooks rely on the output
//! of an earlier table. Without a config file every built-in app is written to its
//! default place.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::apps::{self, App};
use crate::tokens::Tokens;

#[derive(Default, Deserialize)]
struct Config {
    #[serde(default)]
    apps: toml::map::Map<String, toml::Value>,
    #[serde(default)]
    plantillas: toml::map::Map<String, toml::Value>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AppEntry {
    #[serde(skip)]
    pub name: String,
    pub salida: Option<String>,
    pub gancho: Option<String>,
    /// The user's file the theme builds on, for apps that need one.
    pub base: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Template {
    #[serde(skip)]
    pub name: String,
    pub entrada: String,
    pub salida: String,
    pub gancho: Option<String>,
}

fn entries<T: for<'de> Deserialize<'de>>(
    map: toml::map::Map<String, toml::Value>,
    kind: &str,
    name_of: impl Fn(&mut T, String),
) -> Result<Vec<T>> {
    map.into_iter()
        .map(|(name, value)| {
            let mut entry: T = value
                .try_into()
                .with_context(|| format!("{kind} «{name}»"))?;
            name_of(&mut entry, name);
            Ok(entry)
        })
        .collect()
}

pub fn parse_config(text: &str) -> Result<(Vec<AppEntry>, Vec<Template>)> {
    let config: Config = toml::from_str(text)?;
    let apps = entries(config.apps, "app", |e: &mut AppEntry, n| e.name = n)?;
    let templates = entries(config.plantillas, "plantilla", |t: &mut Template, n| {
        t.name = n
    })?;
    Ok((apps, templates))
}

pub fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => Path::new(&home).join(rest),
        _ => PathBuf::from(path),
    }
}

/// Replaces `{{token}}` and `{{imagen}}`. Unknown markers stay in the text and are
/// returned, so a typo can't silently produce an empty colour.
pub fn fill(text: &str, colors: &HashMap<String, String>, image: &str) -> (String, Vec<String>) {
    let mut out = String::with_capacity(text.len());
    let mut unknown = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return (out, unknown);
        };
        let marker = after[..end].trim();
        let value = if marker == "imagen" {
            Some(image)
        } else {
            colors.get(marker).map(String::as_str)
        };
        match value {
            Some(value) => out.push_str(value),
            None => {
                unknown.push(marker.to_string());
                out.push_str(&rest[start..start + end + 4]);
            }
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    (out, unknown)
}

pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Apps that watch the file must never read a half-written one.
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".velure-tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn run_hook(hook: &str) -> Result<()> {
    // Quickshell parses this program's stdout as JSON: a chatty hook must not reach it.
    let status = Command::new("sh")
        .args(["-c", hook])
        .stdout(Stdio::from(std::io::stderr()))
        .status()?;
    anyhow::ensure!(status.success(), "terminó con {status}");
    Ok(())
}

fn run_hook_if(hook: Option<&str>, hooks: bool) -> Result<()> {
    if hooks && let Some(hook) = hook {
        run_hook(hook).context("gancho")?;
    }
    Ok(())
}

fn write_and_hook(output: &Path, content: &str, hook: Option<&str>, hooks: bool) -> Result<()> {
    write_atomic(output, content)
        .with_context(|| format!("no se pudo escribir {}", output.display()))?;
    run_hook_if(hook, hooks)
}

/// Writes the apps and templates of `config` and runs their hooks. One broken app must
/// not leave the others without a theme, so failures are reported and the rest continue.
/// Returns how many failed.
pub fn apply(config: &Path, tokens: &Tokens, image: &str, hooks: bool) -> Result<usize> {
    let (apps, templates) = match std::fs::read_to_string(config) {
        Ok(text) => parse_config(&text).with_context(|| config.display().to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
            App::ALL
                .iter()
                .map(|a| AppEntry {
                    name: a.name().to_string(),
                    ..AppEntry::default()
                })
                .collect(),
            Vec::new(),
        ),
        Err(e) => {
            return Err(anyhow!(e))
                .with_context(|| format!("no se pudo leer {}", config.display()));
        }
    };
    let colors: HashMap<String, String> = tokens
        .iter()
        .map(|(k, v)| (k.to_string(), v.hex()))
        .collect();
    let mut failed = 0;
    for entry in &apps {
        if let Err(e) = apply_app(entry, tokens, hooks) {
            eprintln!("velure: app {}: {e:#}", entry.name);
            failed += 1;
        }
    }
    for template in &templates {
        if let Err(e) = apply_template(template, &colors, image, hooks) {
            eprintln!("velure: plantilla {}: {e:#}", template.name);
            failed += 1;
        }
    }
    Ok(failed)
}

fn apply_app(entry: &AppEntry, tokens: &Tokens, hooks: bool) -> Result<()> {
    let app = App::from_name(&entry.name).ok_or_else(|| {
        let known: Vec<&str> = App::ALL.iter().map(|a| a.name()).collect();
        anyhow!("no es una app de velure (hay: {})", known.join(", "))
    })?;
    let output = apps::output_path(app, entry.salida.as_deref());
    app.write(tokens, entry.base.as_deref(), &output)?;
    run_hook_if(entry.gancho.as_deref(), hooks)
}

fn apply_template(
    template: &Template,
    colors: &HashMap<String, String>,
    image: &str,
    hooks: bool,
) -> Result<()> {
    let input = expand_home(&template.entrada);
    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("no se pudo leer {}", input.display()))?;
    let (filled, unknown) = fill(&source, colors, image);
    if !unknown.is_empty() {
        eprintln!(
            "velure: plantilla {}: sin valor para {}",
            template.name,
            unknown.join(", ")
        );
    }
    let output = expand_home(&template.salida);
    write_and_hook(&output, &filled, template.gancho.as_deref(), hooks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colors() -> HashMap<String, String> {
        HashMap::from([("text".to_string(), "#112233".to_string())])
    }

    #[test]
    fn fills_colors_and_image() {
        let (out, unknown) = fill("a {{text}}80 {{ imagen }} {{text}}", &colors(), "/f.jpg");
        assert_eq!(out, "a #11223380 /f.jpg #112233");
        assert!(unknown.is_empty());
    }

    #[test]
    fn keeps_and_reports_unknown_markers() {
        let (out, unknown) = fill("{{nope}} {{raro}} {{sin cerrar", &colors(), "");
        assert_eq!(out, "{{nope}} {{raro}} {{sin cerrar");
        assert_eq!(unknown, ["nope", "raro"]);
    }

    #[test]
    fn keeps_the_order_of_the_file() {
        let (apps, list) = parse_config(
            "[apps.zed]\ngancho = 'x'\n[plantillas.zeta]\nentrada = 'a' # nota\nsalida = \"b\"\n\
             [plantillas.alfa]\nentrada = 'c'\nsalida = 'd'\ngancho = 'e'\n",
        )
        .unwrap();
        assert_eq!(apps[0].name, "zed");
        assert_eq!(apps[0].gancho.as_deref(), Some("x"));
        assert_eq!(apps[0].salida, None);
        let names: Vec<&str> = list.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["zeta", "alfa"]);
        assert_eq!(list[0].entrada, "a");
        assert_eq!(list[1].gancho.as_deref(), Some("e"));
    }

    fn tokens() -> Tokens {
        use crate::palette::{Mode, Options, Style};
        let img = image::RgbImage::from_fn(80, 40, |x, _| {
            image::Rgb(if x < 40 {
                [160, 140, 210]
            } else {
                [30, 25, 50]
            })
        });
        let clusters = crate::extract::clusters(&image::DynamicImage::ImageRgb8(img));
        let palette = crate::palette::build(
            &crate::harmony::analyze(clusters),
            &Options {
                source: "prueba".into(),
                mode: Some(Mode::Dark),
                style: Style::Balanced,
                scheme: None,
            },
        );
        crate::tokens::derive(&palette)
    }

    #[test]
    fn a_broken_entry_does_not_stop_the_others() {
        let dir = std::env::temp_dir().join(format!("velure-plantillas-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bien.txt"), "{{text}}").unwrap();
        let config = dir.join("config.toml");
        let d = dir.display();
        std::fs::write(
            &config,
            format!(
                "[apps.zed]\nsalida = '{d}/zed.json'\n[apps.nada]\n\
                 [plantillas.mala]\nentrada = '{d}/no-existe'\nsalida = '{d}/mala'\n\
                 [plantillas.gancho]\nentrada = '{d}/bien.txt'\nsalida = '{d}/g'\ngancho = 'exit 3'\n\
                 [plantillas.bien]\nentrada = '{d}/bien.txt'\nsalida = '{d}/sub/bien'\n"
            ),
        )
        .unwrap();
        let t = tokens();
        assert_eq!(apply(&config, &t, "", true).unwrap(), 3);
        let text = t.get("text").unwrap().hex();
        assert_eq!(std::fs::read_to_string(dir.join("sub/bien")).unwrap(), text);
        assert_eq!(std::fs::read_to_string(dir.join("g")).unwrap(), text);
        let zed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("zed.json")).unwrap()).unwrap();
        assert_eq!(zed["themes"][0]["name"], "Velure");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

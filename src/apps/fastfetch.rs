//! fastfetch: `~/.config/fastfetch/config.jsonc`. Which modules to show is the user's
//! choice, not a colour decision, so it comes from a base file (`base`, by default
//! `~/.config/fastfetch/base.jsonc`) and velure only sets its colours: the logo and the
//! keys and the title in the primary accent.

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};

use crate::tokens::Tokens;

pub const DEFAULT_BASE: &str = "~/.config/fastfetch/base.jsonc";

/// JSON with comments and trailing commas, as fastfetch accepts, turned into plain JSON.
pub fn strip_jsonc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                '\\' => out.extend(chars.next()),
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for skipped in chars.by_ref() {
                    if last == '*' && skipped == '/' {
                        break;
                    }
                    last = skipped;
                }
            }
            (',', _) => {
                let rest: String = chars.clone().collect();
                let next = rest.trim_start().chars().next();
                if !matches!(next, Some('}' | ']')) {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn object<'a>(parent: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    let entry = parent.entry(key).or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    entry.as_object_mut().expect("objeto")
}

pub fn render(tokens: &Tokens, base: &str) -> Result<String> {
    let mut config: Value = serde_json::from_str(&strip_jsonc(base))
        .context("la base de fastfetch no es JSON válido")?;
    let root = config
        .as_object_mut()
        .context("la base de fastfetch debe ser un objeto")?;
    let primary = tokens.get("primary").expect("token primary").hex();
    object(object(root, "logo"), "color").insert("1".into(), json!(primary));
    let display = object(object(root, "display"), "color");
    display.insert("keys".into(), json!(primary));
    display.insert("title".into(), json!(primary));
    // A colour set per module would override the theme.
    if let Some(Value::Array(modules)) = root.get_mut("modules") {
        for module in modules.iter_mut().filter_map(Value::as_object_mut) {
            module.remove("keyColor");
        }
    }
    let mut out = String::from(
        "// Generado por velure desde el fondo de pantalla. No editar a mano: los módulos se\n\
         // cambian en la base (~/.config/fastfetch/base.jsonc).\n",
    );
    out.push_str(&serde_json::to_string_pretty(&config)?);
    out.push('\n');
    Ok(out)
}

pub fn usage() -> Vec<&'static str> {
    vec!["primary", "primary", "primary"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_comments_and_trailing_commas_but_not_strings() {
        let text = "{\n  // nota\n  \"a\": \"http://x\", /* b */ \"c\": [1, 2,],\n  \"d\": \"\\\"//\\\"\",\n}";
        let value: Value = serde_json::from_str(&strip_jsonc(text)).unwrap();
        assert_eq!(value["a"], "http://x");
        assert_eq!(value["c"], json!([1, 2]));
        assert_eq!(value["d"], "\"//\"");
    }
}

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use ratatui::style::Color;
use serde::Deserialize;

use crate::config::{Theme, fallback_theme};

const USER_THEME_APP_DIR: &str = "terminal-snake";

include!(concat!(env!("OUT_DIR"), "/builtin_themes.rs"));

#[derive(Debug, Clone)]
pub struct ThemeItem {
    pub id: String,
    pub theme: Theme,
}

#[derive(Debug, Clone)]
pub struct ThemeCatalog {
    themes: Vec<ThemeItem>,
    selected_idx: usize,
}

impl ThemeCatalog {
    /// Loads embedded bundled themes, then overlays user-provided themes.
    #[must_use]
    pub fn load() -> Self {
        let mut order = Vec::<String>::new();
        let mut by_id = HashMap::<String, Theme>::new();

        merge_embedded_themes(&mut order, &mut by_id);

        if let Some(path) = user_theme_dir() {
            merge_theme_dir(&path, &mut order, &mut by_id);
        }

        if by_id.is_empty() {
            insert_theme(
                &mut order,
                &mut by_id,
                "fallback".to_owned(),
                fallback_theme(),
            );
        }

        let mut themes = Vec::with_capacity(order.len());
        for id in order {
            if let Some(theme) = by_id.remove(&id) {
                themes.push(ThemeItem { id, theme });
            }
        }

        let selected_idx = themes
            .iter()
            .position(|theme| theme.id == "ember")
            .unwrap_or(0);

        Self {
            themes,
            selected_idx,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.themes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.themes.is_empty()
    }

    #[must_use]
    pub fn current_theme(&self) -> &Theme {
        &self.themes[self.selected_idx].theme
    }

    #[must_use]
    pub fn current_index(&self) -> usize {
        self.selected_idx
    }

    #[must_use]
    pub fn current_id(&self) -> &str {
        &self.themes[self.selected_idx].id
    }

    #[must_use]
    pub fn items(&self) -> &[ThemeItem] {
        &self.themes
    }

    #[must_use]
    pub fn theme_at(&self, idx: usize) -> Option<&Theme> {
        self.themes.get(idx).map(|item| &item.theme)
    }

    #[must_use]
    pub fn id_at(&self, idx: usize) -> Option<&str> {
        self.themes.get(idx).map(|item| item.id.as_str())
    }

    pub fn select_index(&mut self, idx: usize) -> bool {
        if idx < self.themes.len() {
            self.selected_idx = idx;
            return true;
        }

        false
    }

    pub fn select_next(&mut self) {
        self.selected_idx = (self.selected_idx + 1) % self.themes.len();
    }

    pub fn select_previous(&mut self) {
        self.selected_idx = if self.selected_idx == 0 {
            self.themes.len() - 1
        } else {
            self.selected_idx - 1
        };
    }

    #[must_use]
    pub fn select_by_id(&mut self, id: &str) -> bool {
        if let Some(idx) = self.themes.iter().position(|theme| theme.id == id) {
            self.selected_idx = idx;
            return true;
        }

        false
    }
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    defs: HashMap<String, ColorValue>,
    theme: HashMap<String, ColorValue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ColorValue {
    String(String),
    Ansi(u8),
    Variant {
        #[serde(default)]
        dark: Option<Box<ColorValue>>,
        #[serde(default)]
        light: Option<Box<ColorValue>>,
    },
}

fn insert_theme(
    order: &mut Vec<String>,
    by_id: &mut HashMap<String, Theme>,
    id: String,
    theme: Theme,
) {
    if !by_id.contains_key(&id) {
        order.push(id.clone());
    }
    by_id.insert(id, theme);
}

fn merge_theme_dir(path: &Path, order: &mut Vec<String>, by_id: &mut HashMap<String, Theme>) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut theme_paths: Vec<PathBuf> = Vec::new();
    for entry_result in entries {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let file_path = entry.path();
        if is_json_file(&file_path) {
            theme_paths.push(file_path);
        }
    }

    theme_paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for file_path in theme_paths {
        let Some(id) = file_path
            .file_stem()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
        else {
            continue;
        };

        let content = match fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(error) => {
                eprintln!(
                    "Warning: failed to read theme file {}: {error}",
                    file_path.display()
                );
                continue;
            }
        };

        match parse_theme_from_str_result(&id, &content) {
            Ok(theme) => insert_theme(order, by_id, id, theme),
            Err(error) => {
                eprintln!(
                    "Warning: invalid theme file {}; skipping: {error}",
                    file_path.display()
                );
            }
        }
    }
}

fn merge_embedded_themes(order: &mut Vec<String>, by_id: &mut HashMap<String, Theme>) {
    for &(id, content) in BUILTIN_THEMES {
        match parse_theme_from_str_result(id, content) {
            Ok(theme) => insert_theme(order, by_id, id.to_owned(), theme),
            Err(error) => {
                eprintln!("Warning: invalid built-in theme '{id}'; skipping: {error}");
            }
        }
    }
}

#[derive(Debug)]
enum ThemeParseError {
    Json(serde_json::Error),
}

impl std::fmt::Display for ThemeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(f, "json parse error: {error}"),
        }
    }
}

impl From<serde_json::Error> for ThemeParseError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

fn parse_theme_from_str_result(id: &str, raw: &str) -> Result<Theme, ThemeParseError> {
    let parsed = serde_json::from_str::<ThemeFile>(raw)?;
    let fallback = fallback_theme();
    let mut stack = Vec::new();
    let ui_muted =
        resolve_token(&parsed, "ui_muted", true, &mut stack).unwrap_or(fallback.ui_muted);
    let ui_bright_default = brighten_30_percent(ui_muted);

    Ok(Theme {
        name: parsed.name.clone().unwrap_or_else(|| display_name(id)),
        snake_head: resolve_token(&parsed, "snake_head", true, &mut stack)
            .unwrap_or(fallback.snake_head),
        snake_body: resolve_token(&parsed, "snake_body", true, &mut stack)
            .unwrap_or(fallback.snake_body),
        snake_tail: resolve_token(&parsed, "snake_tail", true, &mut stack)
            .unwrap_or(fallback.snake_tail),
        food: resolve_token(&parsed, "food", true, &mut stack).unwrap_or(fallback.food),
        super_food: resolve_token(&parsed, "super_food", true, &mut stack)
            .unwrap_or(fallback.super_food),
        terminal_bg: resolve_token(&parsed, "terminal_bg", true, &mut stack)
            .unwrap_or(fallback.terminal_bg),
        field_bg: resolve_token(&parsed, "field_bg", true, &mut stack).unwrap_or(fallback.field_bg),
        ui_bg: resolve_token(&parsed, "ui_bg", true, &mut stack).unwrap_or(fallback.ui_bg),
        ui_text: resolve_token(&parsed, "ui_text", true, &mut stack).unwrap_or(fallback.ui_text),
        ui_accent: resolve_token(&parsed, "ui_accent", true, &mut stack)
            .unwrap_or(fallback.ui_accent),
        ui_muted,
        ui_bright: resolve_token(&parsed, "ui_bright", true, &mut stack)
            .unwrap_or(ui_bright_default),
    })
}

fn brighten_30_percent(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            brighten_channel_30_percent(r),
            brighten_channel_30_percent(g),
            brighten_channel_30_percent(b),
        ),
        Color::Black => Color::DarkGray,
        Color::DarkGray => Color::Gray,
        Color::Gray => Color::White,
        Color::White => Color::White,
        other => other,
    }
}

fn brighten_channel_30_percent(channel: u8) -> u8 {
    let remaining = 255u16.saturating_sub(u16::from(channel));
    let increase = (remaining * 30 + 50) / 100;
    (u16::from(channel) + increase).min(255) as u8
}

fn resolve_token(
    file: &ThemeFile,
    token: &str,
    prefer_dark: bool,
    stack: &mut Vec<String>,
) -> Option<Color> {
    let value = file.theme.get(token)?;
    resolve_value(file, value, prefer_dark, stack)
}

fn resolve_value(
    file: &ThemeFile,
    value: &ColorValue,
    prefer_dark: bool,
    stack: &mut Vec<String>,
) -> Option<Color> {
    match value {
        ColorValue::String(s) => parse_color_string(file, s, prefer_dark, stack),
        ColorValue::Ansi(code) => Some(Color::Indexed(*code)),
        ColorValue::Variant { dark, light } => {
            let preferred = if prefer_dark {
                dark.as_deref()
            } else {
                light.as_deref()
            };
            let fallback = if prefer_dark {
                light.as_deref()
            } else {
                dark.as_deref()
            };

            preferred
                .and_then(|value| resolve_value(file, value, prefer_dark, stack))
                .or_else(|| {
                    fallback.and_then(|value| resolve_value(file, value, prefer_dark, stack))
                })
        }
    }
}

fn parse_color_string(
    file: &ThemeFile,
    value: &str,
    prefer_dark: bool,
    stack: &mut Vec<String>,
) -> Option<Color> {
    if value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("reset") {
        return Some(Color::Reset);
    }

    if let Some(color) = parse_hex_color(value) {
        return Some(color);
    }

    if let Some(color) = parse_named_ansi_color(value) {
        return Some(color);
    }

    if stack.iter().any(|seen| seen == value) {
        return None;
    }

    let referenced = file.defs.get(value).or_else(|| file.theme.get(value))?;

    stack.push(value.to_owned());
    let resolved = resolve_value(file, referenced, prefer_dark, stack);
    let _ = stack.pop();
    resolved
}

fn parse_named_ansi_color(value: &str) -> Option<Color> {
    match value {
        value if value.eq_ignore_ascii_case("black") => Some(Color::Black),
        value if value.eq_ignore_ascii_case("red") => Some(Color::Red),
        value if value.eq_ignore_ascii_case("green") => Some(Color::Green),
        value if value.eq_ignore_ascii_case("yellow") => Some(Color::Yellow),
        value if value.eq_ignore_ascii_case("blue") => Some(Color::Blue),
        value if value.eq_ignore_ascii_case("magenta") => Some(Color::Magenta),
        value if value.eq_ignore_ascii_case("cyan") => Some(Color::Cyan),
        value if value.eq_ignore_ascii_case("white") => Some(Color::White),
        value if value.eq_ignore_ascii_case("gray") || value.eq_ignore_ascii_case("grey") => {
            Some(Color::Gray)
        }
        value
            if value.eq_ignore_ascii_case("darkgray")
                || value.eq_ignore_ascii_case("darkgrey")
                || value.eq_ignore_ascii_case("dark_gray")
                || value.eq_ignore_ascii_case("dark_grey")
                || value.eq_ignore_ascii_case("dark-gray")
                || value.eq_ignore_ascii_case("dark-grey") =>
        {
            Some(Color::DarkGray)
        }
        value
            if value.eq_ignore_ascii_case("lightred")
                || value.eq_ignore_ascii_case("light_red")
                || value.eq_ignore_ascii_case("light-red") =>
        {
            Some(Color::LightRed)
        }
        value
            if value.eq_ignore_ascii_case("lightgreen")
                || value.eq_ignore_ascii_case("light_green")
                || value.eq_ignore_ascii_case("light-green") =>
        {
            Some(Color::LightGreen)
        }
        value
            if value.eq_ignore_ascii_case("lightyellow")
                || value.eq_ignore_ascii_case("light_yellow")
                || value.eq_ignore_ascii_case("light-yellow") =>
        {
            Some(Color::LightYellow)
        }
        value
            if value.eq_ignore_ascii_case("lightblue")
                || value.eq_ignore_ascii_case("light_blue")
                || value.eq_ignore_ascii_case("light-blue") =>
        {
            Some(Color::LightBlue)
        }
        value
            if value.eq_ignore_ascii_case("lightmagenta")
                || value.eq_ignore_ascii_case("light_magenta")
                || value.eq_ignore_ascii_case("light-magenta") =>
        {
            Some(Color::LightMagenta)
        }
        value
            if value.eq_ignore_ascii_case("lightcyan")
                || value.eq_ignore_ascii_case("light_cyan")
                || value.eq_ignore_ascii_case("light-cyan") =>
        {
            Some(Color::LightCyan)
        }
        value
            if value.eq_ignore_ascii_case("lightwhite")
                || value.eq_ignore_ascii_case("light_white")
                || value.eq_ignore_ascii_case("light-white") =>
        {
            Some(Color::White)
        }
        _ => None,
    }
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::Rgb(red, green, blue))
}

fn is_json_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

fn display_name(id: &str) -> String {
    let mut output = String::new();
    for (idx, part) in id.split(['-', '_']).enumerate() {
        if idx > 0 {
            output.push(' ');
        }

        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            output.push(first.to_ascii_uppercase());
            output.push_str(chars.as_str());
        }
    }
    output
}

fn user_theme_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|config_dir| config_dir.join(USER_THEME_APP_DIR).join("themes"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use ratatui::style::Color;

    use super::{merge_theme_dir, parse_hex_color, parse_theme_from_str_result};

    #[test]
    fn parses_hex_color() {
        assert_eq!(parse_hex_color("#AABBCC"), Some(Color::Rgb(170, 187, 204)));
        assert_eq!(parse_hex_color("invalid"), None);
    }

    #[test]
    fn resolves_defs_and_variants() {
        let json = r##"
        {
          "defs": {
            "bg": "#111111",
            "panel": { "dark": "#222222", "light": "#eeeeee" },
            "accent_ref": "#AA00AA"
          },
          "theme": {
            "snake_head":  "accent_ref",
            "snake_body":  "accent_ref",
            "snake_tail":  "accent_ref",
            "food":        "#FF0000",
            "field_bg":    "bg",
            "ui_bg":       "panel",
            "ui_text":     "#00FF00",
            "ui_accent":   "accent_ref",
            "ui_muted":    "#888888"
          }
        }
        "##;

        let theme = parse_theme_from_str_result("custom", json).expect("theme should parse");
        assert_eq!(theme.field_bg, Color::Rgb(17, 17, 17));
        assert_eq!(theme.ui_bg, Color::Rgb(34, 34, 34));
        assert_eq!(theme.ui_accent, Color::Rgb(170, 0, 170));
        assert_eq!(theme.ui_bright, Color::Rgb(172, 172, 172));
    }

    #[test]
    fn none_maps_to_terminal_default() {
        let json = r##"
        {
          "theme": {
            "snake_head":  "#00CC00",
            "snake_body":  "#00CC00",
            "snake_tail":  "#00CC00",
            "food":        "#FF0000",
            "field_bg":    "none",
            "ui_bg":       "none",
            "ui_text":     "#00FF00",
            "ui_accent":   "#00CC00",
            "ui_muted":    "#777777"
          }
        }
        "##;

        let theme = parse_theme_from_str_result("system", json).expect("theme should parse");
        assert_eq!(theme.field_bg, Color::Reset);
        assert_eq!(theme.ui_bg, Color::Reset);
    }

    #[test]
    fn reset_alias_maps_to_terminal_default() {
        let json = r##"
        {
          "theme": {
            "snake_head":  "#00CC00",
            "snake_body":  "#00CC00",
            "snake_tail":  "#00CC00",
            "food":        "#FF0000",
            "field_bg":    "reset",
            "ui_bg":       "reset",
            "ui_text":     "#00FF00",
            "ui_accent":   "#00CC00",
            "ui_muted":    "#777777"
          }
        }
        "##;

        let theme = parse_theme_from_str_result("system", json).expect("theme should parse");
        assert_eq!(theme.field_bg, Color::Reset);
        assert_eq!(theme.ui_bg, Color::Reset);
    }

    #[test]
    fn named_ansi_colors_are_supported() {
        let json = r##"
        {
          "theme": {
            "snake_head":  "red",
            "snake_body":  "light_green",
            "snake_tail":  "dark-gray",
            "food":        "yellow",
            "field_bg":    "black",
            "ui_bg":       "blue",
            "ui_text":     "white",
            "ui_accent":   "magenta",
            "ui_muted":    "cyan"
          }
        }
        "##;

        let theme = parse_theme_from_str_result("named", json).expect("theme should parse");
        assert_eq!(theme.snake_head, Color::Red);
        assert_eq!(theme.snake_body, Color::LightGreen);
        assert_eq!(theme.snake_tail, Color::DarkGray);
        assert_eq!(theme.food, Color::Yellow);
        assert_eq!(theme.field_bg, Color::Black);
        assert_eq!(theme.ui_bg, Color::Blue);
        assert_eq!(theme.ui_text, Color::White);
        assert_eq!(theme.ui_accent, Color::Magenta);
        assert_eq!(theme.ui_muted, Color::Cyan);
    }

    #[test]
    fn explicit_ui_bright_overrides_default() {
        let json = r##"
        {
          "theme": {
            "snake_head":  "#00CC00",
            "snake_body":  "#00CC00",
            "snake_tail":  "#00CC00",
            "food":        "#FF0000",
            "field_bg":    "#000000",
            "ui_bg":       "#111111",
            "ui_text":     "#00FF00",
            "ui_accent":   "#00CC00",
            "ui_muted":    "#202020",
            "ui_bright":   "#123456"
          }
        }
        "##;

        let theme = parse_theme_from_str_result("custom", json).expect("theme should parse");
        assert_eq!(theme.ui_bright, Color::Rgb(18, 52, 86));
    }

    #[test]
    fn merge_theme_dir_sorts_by_filename() {
        let dir = unique_test_dir("sorted");
        fs::create_dir_all(&dir).expect("test directory should be creatable");

        write_theme_file(&dir.join("z-last.json"), "#00AA00");
        write_theme_file(&dir.join("a-first.json"), "#AA0000");

        let mut order = Vec::new();
        let mut by_id = HashMap::new();
        merge_theme_dir(&dir, &mut order, &mut by_id);

        assert_eq!(order, vec!["a-first", "z-last"]);
        cleanup_test_dir(&dir);
    }

    #[test]
    fn later_directory_overrides_same_theme_id() {
        let low_dir = unique_test_dir("low");
        let high_dir = unique_test_dir("high");
        fs::create_dir_all(&low_dir).expect("low-priority dir should be creatable");
        fs::create_dir_all(&high_dir).expect("high-priority dir should be creatable");

        write_theme_file(&low_dir.join("dup.json"), "#112233");
        write_theme_file(&high_dir.join("dup.json"), "#AABBCC");

        let mut order = Vec::new();
        let mut by_id = HashMap::new();
        merge_theme_dir(&low_dir, &mut order, &mut by_id);
        merge_theme_dir(&high_dir, &mut order, &mut by_id);

        assert_eq!(order, vec!["dup"]);
        let theme = by_id
            .get("dup")
            .expect("theme should be present after override");
        assert_eq!(theme.snake_head, Color::Rgb(170, 187, 204));

        cleanup_test_dir(&low_dir);
        cleanup_test_dir(&high_dir);
    }

    fn write_theme_file(path: &PathBuf, color: &str) {
        let raw = format!(
            "{{\"theme\":{{\"snake_head\":\"{color}\",\"snake_body\":\"{color}\",\"snake_tail\":\"{color}\",\"food\":\"#FF0000\",\"terminal_bg\":\"#000000\",\"field_bg\":\"#000000\",\"ui_bg\":\"#111111\",\"ui_text\":\"#FFFFFF\",\"ui_accent\":\"{color}\",\"ui_muted\":\"#777777\"}}}}"
        );
        fs::write(path, raw).expect("theme file should be writable");
    }

    fn unique_test_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();

        std::env::temp_dir()
            .join("snake-theme-tests")
            .join(format!("{label}-{nanos}"))
    }

    fn cleanup_test_dir(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }
}

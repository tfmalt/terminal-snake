use ratatui::style::Color;
use std::sync::OnceLock;

/// Logical grid dimensions passed through the game as a named type.
///
/// Replaces the anonymous `(u16, u16)` tuple that was used for bounds,
/// making width vs. height unambiguous at every call site.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct GridSize {
    pub width: u16,
    pub height: u16,
}

impl GridSize {
    /// Returns the total number of cells in the grid.
    #[must_use]
    pub fn total_cells(self) -> usize {
        usize::from(self.width) * usize::from(self.height)
    }
}

/// A color theme applied to all visual elements.
///
/// In half-block rendering mode every entity is a solid colored block.
/// The `snake_head`, `snake_body`, `snake_tail`, and `food` fields each
/// specify the solid block color for that entity.
///
/// UI fields (`ui_bg`, `ui_text`, `ui_accent`, `ui_muted`, `ui_bright`) style
/// the HUD and menu panels. JSON theme keys match these field names 1:1.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    /// Solid block color for the snake head segment.
    pub snake_head: Color,
    /// Solid block color for body segments.
    pub snake_body: Color,
    /// Solid block color for the tail segment.
    pub snake_tail: Color,
    /// Solid block color for food items.
    pub food: Color,
    /// Solid block color for super food items.
    pub super_food: Color,
    /// Background color painted across the entire terminal before all other layers.
    /// Set to `Color::Reset` to use the terminal's own default background.
    pub terminal_bg: Color,
    /// Background color for empty play-field cells.
    pub field_bg: Color,
    /// Background color for menu panels and popups.
    pub ui_bg: Color,
    /// Primary text color used in the HUD, score display, and menu body.
    pub ui_text: Color,
    /// Accent color for menu titles and selected-option highlights.
    pub ui_accent: Color,
    /// Subdued color for footer hints and secondary labels.
    pub ui_muted: Color,
    /// Brighter UI accent for standout secondary text and highlights.
    pub ui_bright: Color,
}

/// Emergency fallback theme used when no external/bundled themes load.
#[must_use]
pub fn fallback_theme() -> Theme {
    Theme {
        name: "fallback".to_owned(),
        snake_head: Color::White,
        snake_body: Color::Blue,
        snake_tail: Color::DarkGray,
        food: Color::Red,
        super_food: Color::Yellow,
        terminal_bg: Color::Reset,
        field_bg: Color::Black,
        ui_bg: Color::DarkGray,
        ui_text: Color::White,
        ui_accent: Color::Green,
        ui_muted: Color::DarkGray,
        ui_bright: Color::Gray,
    }
}

/// Horizontal margin (columns) around the gameplay viewport.
pub const PLAY_AREA_MARGIN_X: u16 = 2;

/// Vertical margin (rows) around the gameplay viewport.
pub const PLAY_AREA_MARGIN_Y: u16 = 1;

/// Bottom margin (rows) below the HUD.
pub const HUD_BOTTOM_MARGIN_Y: u16 = 1;

/// Upper half-block glyph for compositing.
pub const GLYPH_HALF_UPPER: &str = "▀";

/// Lower half-block glyph for compositing.
pub const GLYPH_HALF_LOWER: &str = "▄";

/// Up indicator used in start menu speed controls.
pub const GLYPH_INDICATOR_UP: &str = "▲";

/// Down indicator used in start menu speed controls.
pub const GLYPH_INDICATOR_DOWN: &str = "▼";

/// Filled square marker used in HUD counters.
pub const GLYPH_MARKER_SQUARE: &str = "■";

/// Runtime-selected glyph mode.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GlyphMode {
    Unicode,
    Ascii,
}

/// Glyph palette used by rendering paths.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct GlyphPalette {
    pub half_upper: &'static str,
    pub half_lower: &'static str,
    pub solid: &'static str,
    pub table_separator: &'static str,
}

impl GlyphMode {
    /// Resolves glyph mode from CLI and optional environment override.
    #[must_use]
    pub fn resolve(force_ascii: bool) -> Self {
        let env = std::env::var("TERMINAL_SNAKE_GLYPHS").ok();
        glyph_mode_from_inputs(force_ascii, env.as_deref())
    }
}

fn glyph_mode_from_inputs(force_ascii: bool, env_value: Option<&str>) -> GlyphMode {
    if force_ascii {
        return GlyphMode::Ascii;
    }

    if env_value.is_some_and(|value| value.eq_ignore_ascii_case("ascii")) {
        GlyphMode::Ascii
    } else {
        GlyphMode::Unicode
    }
}

static GLYPH_PALETTE: OnceLock<GlyphPalette> = OnceLock::new();

/// Configures the global glyph palette. First call wins.
pub fn configure_glyphs(mode: GlyphMode) {
    let _ = GLYPH_PALETTE.set(match mode {
        GlyphMode::Unicode => GlyphPalette {
            half_upper: GLYPH_HALF_UPPER,
            half_lower: GLYPH_HALF_LOWER,
            solid: "█",
            table_separator: "│",
        },
        GlyphMode::Ascii => GlyphPalette {
            half_upper: "#",
            half_lower: "#",
            solid: "#",
            table_separator: "|",
        },
    });
}

/// Returns the active glyph palette.
#[must_use]
pub fn glyphs() -> &'static GlyphPalette {
    GLYPH_PALETTE.get_or_init(|| GlyphPalette {
        half_upper: GLYPH_HALF_UPPER,
        half_lower: GLYPH_HALF_LOWER,
        solid: "█",
        table_separator: "│",
    })
}

/// Base tick interval in milliseconds.
pub const DEFAULT_TICK_INTERVAL_MS: u64 = 200;

/// Minimum tick interval in milliseconds.
pub const MIN_TICK_INTERVAL_MS: u64 = 60;

/// Food items eaten per speed level increase.
pub const FOOD_PER_SPEED_LEVEL: u32 = 5;

/// Minimum selectable starting speed level.
pub const MIN_START_SPEED_LEVEL: u32 = 1;

/// Maximum selectable starting speed level.
pub const MAX_START_SPEED_LEVEL: u32 = 15;

#[cfg(test)]
mod tests {
    use super::{GlyphMode, glyph_mode_from_inputs};

    #[test]
    fn glyph_mode_resolve_prefers_cli_flag() {
        assert_eq!(
            glyph_mode_from_inputs(true, Some("unicode")),
            GlyphMode::Ascii
        );
    }

    #[test]
    fn glyph_mode_uses_ascii_when_env_requests_it() {
        assert_eq!(
            glyph_mode_from_inputs(false, Some("ascii")),
            GlyphMode::Ascii
        );
    }

    #[test]
    fn glyph_mode_defaults_to_unicode() {
        assert_eq!(
            glyph_mode_from_inputs(false, Some("unicode")),
            GlyphMode::Unicode
        );
    }
}

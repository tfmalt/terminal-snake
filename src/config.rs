use ratatui::style::Color;
use ratatui::symbols::border;

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
#[derive(Debug)]
pub struct Theme {
    pub name: &'static str,
    /// Background color shared by all snake segments.
    pub snake_bg: Color,
    /// Foreground color for the head glyph.
    pub snake_head_fg: Color,
    /// Foreground color for body segment glyphs.
    pub snake_body_fg: Color,
    /// Foreground color for the tail glyph.
    pub snake_tail_fg: Color,
    pub food: Color,
    pub border_fg: Color,
    pub border_bg: Color,
    pub hud_score: Color,
    pub menu_title: Color,
    pub menu_footer: Color,
}

/// Classic blue snake on dark theme.
pub const THEME_CLASSIC: Theme = Theme {
    name: "Classic",
    snake_bg: Color::Blue,
    snake_head_fg: Color::LightBlue,
    snake_body_fg: Color::LightBlue,
    snake_tail_fg: Color::LightBlue,
    food: Color::Red,
    border_fg: Color::White,
    border_bg: Color::DarkGray,
    hud_score: Color::White,
    menu_title: Color::Green,
    menu_footer: Color::DarkGray,
};

/// Ocean cyan theme.
pub const THEME_OCEAN: Theme = Theme {
    name: "Ocean",
    snake_bg: Color::Cyan,
    snake_head_fg: Color::LightCyan,
    snake_body_fg: Color::LightCyan,
    snake_tail_fg: Color::LightCyan,
    food: Color::Yellow,
    border_fg: Color::Cyan,
    border_bg: Color::DarkGray,
    hud_score: Color::Cyan,
    menu_title: Color::Cyan,
    menu_footer: Color::DarkGray,
};

/// Neon magenta/yellow theme.
pub const THEME_NEON: Theme = Theme {
    name: "Neon",
    snake_bg: Color::Magenta,
    snake_head_fg: Color::LightMagenta,
    snake_body_fg: Color::LightMagenta,
    snake_tail_fg: Color::LightMagenta,
    food: Color::Yellow,
    border_fg: Color::Magenta,
    border_bg: Color::Black,
    hud_score: Color::Magenta,
    menu_title: Color::Magenta,
    menu_footer: Color::DarkGray,
};

/// Monochrome theme (terminal default colors, no styling).
pub const THEME_MONO: Theme = Theme {
    name: "Mono",
    snake_bg: Color::Reset,
    snake_head_fg: Color::Reset,
    snake_body_fg: Color::Reset,
    snake_tail_fg: Color::Reset,
    food: Color::Reset,
    border_fg: Color::Reset,
    border_bg: Color::Reset,
    hud_score: Color::Reset,
    menu_title: Color::Reset,
    menu_footer: Color::Reset,
};

/// All available themes in cycle order.
pub const THEMES: &[Theme] = &[THEME_CLASSIC, THEME_OCEAN, THEME_NEON, THEME_MONO];

/// Half-block border set: solid side faces the play area.
///
/// - Top row + top corners: `▄` (solid bottom → play area below)
/// - Bottom row + bottom corners: `▀` (solid top → play area above)
/// - Left column: `█` (fully solid)
/// - Right column: `█` (fully solid)
pub const BORDER_HALF_BLOCK: border::Set = border::Set {
    top_left: "▄",
    top_right: "▄",
    bottom_left: "▀",
    bottom_right: "▀",
    vertical_left: "█",
    vertical_right: "█",
    horizontal_top: "▄",
    horizontal_bottom: "▀",
};

/// Base tick interval in milliseconds.
pub const DEFAULT_TICK_INTERVAL_MS: u64 = 200;

/// Minimum tick interval in milliseconds.
pub const MIN_TICK_INTERVAL_MS: u64 = 60;

/// Score needed per speed level increase.
pub const POINTS_PER_SPEED_LEVEL: u32 = 5;

/// Head glyph when moving up.
pub const GLYPH_SNAKE_HEAD_UP: &str = "▲";

/// Head glyph when moving down.
pub const GLYPH_SNAKE_HEAD_DOWN: &str = "▼";

/// Head glyph when moving left.
pub const GLYPH_SNAKE_HEAD_LEFT: &str = "◀";

/// Head glyph when moving right.
pub const GLYPH_SNAKE_HEAD_RIGHT: &str = "▶";

/// Body glyph — a centered square so the cell background is visible around it.
pub const GLYPH_SNAKE_BODY: &str = "■";

/// Tail glyph.
pub const GLYPH_SNAKE_TAIL: &str = "▓";

/// Food glyph.
pub const GLYPH_FOOD: &str = "●";

/// Border top-left glyph.
pub const GLYPH_BORDER_TOP_LEFT: &str = "╔";

/// Border horizontal glyph.
pub const GLYPH_BORDER_HORIZONTAL: &str = "═";

/// Border top-right glyph.
pub const GLYPH_BORDER_TOP_RIGHT: &str = "╗";

/// Border vertical glyph.
pub const GLYPH_BORDER_VERTICAL: &str = "║";

/// Border bottom-right glyph.
pub const GLYPH_BORDER_BOTTOM_RIGHT: &str = "╝";

/// Border bottom-left glyph.
pub const GLYPH_BORDER_BOTTOM_LEFT: &str = "╚";

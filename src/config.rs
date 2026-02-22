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
///
/// In half-block rendering mode every entity is a solid colored block.
/// The `snake_head`, `snake_body`, `snake_tail`, and `food` fields each
/// specify the solid block color for that entity.
#[derive(Debug)]
pub struct Theme {
    pub name: &'static str,
    /// Solid block color for the snake head.
    pub snake_head: Color,
    /// Solid block color for body segments.
    pub snake_body: Color,
    /// Solid block color for the tail segment.
    pub snake_tail: Color,
    /// Solid block color for food.
    pub food: Color,
    /// Background color for empty play-area cells.
    pub play_bg: Color,
    pub border_fg: Color,
    pub border_bg: Color,
    pub hud_score: Color,
    pub menu_title: Color,
    pub menu_footer: Color,
}

/// Classic blue snake on dark theme.
pub const THEME_CLASSIC: Theme = Theme {
    name: "Classic",
    snake_head: Color::White,
    snake_body: Color::Blue,
    snake_tail: Color::DarkGray,
    food: Color::Red,
    play_bg: Color::Black,
    border_fg: Color::White,
    border_bg: Color::DarkGray,
    hud_score: Color::White,
    menu_title: Color::Green,
    menu_footer: Color::DarkGray,
};

/// Ocean cyan theme.
pub const THEME_OCEAN: Theme = Theme {
    name: "Ocean",
    snake_head: Color::White,
    snake_body: Color::Cyan,
    snake_tail: Color::DarkGray,
    food: Color::Yellow,
    play_bg: Color::Black,
    border_fg: Color::Cyan,
    border_bg: Color::DarkGray,
    hud_score: Color::Cyan,
    menu_title: Color::Cyan,
    menu_footer: Color::DarkGray,
};

/// Neon magenta/yellow theme.
pub const THEME_NEON: Theme = Theme {
    name: "Neon",
    snake_head: Color::White,
    snake_body: Color::Magenta,
    snake_tail: Color::DarkGray,
    food: Color::Yellow,
    play_bg: Color::Black,
    border_fg: Color::Magenta,
    border_bg: Color::Black,
    hud_score: Color::Magenta,
    menu_title: Color::Magenta,
    menu_footer: Color::DarkGray,
};

/// All available themes in cycle order.
pub const THEMES: &[Theme] = &[THEME_CLASSIC, THEME_OCEAN, THEME_NEON];

/// Half-block border set: solid side faces the play area.
///
/// - Top row + top corners: `▄` (solid bottom -> play area below)
/// - Bottom row + bottom corners: `▀` (solid top -> play area above)
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

/// Upper half-block glyph for compositing.
pub const GLYPH_HALF_UPPER: &str = "▀";

/// Lower half-block glyph for compositing.
pub const GLYPH_HALF_LOWER: &str = "▄";

/// Base tick interval in milliseconds.
pub const DEFAULT_TICK_INTERVAL_MS: u64 = 200;

/// Minimum tick interval in milliseconds.
pub const MIN_TICK_INTERVAL_MS: u64 = 60;

/// Score needed per speed level increase.
pub const POINTS_PER_SPEED_LEVEL: u32 = 5;

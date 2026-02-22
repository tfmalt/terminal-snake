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

/// Body glyph.
pub const GLYPH_SNAKE_BODY: &str = "█";

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

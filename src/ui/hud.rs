use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::colors::{blend_color, ease_out_cubic};
use crate::config::{GLYPH_MARKER_SQUARE, HUD_BOTTOM_MARGIN_Y, PLAY_AREA_MARGIN_X, Theme, glyphs};
use crate::game::GameState;
use crate::platform::Platform;

const HUD_INNER_MARGIN_X: u16 = 1;
const VALUE_FLASH_HOLD_DURATION: Duration = Duration::from_secs(1);
const VALUE_FLASH_FADE_DURATION: Duration = Duration::from_secs(2);
const VALUE_FLASH_DURATION: Duration =
    Duration::from_secs(VALUE_FLASH_HOLD_DURATION.as_secs() + VALUE_FLASH_FADE_DURATION.as_secs());

/// Per-value flash timestamps for HUD value transitions.
#[derive(Debug, Clone, Copy, Default)]
pub struct HudValueFlash {
    pub length_changed_at: Option<Instant>,
    pub level_changed_at: Option<Instant>,
    pub score_changed_at: Option<Instant>,
    pub high_score_changed_at: Option<Instant>,
    pub dimensions_changed_at: Option<Instant>,
    pub foods_to_next_level_changed_at: Option<Instant>,
    pub food_count_changed_at: Option<Instant>,
    pub next_points_changed_at: Option<Instant>,
    pub bonus_multiplier_changed_at: Option<Instant>,
    pub coverage_changed_at: Option<Instant>,
}

/// Supplemental values displayed by the HUD rows.
#[derive(Debug, Clone)]
pub struct HudInfo<'a> {
    pub high_score: u32,
    pub game_over_reference_high_score: u32,
    pub theme: &'a Theme,
    /// Whether the debug row is enabled (`--debug` flag).
    pub debug: bool,
    /// Pre-formatted debug string; empty when `debug` is false.
    pub debug_line: String,
    /// Wall-clock instant of this frame render.
    pub now: Instant,
    /// Last-change timestamps for HUD values.
    pub value_flash: HudValueFlash,
}

/// Grouped context for top HUD line rendering, replacing many function parameters.
struct TopLineContext {
    length: usize,
    level: u32,
    score: u32,
    high_score: u32,
    reference_high_score: u32,
    available_width: usize,
    highlight_color: Color,
    value_color: Color,
    muted_color: Color,
    value_flash: HudValueFlash,
    now: Instant,
}

/// Grouped context for bottom HUD line rendering, replacing many function parameters.
struct BottomLineContext<'a> {
    dimensions: &'a str,
    foods_to_next_level: &'a str,
    food_count: &'a str,
    next_food_points: &'a str,
    bonus_multiplier: &'a str,
    food_color: Color,
    value_color: Color,
    highlight_color: Color,
    value_flash: HudValueFlash,
    now: Instant,
}

/// Renders the two-line HUD and returns the remaining play area above it.
#[must_use]
pub fn render_hud(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &GameState,
    _platform: Platform,
    info: &HudInfo<'_>,
) -> Rect {
    let debug_height = u16::from(info.debug);
    let [
        play_area,
        score_area,
        status_area,
        debug_area,
        bottom_margin,
    ] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(debug_height),
        Constraint::Length(HUD_BOTTOM_MARGIN_Y),
    ])
    .areas(area);

    let score_band = inset_horizontal(score_area, PLAY_AREA_MARGIN_X);
    let status_band = inset_horizontal(status_area, PLAY_AREA_MARGIN_X);
    let debug_band = inset_horizontal(debug_area, PLAY_AREA_MARGIN_X);

    let score_area = inset_horizontal(score_band, HUD_INNER_MARGIN_X);
    let status_area = inset_horizontal(status_band, HUD_INNER_MARGIN_X);
    let debug_area = inset_horizontal(debug_band, HUD_INNER_MARGIN_X);

    // Paint the HUD rows with field_bg before individual widgets render on top.
    // Without this, the terminal_bg painted by renderer.rs bleeds through any
    // paragraph that does not set an explicit bg, and the unused left quarter
    // of the status row would remain terminal_bg.
    let hud_bg = Style::default().bg(info.theme.field_bg);
    frame.render_widget(Paragraph::new("").style(hud_bg), score_band);
    frame.render_widget(Paragraph::new("").style(hud_bg), status_band);

    // Top status line: Length | Level | Score | Hi
    let top_ctx = TopLineContext {
        length: state.snake.len(),
        level: state.speed_level,
        score: state.score,
        high_score: info.high_score,
        reference_high_score: info.game_over_reference_high_score,
        available_width: usize::from(score_area.width),
        highlight_color: info.theme.ui_accent,
        value_color: info.theme.ui_bright,
        muted_color: info.theme.ui_muted,
        value_flash: info.value_flash,
        now: info.now,
    };
    frame.render_widget(
        Paragraph::new(top_info_line(&top_ctx))
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray)),
        score_area,
    );

    // Bottom status line: dimensions, food count, next points, bonus multiplier
    let dimensions_text = format!("{}x{}", state.bounds().width, state.bounds().height);
    let foods_to_next_level_text = state.foods_until_next_speed_level().to_string();
    let food_count_text = state.calculated_food_count().to_string();
    let next_food_points_text = state.ordinary_food_projected_points().to_string();
    let bonus_multiplier_text = format!("{:.2}x", state.ordinary_food_projected_multiplier());
    let bot_ctx = BottomLineContext {
        dimensions: dimensions_text.as_str(),
        foods_to_next_level: foods_to_next_level_text.as_str(),
        food_count: food_count_text.as_str(),
        next_food_points: next_food_points_text.as_str(),
        bonus_multiplier: bonus_multiplier_text.as_str(),
        food_color: info.theme.food,
        value_color: info.theme.ui_muted,
        highlight_color: info.theme.ui_accent,
        value_flash: info.value_flash,
        now: info.now,
    };
    frame.render_widget(
        Paragraph::new(bottom_info_line(&bot_ctx))
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray)),
        status_area,
    );

    if info.debug {
        frame.render_widget(Paragraph::new("").style(hud_bg), debug_band);
        let debug_width = bottom_info_width(
            dimensions_text.as_str(),
            foods_to_next_level_text.as_str(),
            food_count_text.as_str(),
            next_food_points_text.as_str(),
            bonus_multiplier_text.as_str(),
        )
        .min(u16::MAX as usize) as u16;
        let [debug_left, debug_right] =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(debug_width)])
                .areas(debug_area);

        frame.render_widget(
            Paragraph::new(Line::from(info.debug_line.as_str()))
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::DarkGray)),
            debug_left,
        );
        let debug_bot_ctx = BottomLineContext {
            dimensions: dimensions_text.as_str(),
            foods_to_next_level: foods_to_next_level_text.as_str(),
            food_count: food_count_text.as_str(),
            next_food_points: next_food_points_text.as_str(),
            bonus_multiplier: bonus_multiplier_text.as_str(),
            food_color: info.theme.food,
            value_color: info.theme.ui_muted,
            highlight_color: info.theme.ui_accent,
            value_flash: info.value_flash,
            now: info.now,
        };
        frame.render_widget(
            Paragraph::new(bottom_info_line(&debug_bot_ctx))
                .alignment(Alignment::Right)
                .style(Style::default().fg(Color::DarkGray)),
            debug_right,
        );
    }

    render_hud_bottom_margin(frame, bottom_margin, info.theme);

    play_area
}

fn inset_horizontal(area: Rect, margin: u16) -> Rect {
    let total_margin = margin.saturating_mul(2);
    Rect {
        x: area.x.saturating_add(margin),
        y: area.y,
        width: area.width.saturating_sub(total_margin),
        height: area.height,
    }
}

fn render_hud_bottom_margin(frame: &mut Frame<'_>, bottom_margin: Rect, theme: &Theme) {
    let margin_band = inset_horizontal(bottom_margin, PLAY_AREA_MARGIN_X);
    let style = Style::default().fg(theme.field_bg).bg(theme.terminal_bg);
    let half_upper = glyphs().half_upper;
    let buffer = frame.buffer_mut();

    for y in margin_band.y..margin_band.bottom() {
        for x in margin_band.x..margin_band.right() {
            buffer.set_string(x, y, half_upper, style);
        }
    }
}

fn top_info_line(ctx: &TopLineContext) -> Line<'static> {
    let hide_score = ctx.score == ctx.high_score && ctx.score > ctx.reference_high_score;
    let has_new_high_score = ctx.score > ctx.reference_high_score;
    let use_compact_labels = top_info_width(
        ctx.length,
        ctx.level,
        ctx.score,
        ctx.high_score,
        false,
        hide_score,
    ) > ctx.available_width;
    let highlight_scores = ctx.score == ctx.high_score;
    let sep = format!(" {} ", glyphs().table_separator);
    let length_label = if use_compact_labels { "L" } else { "Length" };
    let level_label = if use_compact_labels { "V" } else { "Level" };
    let score_label = if use_compact_labels { "S" } else { "Score" };
    let high_score_label = if use_compact_labels { "H" } else { "Hi" };
    let length_color = flash_color(
        ctx.value_color,
        ctx.highlight_color,
        ctx.value_flash.length_changed_at,
        ctx.now,
    );
    let level_color = flash_color(
        ctx.value_color,
        ctx.highlight_color,
        ctx.value_flash.level_changed_at,
        ctx.now,
    );
    let score_base_color = if highlight_scores {
        ctx.highlight_color
    } else {
        ctx.value_color
    };
    let high_score_base_color = if has_new_high_score {
        ctx.highlight_color
    } else {
        ctx.muted_color
    };
    let score_color = flash_color(
        score_base_color,
        ctx.highlight_color,
        ctx.value_flash.score_changed_at,
        ctx.now,
    );
    let high_score_color = flash_color(
        high_score_base_color,
        ctx.highlight_color,
        ctx.value_flash.high_score_changed_at,
        ctx.now,
    );
    let length_style = Style::default().fg(length_color);
    let level_style = Style::default().fg(level_color);
    let score_style = Style::default().fg(score_color);
    let high_score_style = Style::default().fg(high_score_color);
    let mut spans = vec![
        Span::raw(format!("{length_label}: ")),
        Span::styled(ctx.length.to_string(), length_style),
        Span::raw(sep.clone()),
        Span::raw(format!("{level_label}: ")),
        Span::styled(ctx.level.to_string(), level_style),
        Span::raw(sep.clone()),
    ];

    if !hide_score {
        spans.push(Span::raw(format!("{score_label}: ")));
        spans.push(Span::styled(ctx.score.to_string(), score_style));
        spans.push(Span::raw(sep));
    }

    spans.push(Span::raw(format!("{high_score_label}: ")));
    spans.push(Span::styled(ctx.high_score.to_string(), high_score_style));

    Line::from(spans)
}

fn top_info_width(
    length: usize,
    level: u32,
    score: u32,
    high_score: u32,
    compact: bool,
    hide_score: bool,
) -> usize {
    let length_label = if compact { "L" } else { "Length" };
    let level_label = if compact { "V" } else { "Level" };
    let score_label = if compact { "S" } else { "Score" };
    let high_score_label = if compact { "H" } else { "Hi" };
    let sep_width = format!(" {} ", glyphs().table_separator).chars().count();

    let mut width = format!("{length_label}: {length}").chars().count()
        + sep_width
        + format!("{level_label}: {level}").chars().count()
        + sep_width;

    if !hide_score {
        width += format!("{score_label}: {score}").chars().count() + sep_width;
    }

    width + format!("{high_score_label}: {high_score}").chars().count()
}

fn bottom_info_line<'a>(ctx: &BottomLineContext<'a>) -> Line<'a> {
    let sep = format!(" {} ", glyphs().table_separator);
    let dimensions_style = Style::default().fg(flash_color(
        ctx.value_color,
        ctx.highlight_color,
        ctx.value_flash.dimensions_changed_at,
        ctx.now,
    ));
    let food_count_style = Style::default().fg(flash_color(
        ctx.value_color,
        ctx.highlight_color,
        ctx.value_flash.food_count_changed_at,
        ctx.now,
    ));
    let foods_to_next_level_style = Style::default().fg(flash_color(
        ctx.value_color,
        ctx.highlight_color,
        ctx.value_flash.foods_to_next_level_changed_at,
        ctx.now,
    ));
    let next_points_style = Style::default().fg(flash_color(
        ctx.value_color,
        ctx.highlight_color,
        ctx.value_flash.next_points_changed_at,
        ctx.now,
    ));
    let bonus_multiplier_style = Style::default().fg(flash_color(
        ctx.value_color,
        ctx.highlight_color,
        ctx.value_flash.bonus_multiplier_changed_at,
        ctx.now,
    ));
    Line::from(vec![
        Span::styled(ctx.dimensions.to_string(), dimensions_style),
        Span::raw(sep.clone()),
        Span::raw("n: "),
        Span::styled(
            ctx.foods_to_next_level.to_string(),
            foods_to_next_level_style,
        ),
        Span::raw(sep.clone()),
        Span::styled(food_count_marker(), Style::default().fg(ctx.food_color)),
        Span::raw(": "),
        Span::styled(ctx.food_count.to_string(), food_count_style),
        Span::raw(sep.clone()),
        Span::raw("v: "),
        Span::styled(ctx.next_food_points.to_string(), next_points_style),
        Span::raw(sep.clone()),
        Span::raw("b: "),
        Span::styled(ctx.bonus_multiplier.to_string(), bonus_multiplier_style),
    ])
}

/// Computes the flash color for a HUD value given its base color, accent,
/// change timestamp, and current time.
///
/// Returns `accent` during the hold phase, blends accent→base during the
/// fade phase, and returns `base` when the flash has expired.
pub(crate) fn flash_color(
    base: Color,
    accent: Color,
    changed_at: Option<Instant>,
    now: Instant,
) -> Color {
    let Some(changed_at) = changed_at else {
        return base;
    };
    let elapsed = now.saturating_duration_since(changed_at);
    if elapsed >= VALUE_FLASH_DURATION {
        return base;
    }
    if elapsed <= VALUE_FLASH_HOLD_DURATION {
        return accent;
    }

    let fade_elapsed = elapsed - VALUE_FLASH_HOLD_DURATION;
    let t = fade_elapsed.as_secs_f32() / VALUE_FLASH_FADE_DURATION.as_secs_f32();
    blend_color(accent, base, ease_out_cubic(t))
}

fn food_count_marker() -> &'static str {
    if glyphs().solid == "#" {
        "#"
    } else {
        GLYPH_MARKER_SQUARE
    }
}

fn bottom_info_width(
    dimensions: &str,
    foods_to_next_level: &str,
    food_count: &str,
    next_food_points: &str,
    bonus_multiplier: &str,
) -> usize {
    // {dimensions} │ n: {foods_to_next_level} │ ■: {food_count} │ v: {next_food_points} │ b: {bonus_multiplier}
    dimensions.chars().count()
        + 3 // " │ "
        + 3 // "n: "
        + foods_to_next_level.chars().count()
        + 3 // " │ "
        + 1 // marker
        + 2 // ": "
        + food_count.chars().count()
        + 3 // " │ "
        + 3 // "v: "
        + next_food_points.chars().count()
        + 3 // " │ "
        + 3 // "b: "
        + bonus_multiplier.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_color_returns_base_when_no_change() {
        let base = Color::Rgb(100, 100, 100);
        let accent = Color::Rgb(255, 255, 0);
        let now = Instant::now();
        assert_eq!(flash_color(base, accent, None, now), base);
    }

    #[test]
    fn flash_color_returns_accent_during_hold_phase() {
        let base = Color::Rgb(100, 100, 100);
        let accent = Color::Rgb(255, 255, 0);
        let now = Instant::now();
        // Changed just now — should be in hold phase.
        assert_eq!(flash_color(base, accent, Some(now), now), accent);
    }

    #[test]
    fn flash_color_returns_base_after_expiry() {
        let base = Color::Rgb(100, 100, 100);
        let accent = Color::Rgb(255, 255, 0);
        let changed_at = Instant::now();
        // 4 seconds later — beyond HOLD(1s) + FADE(2s) = 3s total.
        let now = changed_at + Duration::from_secs(4);
        assert_eq!(flash_color(base, accent, Some(changed_at), now), base);
    }

    #[test]
    fn flash_color_blends_during_fade_phase() {
        let base = Color::Rgb(0, 0, 0);
        let accent = Color::Rgb(255, 255, 255);
        let changed_at = Instant::now();
        // 2 seconds later — 1s into the fade phase (halfway through 2s fade).
        let now = changed_at + Duration::from_secs(2);
        let result = flash_color(base, accent, Some(changed_at), now);
        // Should be between accent and base — not exactly either.
        assert_ne!(result, accent);
        assert_ne!(result, base);
    }

    #[test]
    fn top_info_width_full_labels() {
        let width = top_info_width(5, 1, 100, 200, false, false);
        // "Length: 5 │ Level: 1 │ Score: 100 │ Hi: 200"
        assert!(width > 30);
    }

    #[test]
    fn top_info_width_compact_labels_shorter() {
        let full = top_info_width(5, 1, 100, 200, false, false);
        let compact = top_info_width(5, 1, 100, 200, true, false);
        assert!(compact < full);
    }

    #[test]
    fn top_info_width_hidden_score_shorter() {
        let with_score = top_info_width(5, 1, 100, 200, false, false);
        let without_score = top_info_width(5, 1, 100, 200, false, true);
        assert!(without_score < with_score);
    }

    #[test]
    fn bottom_info_width_consistent() {
        let w = bottom_info_width("40x20", "6", "3", "10", "1.50x");
        // Should be: "40x20" + " │ " + "n: " + "6" + " │ " + "■" + ": " + "3" + " │ " + "v: " + "10" + " │ " + "b: " + "1.50x"
        // = 5 + 3 + 3 + 1 + 3 + 1 + 2 + 1 + 3 + 3 + 2 + 3 + 3 + 5 = 38
        assert_eq!(w, 38);
    }
}

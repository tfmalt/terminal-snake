use std::time::{Duration, Instant};

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::config::{glyphs, Theme, GLYPH_MARKER_SQUARE, HUD_BOTTOM_MARGIN_Y, PLAY_AREA_MARGIN_X};
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
    let [play_area, score_area, status_area, debug_area, bottom_margin] = Layout::vertical([
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
    frame.render_widget(
        Paragraph::new(top_info_line(
            state.snake.len(),
            state.speed_level,
            state.score,
            info.high_score,
            info.game_over_reference_high_score,
            usize::from(score_area.width),
            info.theme.ui_accent,
            info.theme.ui_bright,
            info.theme.ui_muted,
            info.value_flash,
            info.now,
        ))
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::DarkGray)),
        score_area,
    );

    // Bottom status line: dimensions, food count, next points, bonus multiplier
    let dimensions_text = format!("{}x{}", state.bounds().width, state.bounds().height);
    let food_count_text = state.calculated_food_count().to_string();
    let next_food_points_text = state.ordinary_food_projected_points().to_string();
    let bonus_multiplier_text = format!("{:.2}x", state.ordinary_food_projected_multiplier());
    frame.render_widget(
        Paragraph::new(bottom_info_line(
            dimensions_text.as_str(),
            food_count_text.as_str(),
            next_food_points_text.as_str(),
            bonus_multiplier_text.as_str(),
            info.theme.food,
            info.theme.ui_muted,
            info.theme.ui_accent,
            info.value_flash,
            info.now,
        ))
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::DarkGray)),
        status_area,
    );

    if info.debug {
        frame.render_widget(Paragraph::new("").style(hud_bg), debug_band);
        let debug_width = bottom_info_width(
            dimensions_text.as_str(),
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
        frame.render_widget(
            Paragraph::new(bottom_info_line(
                dimensions_text.as_str(),
                food_count_text.as_str(),
                next_food_points_text.as_str(),
                bonus_multiplier_text.as_str(),
                info.theme.food,
                info.theme.ui_muted,
                info.theme.ui_accent,
                info.value_flash,
                info.now,
            ))
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

fn top_info_line(
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
) -> Line<'static> {
    let hide_score = score == high_score && score > reference_high_score;
    let has_new_high_score = score > reference_high_score;
    let use_compact_labels =
        top_info_width(length, level, score, high_score, false, hide_score) > available_width;
    let highlight_scores = score == high_score;
    let sep = format!(" {} ", glyphs().table_separator);
    let length_label = if use_compact_labels { "L" } else { "Length" };
    let level_label = if use_compact_labels { "V" } else { "Level" };
    let score_label = if use_compact_labels { "S" } else { "Score" };
    let high_score_label = if use_compact_labels { "H" } else { "Hi" };
    let length_color = flash_color(
        value_color,
        highlight_color,
        value_flash.length_changed_at,
        now,
    );
    let level_color = flash_color(
        value_color,
        highlight_color,
        value_flash.level_changed_at,
        now,
    );
    let score_base_color = if highlight_scores {
        highlight_color
    } else {
        value_color
    };
    let high_score_base_color = if has_new_high_score {
        highlight_color
    } else {
        muted_color
    };
    let score_color = flash_color(
        score_base_color,
        highlight_color,
        value_flash.score_changed_at,
        now,
    );
    let high_score_color = flash_color(
        high_score_base_color,
        highlight_color,
        value_flash.high_score_changed_at,
        now,
    );
    let length_style = Style::default().fg(length_color);
    let level_style = Style::default().fg(level_color);
    let score_style = Style::default().fg(score_color);
    let high_score_style = Style::default().fg(high_score_color);
    let mut spans = vec![
        Span::raw(format!("{length_label}: ")),
        Span::styled(length.to_string(), length_style),
        Span::raw(sep.clone()),
        Span::raw(format!("{level_label}: ")),
        Span::styled(level.to_string(), level_style),
        Span::raw(sep.clone()),
    ];

    if !hide_score {
        spans.push(Span::raw(format!("{score_label}: ")));
        spans.push(Span::styled(score.to_string(), score_style));
        spans.push(Span::raw(sep));
    }

    spans.push(Span::raw(format!("{high_score_label}: ")));
    spans.push(Span::styled(high_score.to_string(), high_score_style));

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

fn bottom_info_line<'a>(
    dimensions: &'a str,
    food_count: &'a str,
    next_food_points: &'a str,
    bonus_multiplier: &'a str,
    food_color: Color,
    value_color: Color,
    highlight_color: Color,
    value_flash: HudValueFlash,
    now: Instant,
) -> Line<'a> {
    let sep = format!(" {} ", glyphs().table_separator);
    let dimensions_style = Style::default().fg(flash_color(
        value_color,
        highlight_color,
        value_flash.dimensions_changed_at,
        now,
    ));
    let food_count_style = Style::default().fg(flash_color(
        value_color,
        highlight_color,
        value_flash.food_count_changed_at,
        now,
    ));
    let next_points_style = Style::default().fg(flash_color(
        value_color,
        highlight_color,
        value_flash.next_points_changed_at,
        now,
    ));
    let bonus_multiplier_style = Style::default().fg(flash_color(
        value_color,
        highlight_color,
        value_flash.bonus_multiplier_changed_at,
        now,
    ));
    Line::from(vec![
        Span::styled(dimensions.to_string(), dimensions_style),
        Span::raw(sep.clone()),
        Span::styled(food_count_marker(), Style::default().fg(food_color)),
        Span::raw(": "),
        Span::styled(food_count.to_string(), food_count_style),
        Span::raw(sep.clone()),
        Span::raw("v: "),
        Span::styled(next_food_points.to_string(), next_points_style),
        Span::raw(sep.clone()),
        Span::raw("b: "),
        Span::styled(bonus_multiplier.to_string(), bonus_multiplier_style),
    ])
}

fn flash_color(base: Color, accent: Color, changed_at: Option<Instant>, now: Instant) -> Color {
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

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn blend_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (color_to_rgb(from), color_to_rgb(to)) {
        (Some((fr, fg, fb)), Some((tr, tg, tb))) => {
            Color::Rgb(lerp_u8(fr, tr, t), lerp_u8(fg, tg, t), lerp_u8(fb, tb, t))
        }
        _ => {
            if t < 1.0 {
                from
            } else {
                to
            }
        }
    }
}

fn lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    ((from as f32) + ((to as f32) - (from as f32)) * t).round() as u8
}

fn color_to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((205, 49, 49)),
        Color::Green => Some((13, 188, 121)),
        Color::Yellow => Some((229, 229, 16)),
        Color::Blue => Some((36, 114, 200)),
        Color::Magenta => Some((188, 63, 188)),
        Color::Cyan => Some((17, 168, 205)),
        Color::Gray => Some((229, 229, 229)),
        Color::DarkGray => Some((102, 102, 102)),
        Color::LightRed => Some((241, 76, 76)),
        Color::LightGreen => Some((35, 209, 139)),
        Color::LightYellow => Some((245, 245, 67)),
        Color::LightBlue => Some((59, 142, 234)),
        Color::LightMagenta => Some((214, 112, 214)),
        Color::LightCyan => Some((41, 184, 219)),
        Color::White => Some((255, 255, 255)),
        Color::Rgb(r, g, b) => Some((r, g, b)),
        _ => None,
    }
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
    food_count: &str,
    next_food_points: &str,
    bonus_multiplier: &str,
) -> usize {
    // {dimensions} │ ■: {food_count} │ v: {next_food_points} │ b: {bonus_multiplier}
    dimensions.chars().count()
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

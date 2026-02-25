use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::config::{HUD_BOTTOM_MARGIN_Y, PLAY_AREA_MARGIN_X, Theme, glyphs};
use crate::game::{GameState, GameStatus};
use crate::platform::Platform;

const HUD_INNER_MARGIN_X: u16 = 1;

/// Supplemental values displayed by the HUD rows.
#[derive(Debug, Clone)]
pub struct HudInfo<'a> {
    pub high_score: u32,
    pub game_over_reference_high_score: u32,
    pub controller_detected: bool,
    pub theme: &'a Theme,
    /// Whether the debug row is enabled (`--debug` flag).
    pub debug: bool,
    /// Pre-formatted debug string; empty when `debug` is false.
    pub debug_line: String,
}

/// Renders the two-line HUD and returns the remaining play area above it.
#[must_use]
pub fn render_hud(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &GameState,
    platform: Platform,
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

    // Score line: Score | Speed | Hi + flags
    let [left, center, right] = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .areas(score_area);

    frame.render_widget(
        Paragraph::new(Line::from(format!("Score: {}", state.score)))
            .alignment(Alignment::Left)
            .style(left_style(info.theme)),
        left,
    );

    frame.render_widget(
        Paragraph::new(Line::from(format!("Speed: {}", state.speed_level)))
            .alignment(Alignment::Center),
        center,
    );

    let right_text = format!(
        "Hi: {} {}{}",
        info.high_score,
        if info.controller_detected {
            "[PAD]"
        } else {
            "[NOPAD]"
        },
        if platform.is_wsl() { " [WSL]" } else { "" }
    );
    frame.render_widget(
        Paragraph::new(Line::from(right_text)).alignment(Alignment::Right),
        right,
    );

    // Status line: game state label
    let dimensions_text = format!("{}x{}", state.bounds().width, state.bounds().height);
    let food_count_text = state.calculated_food_count().to_string();
    let [_, status_center, status_right] = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(50),
        Constraint::Percentage(25),
    ])
    .areas(status_area);

    frame.render_widget(
        Paragraph::new(Line::from(status_label(state, platform)))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        status_center,
    );
    frame.render_widget(
        Paragraph::new(bottom_info_line(
            dimensions_text.as_str(),
            food_count_text.as_str(),
            info.theme.food,
        ))
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::DarkGray)),
        status_right,
    );

    if info.debug {
        frame.render_widget(Paragraph::new("").style(hud_bg), debug_band);
        let debug_width = bottom_info_width(dimensions_text.as_str(), food_count_text.as_str())
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
                info.theme.food,
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

fn status_label(state: &GameState, platform: Platform) -> &'static str {
    match state.status {
        GameStatus::Paused if state.is_start_screen() => {
            if platform.is_wsl() {
                "snake (wsl)"
            } else {
                "snake"
            }
        }
        GameStatus::Playing => {
            if platform.is_wsl() {
                "snake (wsl)"
            } else {
                "snake"
            }
        }
        GameStatus::Paused => "paused",
        GameStatus::GameOver => "game over",
        GameStatus::Victory => "victory",
    }
}

fn left_style(theme: &Theme) -> Style {
    Style::default()
        .fg(theme.ui_text)
        .add_modifier(Modifier::BOLD)
}

fn bottom_info_line<'a>(dimensions: &'a str, food_count: &'a str, food_color: Color) -> Line<'a> {
    Line::from(vec![
        Span::raw(format!("{dimensions}  ")),
        Span::styled(glyphs().solid, Style::default().fg(food_color)),
        Span::raw(format!(" = {food_count}")),
    ])
}

fn bottom_info_width(dimensions: &str, food_count: &str) -> usize {
    dimensions.chars().count() + 2 + 1 + 3 + food_count.chars().count()
}

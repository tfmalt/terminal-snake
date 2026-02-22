use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::config::Theme;
use crate::game::{GameState, GameStatus};
use crate::platform::Platform;

/// Supplemental values displayed by the HUD rows.
#[derive(Debug, Clone)]
pub struct HudInfo {
    pub high_score: u32,
    pub game_over_reference_high_score: u32,
    pub controller_enabled: bool,
    pub theme: &'static Theme,
    /// Whether the debug row is enabled (`--debug` flag).
    pub debug: bool,
    /// Pre-formatted debug string; empty when `debug` is false.
    pub debug_line: String,
}

impl Default for HudInfo {
    fn default() -> Self {
        use crate::config::THEME_CLASSIC;
        Self {
            high_score: 0,
            game_over_reference_high_score: 0,
            controller_enabled: true,
            theme: &THEME_CLASSIC,
            debug: false,
            debug_line: String::new(),
        }
    }
}

/// Renders the two-line HUD and returns the remaining play area above it.
#[must_use]
pub fn render_hud(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &GameState,
    platform: Platform,
    info: &HudInfo,
) -> Rect {
    let debug_height = u16::from(info.debug);
    let [play_area, score_area, status_area, debug_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(debug_height),
    ])
    .areas(area);

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
        if info.controller_enabled {
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
    frame.render_widget(
        Paragraph::new(Line::from(status_label(state, platform)))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        status_area,
    );

    if info.debug {
        frame.render_widget(
            Paragraph::new(Line::from(info.debug_line.as_str()))
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::DarkGray)),
            debug_area,
        );
    }

    play_area
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
        .fg(theme.hud_score)
        .add_modifier(Modifier::BOLD)
}

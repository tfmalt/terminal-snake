use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::game::GameState;
use crate::platform::Platform;

/// Supplemental values displayed by the top HUD row.
#[derive(Debug, Clone, Copy)]
pub struct HudInfo {
    pub high_score: u32,
    pub game_over_reference_high_score: u32,
    pub controller_enabled: bool,
    pub monochrome: bool,
}

impl Default for HudInfo {
    fn default() -> Self {
        Self {
            high_score: 0,
            game_over_reference_high_score: 0,
            controller_enabled: true,
            monochrome: false,
        }
    }
}

/// Renders HUD and returns the remaining play area below it.
#[must_use]
pub fn render_hud(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &GameState,
    platform: Platform,
    info: HudInfo,
) -> Rect {
    let [hud_area, play_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let [left, center, right] = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .areas(hud_area);

    let left_text = format!("Score: {}", state.score);
    frame.render_widget(
        Paragraph::new(Line::from(left_text))
            .alignment(Alignment::Left)
            .style(left_style(info.monochrome)),
        left,
    );

    let center_text = format!("Speed: {}", state.speed_level);
    frame.render_widget(
        Paragraph::new(Line::from(center_text)).alignment(Alignment::Center),
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

    play_area
}

fn left_style(monochrome: bool) -> Style {
    if monochrome {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }
}

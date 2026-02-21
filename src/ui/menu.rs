use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

use crate::game::DeathReason;

/// Draws the start screen as a centered popup.
pub fn render_start_menu(frame: &mut Frame<'_>, area: Rect, high_score: u32) {
    let popup = centered_popup(area, 70, 45);
    frame.render_widget(Clear, popup);

    let [title_row, body_row, footer_row] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .areas(popup);

    frame.render_widget(
        Paragraph::new(Line::from("SNAKE"))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        title_row,
    );

    let body = vec![
        Line::from(format!("High score: {high_score}")),
        Line::from(""),
        Line::from("[Enter]/[Space]/[A] Start"),
        Line::from("[Q]/[Back] Quit"),
    ];
    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Center)
            .block(Block::bordered().title(" start ")),
        body_row,
    );

    frame.render_widget(
        Paragraph::new(Line::from("Use arrows/WASD or D-pad/stick to move"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        footer_row,
    );
}

/// Draws the pause screen as a centered popup.
pub fn render_pause_menu(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered_popup(area, 60, 30);
    frame.render_widget(Clear, popup);

    let lines = vec![
        Line::from("PAUSED"),
        Line::from(""),
        Line::from("[P]/[Start] Resume"),
        Line::from("[Q]/[Back] Quit"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(Block::bordered().title(" pause ")),
        popup,
    );
}

/// Draws the game-over screen as a centered popup.
pub fn render_game_over_menu(
    frame: &mut Frame<'_>,
    area: Rect,
    score: u32,
    high_score: u32,
    death_reason: Option<DeathReason>,
) {
    let popup = centered_popup(area, 70, 40);
    frame.render_widget(Clear, popup);

    let is_new_high = score > high_score;
    let lines = vec![
        Line::from("GAME OVER"),
        Line::from(""),
        Line::from(format!("Score: {score}")),
        Line::from(format!(
            "High score: {}",
            if is_new_high { score } else { high_score }
        )),
        Line::from(match death_reason {
            Some(DeathReason::WallCollision) => "Cause: hit wall",
            Some(DeathReason::SelfCollision) => "Cause: hit yourself",
            None => "",
        }),
        Line::from(if is_new_high { "New high score!" } else { "" }),
        Line::from(""),
        Line::from("[Enter]/[Space]/[A] Play Again"),
        Line::from("[Q]/[Back] Quit"),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(Block::bordered().title(" game over ")),
        popup,
    );
}

fn centered_popup(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let [_, mid, _] = Layout::vertical([
        Constraint::Percentage((100 - height_percent) / 2),
        Constraint::Percentage(height_percent),
        Constraint::Percentage((100 - height_percent) / 2),
    ])
    .areas(area);

    let [_, center, _] = Layout::horizontal([
        Constraint::Percentage((100 - width_percent) / 2),
        Constraint::Percentage(width_percent),
        Constraint::Percentage((100 - width_percent) / 2),
    ])
    .areas(mid);

    center
}

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use std::time::Duration;

use crate::block_font::{FONT_HEIGHT, render_text, text_width};
use crate::config::{
    GLYPH_INDICATOR_DOWN, GLYPH_INDICATOR_UP, MAX_START_SPEED_LEVEL, MIN_START_SPEED_LEVEL, Theme,
    glyphs,
};
use crate::game::DeathReason;
use crate::theme::ThemeItem;

pub struct ThemeSelectView<'a> {
    pub selected_idx: usize,
    pub themes: &'a [ThemeItem],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartTitleMode {
    FullBlock,
    MixedOverlap,
    MixedStacked,
    PlainUpper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameOverTitleMode {
    FullBlock,
    MixedNarrow,
    Plain,
}

/// Draws the start screen as a centered popup.
#[allow(clippy::too_many_arguments)]
pub fn render_start_menu(
    frame: &mut Frame<'_>,
    area: Rect,
    _high_score: u32,
    theme: &Theme,
    play_area_too_small: bool,
    selected_idx: usize,
    settings_open: bool,
    settings_selected_idx: usize,
    start_speed_level: u32,
    speed_adjust_mode: bool,
    checkerboard_enabled: bool,
    game_border_enabled: bool,
    theme_select: Option<ThemeSelectView<'_>>,
) {
    // Breakpoints:
    // 1) Full block-font "terminal   snake".
    // 2) Plain "TERMINAL" + block-font "snake".
    // 3) Plain "TERMINAL" + plain "SNAKE" for very narrow windows.
    let full_title_width = text_width("terminal") + 3 + text_width("snake");
    let snake_block_width = text_width("snake");
    let snake_plain_width = "SNAKE".chars().count();
    let theme_editing = theme_select.is_some();
    // Start layout decision is based on content width at the target popup width.
    let popup_for_measure = centered_popup_with_height(area, 76, area.height.max(1));
    let warning_wrap_width = usize::from(popup_for_measure.width.saturating_sub(2)).max(1);
    let mut body = if settings_open {
        vec![
            menu_option_value_line(
                "Speed",
                start_speed_level.to_string(),
                settings_selected_idx == 0,
                speed_adjust_mode,
                theme,
            ),
            menu_option_value_line(
                "Theme",
                theme.name.to_string(),
                settings_selected_idx == 1,
                theme_editing,
                theme,
            ),
            menu_option_value_line(
                "Grid",
                if checkerboard_enabled { "On" } else { "Off" }.to_string(),
                settings_selected_idx == 2,
                false,
                theme,
            ),
            menu_option_value_line(
                "Border",
                if game_border_enabled { "On" } else { "Off" }.to_string(),
                settings_selected_idx == 3,
                false,
                theme,
            ),
            menu_option_line("Back", settings_selected_idx == 4, theme),
        ]
    } else {
        vec![
            menu_option_line("Start", selected_idx == 0, theme),
            menu_option_line("Settings", selected_idx == 1, theme),
            menu_option_line("Quit", selected_idx == 2, theme),
        ]
    };

    if play_area_too_small {
        let mut warning_lines = Vec::new();
        for line in wrap_text_words(
            "Play area too small (minimum 30x30 cells).",
            warning_wrap_width,
        ) {
            warning_lines.push(
                Line::from(line).style(
                    Style::default()
                        .fg(theme.ui_accent)
                        .add_modifier(Modifier::BOLD),
                ),
            );
        }
        for line in wrap_text_words("Resize terminal to continue.", warning_wrap_width) {
            warning_lines.push(Line::from(line));
        }
        warning_lines.push(Line::from(""));
        warning_lines.append(&mut body);
        body = warning_lines;
    }

    let menu_height = u16::try_from(body.len()).unwrap_or(u16::MAX);

    let snake_top_blank = render_text("snake")
        .first()
        .is_some_and(|r| r.chars().all(|c| c == ' '));
    let title_mode = choose_start_title_mode(usize::from(popup_for_measure.width), snake_top_blank);

    // title_row height:
    // - full/overlap = FONT_HEIGHT + 1 (version)
    // - narrow-no-overlap = 1 (plain) + FONT_HEIGHT + 1 (version)
    // - extra-narrow = 1 (TERMINAL) + 1 (SNAKE) + 1 (version)
    let title_row_height: u16 = match title_mode {
        StartTitleMode::FullBlock | StartTitleMode::MixedOverlap => 5,
        StartTitleMode::MixedStacked => 6,
        StartTitleMode::PlainUpper => 3,
    };

    let logo_to_menu_gap = MENU_MARGIN_ROWS.saturating_sub(1);
    let popup_height = menu_popup_height(title_row_height, menu_height).saturating_add(4);
    let popup = centered_popup_with_height(area, 76, popup_height);
    frame.render_widget(Clear, popup);
    render_menu_panel(frame, popup, theme);

    let [_, title_row, _, body_row, _, hint_row, copyright_row, _] = Layout::vertical([
        Constraint::Length(MENU_MARGIN_ROWS),
        Constraint::Length(title_row_height),
        Constraint::Length(logo_to_menu_gap),
        Constraint::Length(menu_height),
        Constraint::Length(MENU_MARGIN_ROWS),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(popup);

    let popup_width = usize::from(popup.width);

    if title_mode == StartTitleMode::FullBlock {
        let [title_font_row, title_version_row] = Layout::vertical([
            Constraint::Length(FONT_HEIGHT as u16),
            Constraint::Length(1),
        ])
        .areas(title_row);

        let title_lines = start_screen_title_lines(theme);
        frame.render_widget(
            Paragraph::new(title_lines)
                .alignment(Alignment::Center)
                .style(Style::default().bg(theme.ui_bg)),
            title_font_row,
        );

        // Right-align the version with the right edge of the block-font title.
        // The title ("terminal" + 3-space gap + "snake") is center-aligned in the
        // popup; ratatui places its right edge at column (popup_width + title_width) / 2.
        // Padding the version string to that width with Alignment::Left lands it there.
        let title_right_col = (popup_width + full_title_width) / 2;
        let version_text = format!("v{}", env!("CARGO_PKG_VERSION"));
        let pad_to = title_right_col.max(version_text.chars().count());
        let padded_version = format!("{:>width$}", version_text, width = pad_to);
        frame.render_widget(
            Paragraph::new(Line::from(padded_version))
                .alignment(Alignment::Left)
                .style(Style::default().fg(theme.ui_bright).bg(theme.ui_bg)),
            title_version_row,
        );
    } else {
        let snake_width = if matches!(
            title_mode,
            StartTitleMode::MixedOverlap | StartTitleMode::MixedStacked
        ) {
            snake_block_width
        } else {
            snake_plain_width
        };
        let snake_left_col = popup_width.saturating_sub(snake_width) / 2;
        let padded_terminal = format!("{}{}", " ".repeat(snake_left_col), "TERMINAL");
        let terminal_style = Style::default()
            .fg(theme.ui_accent)
            .add_modifier(Modifier::BOLD)
            .bg(theme.ui_bg);

        if title_mode == StartTitleMode::MixedOverlap {
            // SNAKE's top row is blank — render TERMINAL on top of it.
            // Layout: [FONT_HEIGHT(4), 1(version)] = 5 rows total.
            let [title_font_row, title_version_row] = Layout::vertical([
                Constraint::Length(FONT_HEIGHT as u16),
                Constraint::Length(1),
            ])
            .areas(title_row);

            let snake_lines = snake_only_title_lines(theme);
            frame.render_widget(
                Paragraph::new(snake_lines)
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(theme.ui_bg)),
                title_font_row,
            );

            // Overwrite SNAKE row 0 (blank) with TERMINAL.
            let overlay_row = Rect {
                height: 1,
                ..title_font_row
            };
            frame.render_widget(
                Paragraph::new(Line::from(padded_terminal))
                    .alignment(Alignment::Left)
                    .style(terminal_style),
                overlay_row,
            );

            let title_right_col = (popup_width + snake_width) / 2;
            let version_text = format!("v{}", env!("CARGO_PKG_VERSION"));
            let pad_to = title_right_col.max(version_text.chars().count());
            let padded_version = format!("{:>width$}", version_text, width = pad_to);
            frame.render_widget(
                Paragraph::new(Line::from(padded_version))
                    .alignment(Alignment::Left)
                    .style(Style::default().fg(theme.ui_bright).bg(theme.ui_bg)),
                title_version_row,
            );
        } else if title_mode == StartTitleMode::MixedStacked {
            // Can't overlap — TERMINAL gets its own row above SNAKE.
            // Layout: [1(plain), FONT_HEIGHT(4), 1(version)] = 6 rows total.
            let [title_plain_row, title_font_row, title_version_row] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(FONT_HEIGHT as u16),
                Constraint::Length(1),
            ])
            .areas(title_row);

            frame.render_widget(
                Paragraph::new(Line::from(padded_terminal))
                    .alignment(Alignment::Left)
                    .style(terminal_style),
                title_plain_row,
            );

            let snake_lines = snake_only_title_lines(theme);
            frame.render_widget(
                Paragraph::new(snake_lines)
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(theme.ui_bg)),
                title_font_row,
            );

            let title_right_col = (popup_width + snake_width) / 2;
            let version_text = format!("v{}", env!("CARGO_PKG_VERSION"));
            let pad_to = title_right_col.max(version_text.chars().count());
            let padded_version = format!("{:>width$}", version_text, width = pad_to);
            frame.render_widget(
                Paragraph::new(Line::from(padded_version))
                    .alignment(Alignment::Left)
                    .style(Style::default().fg(theme.ui_bright).bg(theme.ui_bg)),
                title_version_row,
            );
        } else {
            // Extra narrow: both words are plain uppercase.
            // Layout: [1(TERMINAL), 1(SNAKE), 1(version)] = 3 rows total.
            let [title_terminal_row, title_snake_row, title_version_row] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(title_row);

            frame.render_widget(
                Paragraph::new(Line::from("TERMINAL"))
                    .alignment(Alignment::Center)
                    .style(terminal_style),
                title_terminal_row,
            );

            frame.render_widget(
                Paragraph::new(Line::from("SNAKE"))
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .fg(theme.ui_text)
                            .add_modifier(Modifier::BOLD)
                            .bg(theme.ui_bg),
                    ),
                title_snake_row,
            );

            let title_right_col = (popup_width + snake_plain_width) / 2;
            let version_text = format!("v{}", env!("CARGO_PKG_VERSION"));
            let pad_to = title_right_col.max(version_text.chars().count());
            let padded_version = format!("{:>width$}", version_text, width = pad_to);
            frame.render_widget(
                Paragraph::new(Line::from(padded_version))
                    .alignment(Alignment::Left)
                    .style(Style::default().fg(theme.ui_bright).bg(theme.ui_bg)),
                title_version_row,
            );
        }
    }

    let menu_width = start_menu_content_width(
        theme,
        start_speed_level,
        checkerboard_enabled,
        game_border_enabled,
        settings_open,
    )
    .saturating_add(2);
    let menu_area = if play_area_too_small {
        body_row
    } else {
        centered_rect_with_max_width(body_row, menu_width)
    };
    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Left)
            .style(menu_body_style(theme)),
        menu_area,
    );

    // Overlay configured up/down indicator glyphs around speed value.
    // Written directly to the buffer after the paragraph so they sit on top of
    // whatever character was already rendered there. Because ratatui redraws the
    // entire frame every tick the source data is never mutated — the overlay is
    // ephemeral and non-destructive.
    if speed_adjust_mode && settings_open {
        // "> Speed:  " = 2 (prefix) + 5 (label) + 3 (":  ") = 10 chars before value.
        let value_x = menu_area.x.saturating_add(10);
        // In settings submenu, Speed is body line index 0.
        let speed_y = menu_area.y;
        let indicator_style = Style::default()
            .fg(theme.ui_accent)
            .bg(theme.ui_bg)
            .add_modifier(Modifier::BOLD);
        let buf = frame.buffer_mut();
        if value_x < menu_area.right() {
            if start_speed_level < MAX_START_SPEED_LEVEL {
                buf.set_string(
                    value_x,
                    speed_y.saturating_sub(1),
                    GLYPH_INDICATOR_UP,
                    indicator_style,
                );
            }
            if start_speed_level > MIN_START_SPEED_LEVEL {
                buf.set_string(
                    value_x,
                    speed_y.saturating_add(1),
                    GLYPH_INDICATOR_DOWN,
                    indicator_style,
                );
            }
        }
    }

    let hint_text = if speed_adjust_mode {
        "↑↓ adjusts speed   Enter/Esc to confirm"
    } else if theme_editing {
        "↑↓ cycles theme   Enter/Esc to confirm"
    } else if settings_open {
        "↑↓ navigate   Enter/→ select   Esc/← back"
    } else {
        "↑↓ navigate   Enter/→ select"
    };
    frame.render_widget(
        Paragraph::new(Line::from(hint_text))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.ui_muted).bg(theme.ui_bg)),
        hint_row,
    );

    frame.render_widget(
        Paragraph::new(Line::from("Copyright (c) 2026 Thomas Malt"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.ui_muted).bg(theme.ui_bg)),
        copyright_row,
    );

    render_menu_bottom_margin(frame, popup, theme);

    if let Some(select_view) = theme_select {
        render_theme_select_list(frame, area, theme, &select_view);
    }
}

/// Draws the pause screen as a centered popup.
pub fn render_pause_menu(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &Theme,
    pause_resize_too_small: bool,
    selected_idx: usize,
    theme_select: Option<ThemeSelectView<'_>>,
) {
    let popup_for_measure = centered_popup_with_height(area, 60, 1);
    let warning_wrap_width = usize::from(popup_for_measure.width.saturating_sub(2)).max(1);
    let mut body = Vec::new();
    if pause_resize_too_small {
        for line in wrap_text_words(
            "Play area too small (minimum 30x30 cells).",
            warning_wrap_width,
        ) {
            body.push(
                Line::from(line).style(
                    Style::default()
                        .fg(theme.ui_accent)
                        .add_modifier(Modifier::BOLD),
                ),
            );
        }

        for line in wrap_text_words("Resize terminal to continue.", warning_wrap_width) {
            body.push(Line::from(line));
        }

        body.push(Line::from(""));
    }
    body.push(menu_option_line("Resume", selected_idx == 0, theme));
    body.push(menu_option_line(
        format!("Theme:  {}", theme.name),
        selected_idx == 1,
        theme,
    ));
    body.push(menu_option_line("Quit", selected_idx == 2, theme));
    let menu_height = u16::try_from(body.len()).unwrap_or(u16::MAX);
    let title_height: u16 = 1;
    let popup_height = menu_popup_height(title_height, menu_height);
    let popup = centered_popup_with_height(area, 60, popup_height);
    frame.render_widget(Clear, popup);
    render_menu_panel(frame, popup, theme);

    let [_, title_row, _, body_row, _] = Layout::vertical([
        Constraint::Length(MENU_MARGIN_ROWS),
        Constraint::Length(title_height),
        Constraint::Length(MENU_MARGIN_ROWS),
        Constraint::Length(menu_height),
        Constraint::Length(MENU_MARGIN_ROWS),
    ])
    .areas(popup);

    frame.render_widget(
        Paragraph::new(Line::from("PAUSED"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.ui_accent).bg(theme.ui_bg)),
        title_row,
    );

    let menu_width = pause_menu_content_width(theme).saturating_add(2);
    let menu_area = if pause_resize_too_small {
        body_row
    } else {
        centered_rect_with_max_width(body_row, menu_width)
    };
    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Left)
            .style(menu_body_style(theme)),
        menu_area,
    );

    render_menu_bottom_margin(frame, popup, theme);

    if let Some(select_view) = theme_select {
        render_theme_select_list(frame, area, theme, &select_view);
    }
}

/// Draws the game-over screen as a centered popup.
#[allow(clippy::too_many_arguments)]
pub fn render_game_over_menu(
    frame: &mut Frame<'_>,
    area: Rect,
    score: u32,
    high_score: u32,
    snake_length: usize,
    coverage_percent: f64,
    death_reason: Option<DeathReason>,
    game_length: Duration,
    theme: &Theme,
    selected_idx: usize,
) {
    let is_new_high = score > high_score;

    let shown_high_score = if is_new_high { score } else { high_score };
    let food_eaten = snake_length.saturating_sub(2);
    let seconds = game_length.as_secs_f64();
    let foods_per_minute = if seconds > 0.0 {
        (food_eaten as f64 / seconds) * 60.0
    } else {
        0.0
    };

    let score_str = score.to_string();
    let length_str = snake_length.to_string();
    let high_score_str = shown_high_score.to_string();
    let coverage_str = format!("{coverage_percent:.2}%");
    let cause_str = match death_reason {
        Some(DeathReason::WallCollision) => "hit wall",
        Some(DeathReason::SelfCollision) => "hit yourself",
        None => "-",
    };
    let game_length_str = format_game_length(game_length);
    let foods_str = format!("{foods_per_minute:.1}");

    let value_col_width = [
        "Value",
        &score_str,
        &high_score_str,
        cause_str,
        &game_length_str,
        &foods_str,
        &length_str,
        &coverage_str,
    ]
    .iter()
    .map(|s| s.len())
    .max()
    .unwrap_or(5);

    let mut body = vec![
        table_header_row("Metric", "Value", value_col_width, theme),
        table_row("Score", &score_str, value_col_width, theme),
        table_row("High score", &high_score_str, value_col_width, theme),
        table_row("Cause", cause_str, value_col_width, theme),
        table_row("Game length", &game_length_str, value_col_width, theme),
        table_row("Food/min", &foods_str, value_col_width, theme),
        table_row("Length", &length_str, value_col_width, theme),
        table_row("Coverage", &coverage_str, value_col_width, theme),
        Line::from(""),
    ];

    if is_new_high {
        body.push(Line::from("New high score!"));
        body.push(Line::from(""));
    }

    body.push(menu_option_line("Play Again", selected_idx == 0, theme));
    body.push(menu_option_line("Quit", selected_idx == 1, theme));

    let menu_height = u16::try_from(body.len()).unwrap_or(u16::MAX);
    let popup_for_measure = centered_popup_with_height(area, 70, area.height.max(1));
    let title_mode = choose_game_over_title_mode(
        usize::from(popup_for_measure.width),
        area.height,
        menu_height,
    );
    let title_height = match title_mode {
        GameOverTitleMode::FullBlock => FONT_HEIGHT as u16,
        GameOverTitleMode::MixedNarrow => FONT_HEIGHT as u16 + 2,
        GameOverTitleMode::Plain => 1,
    };
    let popup_height = menu_popup_height(title_height, menu_height).saturating_add(2);
    let popup = centered_popup_with_height(area, 70, popup_height);
    frame.render_widget(Clear, popup);
    render_menu_panel(frame, popup, theme);

    let [_, title_row, _, body_row, _, footer_hint_row] = Layout::vertical([
        Constraint::Length(MENU_MARGIN_ROWS),
        Constraint::Length(title_height),
        Constraint::Length(MENU_MARGIN_ROWS),
        Constraint::Length(menu_height),
        Constraint::Length(MENU_MARGIN_ROWS),
        Constraint::Length(1),
    ])
    .areas(popup);

    match title_mode {
        GameOverTitleMode::FullBlock => {
            frame.render_widget(
                Paragraph::new(game_over_block_title_lines(theme))
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(theme.ui_bg)),
                title_row,
            );
        }
        GameOverTitleMode::MixedNarrow => {
            let popup_width = usize::from(popup.width);
            let over_width = text_width("over");
            let over_left_col = popup_width.saturating_sub(over_width) / 2;
            let padded_game = format!("{}GAME", " ".repeat(over_left_col));
            let [_, game_row, over_row] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(FONT_HEIGHT as u16),
            ])
            .areas(title_row);

            frame.render_widget(
                Paragraph::new(Line::from(padded_game))
                    .alignment(Alignment::Left)
                    .style(
                        Style::default()
                            .fg(theme.ui_accent)
                            .add_modifier(Modifier::BOLD)
                            .bg(theme.ui_bg),
                    ),
                game_row,
            );

            frame.render_widget(
                Paragraph::new(over_only_title_lines(theme))
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(theme.ui_bg)),
                over_row,
            );
        }
        GameOverTitleMode::Plain => {
            frame.render_widget(
                Paragraph::new(Line::from("Game Over"))
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .fg(theme.ui_text)
                            .add_modifier(Modifier::BOLD)
                            .bg(theme.ui_bg),
                    ),
                title_row,
            );
        }
    }

    // label cell (16) + inner separator (1) + value cell (2 + value_col_width)
    // = 19 + value_col_width.
    let table_width = u16::try_from(19 + value_col_width).unwrap_or(u16::MAX);
    let centered_body = centered_rect_with_max_width(body_row, table_width);
    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Left)
            .style(menu_body_style(theme)),
        centered_body,
    );

    let table_rows: u16 = 8;
    let table_area = Rect {
        x: centered_body.x,
        y: centered_body.y,
        width: centered_body.width,
        height: table_rows.min(centered_body.height),
    };
    render_outer_table_border(frame, table_area, theme);

    frame.render_widget(
        Paragraph::new(Line::from("Use arrows/WASD to move"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.ui_muted).bg(theme.ui_bg)),
        footer_hint_row,
    );

    render_menu_bottom_margin(frame, popup, theme);
}

fn choose_game_over_title_mode(
    available_width: usize,
    available_height: u16,
    menu_height: u16,
) -> GameOverTitleMode {
    let full_title_width = text_width("game") + 3 + text_width("over");
    let over_only_width = text_width("over");
    let full_block_height = FONT_HEIGHT as u16;

    // If we're short on vertical space, prefer a one-line title so the table/menu
    // rows remain visible.
    if menu_popup_height(full_block_height, menu_height).saturating_add(2) > available_height {
        return GameOverTitleMode::Plain;
    }

    if full_title_width + 4 <= available_width {
        GameOverTitleMode::FullBlock
    } else if over_only_width + 4 <= available_width {
        GameOverTitleMode::MixedNarrow
    } else {
        GameOverTitleMode::Plain
    }
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

fn centered_popup_with_height(area: Rect, width_percent: u16, desired_height: u16) -> Rect {
    let [_, center, _] = Layout::horizontal([
        Constraint::Percentage((100 - width_percent) / 2),
        Constraint::Percentage(width_percent),
        Constraint::Percentage((100 - width_percent) / 2),
    ])
    .areas(area);

    let popup_height = desired_height.min(area.height).max(1);
    let y = area.y + area.height.saturating_sub(popup_height) / 2;

    Rect {
        x: center.x,
        y,
        width: center.width,
        height: popup_height,
    }
}

fn render_theme_select_list(
    frame: &mut Frame<'_>,
    area: Rect,
    active_theme: &Theme,
    select_view: &ThemeSelectView<'_>,
) {
    let desired_list_height = u16::try_from(select_view.themes.len().max(1)).unwrap_or(u16::MAX);
    let desired_popup_height = desired_list_height;
    let base_popup = centered_popup(area, 52, 40);
    let desired_popup_width = theme_list_width(select_view.themes);
    let popup = left_anchored_popup_with_size(
        area,
        base_popup.x,
        desired_popup_width,
        desired_popup_height,
    );
    frame.render_widget(Clear, popup);
    render_menu_panel(frame, popup, active_theme);
    let inner = popup;

    let list_height = desired_list_height.min(inner.height.max(1));

    let [list_row] = Layout::vertical([Constraint::Length(list_height)]).areas(inner);

    let items = visible_theme_lines(
        select_view.themes,
        select_view.selected_idx,
        usize::from(list_height),
        active_theme,
    );
    frame.render_widget(
        Paragraph::new(items)
            .alignment(Alignment::Left)
            .style(theme_select_list_style(active_theme)),
        list_row,
    );

    render_menu_bottom_margin(frame, popup, active_theme);

    if let Some(preview_area) = right_preview_area(area, popup) {
        render_theme_preview_palette(frame, preview_area, active_theme);
    }
}

fn right_preview_area(container: Rect, anchor: Rect) -> Option<Rect> {
    let x = anchor.right().saturating_add(1);
    if x >= container.right() {
        return None;
    }

    let available_width = container.right().saturating_sub(x);
    if available_width < 18 {
        return None;
    }

    Some(Rect {
        x,
        y: anchor.y,
        width: available_width.min(30),
        height: anchor.height,
    })
}

fn render_theme_preview_palette(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    frame.render_widget(Clear, area);
    render_menu_panel(frame, area, theme);

    let [_, content] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);

    frame.render_widget(
        Paragraph::new(theme_preview_lines(theme))
            .alignment(Alignment::Left)
            .style(menu_body_style(theme)),
        content,
    );

    render_menu_bottom_margin(frame, area, theme);
}

fn theme_preview_lines(theme: &Theme) -> Vec<Line<'static>> {
    vec![
        Line::from("Preview"),
        Line::from(""),
        swatch_line("Head", theme.snake_head, "snake head"),
        swatch_line("Body", theme.snake_body, "snake body"),
        swatch_line("Tail", theme.snake_tail, "snake tail"),
        swatch_line("Food", theme.food, "food"),
        swatch_line("Term bg", theme.terminal_bg, "terminal_bg"),
        swatch_line("Field", theme.field_bg, "field_bg"),
        swatch_line("UI bg", theme.ui_bg, "ui_bg"),
        swatch_line("UI text", theme.ui_text, "ui_text"),
    ]
}

fn swatch_line(label: &str, color: ratatui::style::Color, usage: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw(format!("{label:<7} ")),
        Span::styled("   ", Style::default().bg(color)),
        Span::raw(format!(" {usage}")),
    ])
}

fn left_anchored_popup_with_size(area: Rect, x: u16, width: u16, height: u16) -> Rect {
    let left = x.clamp(area.x, area.right().saturating_sub(1));
    let max_width = area.right().saturating_sub(left);
    let popup_width = width.min(max_width).max(1);
    let popup_height = height.min(area.height).max(1);

    let y = area.y + area.height.saturating_sub(popup_height) / 2;

    Rect {
        x: left,
        y,
        width: popup_width,
        height: popup_height,
    }
}

fn visible_theme_lines(
    themes: &[ThemeItem],
    selected_idx: usize,
    count: usize,
    active_theme: &Theme,
) -> Vec<Line<'static>> {
    let longest_name = longest_theme_name_width(themes);

    if themes.is_empty() {
        return vec![Line::from(format!(
            " {:<width$} ",
            "No themes available",
            width = longest_name,
        ))];
    }

    let show_count = count.min(themes.len());
    let center = show_count / 2;
    let start = (selected_idx + themes.len() - center) % themes.len();

    let mut lines = Vec::with_capacity(show_count);
    for offset in 0..show_count {
        let idx = (start + offset) % themes.len();
        let line = format!(" {:<width$} ", themes[idx].theme.name, width = longest_name,);
        if idx == selected_idx {
            lines.push(
                Line::from(line).style(
                    Style::default()
                        .fg(active_theme.ui_bg)
                        .bg(active_theme.ui_text)
                        .add_modifier(Modifier::BOLD),
                ),
            );
        } else {
            lines.push(Line::from(line));
        }
    }
    lines
}

fn menu_option_line<T: Into<String>>(label: T, selected: bool, theme: &Theme) -> Line<'static> {
    let prefix = if selected { "> " } else { "  " };
    let line = format!("{prefix}{}", label.into());
    if selected {
        Line::from(line).style(
            Style::default()
                .fg(theme.ui_accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Line::from(line)
    }
}

/// Renders a menu row that has a label and an editable value (e.g. "Speed: 5").
///
/// When `editing` is true the value is rendered with inverted colours to show
/// it is the thing being actively changed, giving a clear visual distinction
/// between "cursor on this row" and "editing this value".
fn menu_option_value_line(
    label: &str,
    value: String,
    selected: bool,
    editing: bool,
    theme: &Theme,
) -> Line<'static> {
    const VALUE_LABEL_WIDTH: usize = 6;
    let prefix = if selected { "> " } else { "  " };
    let padded_label = format!("{label:<VALUE_LABEL_WIDTH$}");
    if editing {
        // Highlight only the value to signal "you are editing this".
        let label_style = Style::default()
            .fg(theme.ui_accent)
            .add_modifier(Modifier::BOLD);
        let value_style = Style::default()
            .fg(theme.ui_bg)
            .bg(theme.ui_accent)
            .add_modifier(Modifier::BOLD);
        Line::from(vec![
            Span::styled(format!("{prefix}{padded_label}:  "), label_style),
            Span::styled(value, value_style),
        ])
    } else if selected {
        Line::from(format!("{prefix}{padded_label}:  {value}")).style(
            Style::default()
                .fg(theme.ui_accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Line::from(format!("{prefix}{padded_label}:  {value}"))
    }
}

fn start_screen_title_lines(theme: &Theme) -> Vec<Line<'static>> {
    let terminal_rows = render_text("terminal");
    let snake_rows = render_text("snake");

    terminal_rows
        .into_iter()
        .zip(snake_rows)
        .map(|(terminal_row, snake_row)| {
            Line::from(vec![
                Span::styled(
                    terminal_row,
                    Style::default()
                        .fg(theme.ui_accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    snake_row,
                    Style::default()
                        .fg(theme.ui_text)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect()
}

/// Renders only "snake" in block font — used when the terminal is too narrow for
/// the full "terminal snake" side-by-side title.
fn snake_only_title_lines(theme: &Theme) -> Vec<Line<'static>> {
    render_text("snake")
        .into_iter()
        .map(|row| {
            Line::from(Span::styled(
                row,
                Style::default()
                    .fg(theme.ui_text)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect()
}

fn game_over_block_title_lines(theme: &Theme) -> Vec<Line<'static>> {
    render_text("game over")
        .into_iter()
        .map(|row| {
            Line::from(Span::styled(
                row,
                Style::default()
                    .fg(theme.ui_text)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect()
}

fn over_only_title_lines(theme: &Theme) -> Vec<Line<'static>> {
    render_text("over")
        .into_iter()
        .map(|row| {
            Line::from(Span::styled(
                row,
                Style::default()
                    .fg(theme.ui_text)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect()
}

fn start_menu_content_width(
    theme: &Theme,
    start_speed_level: u32,
    checkerboard_enabled: bool,
    game_border_enabled: bool,
    settings_open: bool,
) -> u16 {
    const VALUE_LABEL_WIDTH: usize = 6;
    let labels = if settings_open {
        [
            format!("{:<VALUE_LABEL_WIDTH$}:  {start_speed_level}", "Speed"),
            format!("{:<VALUE_LABEL_WIDTH$}:  {}", "Theme", theme.name),
            format!(
                "{:<VALUE_LABEL_WIDTH$}:  {}",
                "Grid",
                if checkerboard_enabled { "On" } else { "Off" }
            ),
            format!(
                "{:<VALUE_LABEL_WIDTH$}:  {}",
                "Border",
                if game_border_enabled { "On" } else { "Off" }
            ),
            "Back".to_string(),
        ]
    } else {
        [
            "Start".to_string(),
            "Settings".to_string(),
            "Quit".to_string(),
            String::new(),
            String::new(),
        ]
    };

    let widest = labels
        .iter()
        .map(|label| label.chars().count())
        .max()
        .unwrap_or(0)
        .saturating_add(2);

    widest.min(u16::MAX as usize) as u16
}

fn pause_menu_content_width(theme: &Theme) -> u16 {
    let labels = [
        "Resume".to_string(),
        format!("Theme:  {}", theme.name),
        "Quit".to_string(),
    ];

    let widest = labels
        .iter()
        .map(|label| label.chars().count())
        .max()
        .unwrap_or(0)
        .saturating_add(2);

    widest.min(u16::MAX as usize) as u16
}

fn choose_start_title_mode(available_width: usize, snake_top_blank: bool) -> StartTitleMode {
    let full_title_width = text_width("terminal") + 3 + text_width("snake");
    let snake_block_width = text_width("snake");

    if full_title_width + 4 <= available_width {
        StartTitleMode::FullBlock
    } else if snake_block_width + 4 <= available_width {
        if snake_top_blank {
            StartTitleMode::MixedOverlap
        } else {
            StartTitleMode::MixedStacked
        }
    } else {
        StartTitleMode::PlainUpper
    }
}

fn menu_popup_height(title_rows: u16, menu_rows: u16) -> u16 {
    MENU_MARGIN_ROWS
        .saturating_mul(3)
        .saturating_add(title_rows)
        .saturating_add(menu_rows)
}

const MENU_MARGIN_ROWS: u16 = 2;

fn centered_rect_with_max_width(area: Rect, max_width: u16) -> Rect {
    if area.width <= max_width {
        return area;
    }

    let width = max_width.max(1);
    let x = area.x + (area.width - width) / 2;
    Rect {
        x,
        y: area.y,
        width,
        height: area.height,
    }
}

fn table_header_row(
    label: &str,
    value: &str,
    value_col_width: usize,
    theme: &Theme,
) -> Line<'static> {
    let inner_separator = glyphs().table_separator;
    let cell_style = Style::default()
        .fg(theme.ui_text)
        .add_modifier(Modifier::REVERSED);
    Line::from(vec![
        Span::styled(format!(" {label:<14} "), cell_style),
        Span::styled(inner_separator, cell_style),
        Span::styled(format!(" {value:<value_col_width$} "), cell_style),
    ])
}

fn table_row(
    label: &str,
    value: impl AsRef<str>,
    value_col_width: usize,
    theme: &Theme,
) -> Line<'static> {
    let inner_separator = glyphs().table_separator;
    Line::from(vec![
        Span::styled(
            format!(" {label:<14} "),
            Style::default().fg(theme.ui_bright),
        ),
        Span::styled(inner_separator, Style::default().fg(theme.ui_bright)),
        Span::styled(
            format!(" {:<value_col_width$} ", value.as_ref()),
            Style::default().fg(theme.ui_text),
        ),
    ])
}

fn render_outer_table_border(frame: &mut Frame<'_>, table_area: Rect, theme: &Theme) {
    if table_area.width == 0 || table_area.height == 0 {
        return;
    }

    let screen = frame.area();
    let style = Style::default().fg(theme.ui_bright).bg(theme.ui_bg);
    let buffer = frame.buffer_mut();

    if table_area.y > screen.y {
        let top_y = table_area.y - 1;
        for x in table_area.x..table_area.right() {
            buffer.set_string(x, top_y, "▁", style);
        }
    }

    if table_area.bottom() < screen.bottom() {
        let bottom_y = table_area.bottom();
        for x in table_area.x..table_area.right() {
            buffer.set_string(x, bottom_y, "▔", style);
        }
    }

    if table_area.x > screen.x {
        let left_x = table_area.x - 1;
        for y in table_area.y..table_area.bottom() {
            buffer.set_string(left_x, y, "▕", style);
        }
    }

    if table_area.right() < screen.right() {
        let right_x = table_area.right();
        for y in table_area.y..table_area.bottom() {
            buffer.set_string(right_x, y, "▏", style);
        }
    }
}

fn format_game_length(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn wrap_text_words(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if word_len > max_width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }

            let mut chunk = String::new();
            for ch in word.chars() {
                if chunk.chars().count() >= max_width {
                    lines.push(std::mem::take(&mut chunk));
                }
                chunk.push(ch);
            }
            if !chunk.is_empty() {
                lines.push(chunk);
            }
            continue;
        }

        let needs_space = usize::from(!current.is_empty());
        if current.chars().count() + needs_space + word_len > max_width {
            lines.push(std::mem::take(&mut current));
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn render_menu_panel(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.ui_bg).fg(theme.ui_text)),
        area,
    );

    if area.height < 2 {
        return;
    }

    let top_y = area.y;
    let bottom_y = area.bottom().saturating_sub(1);
    let margin_style = Style::default().fg(theme.ui_bg).bg(theme.field_bg);
    let palette = glyphs();
    let buffer = frame.buffer_mut();

    for x in area.x..area.right() {
        buffer.set_string(x, top_y, palette.half_lower, margin_style);
        buffer.set_string(x, bottom_y, palette.half_upper, margin_style);
    }
}

fn render_menu_bottom_margin(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    if area.height < 1 {
        return;
    }

    let bottom_y = area.bottom().saturating_sub(1);
    let margin_style = Style::default().fg(theme.ui_bg).bg(theme.field_bg);
    let half_upper = glyphs().half_upper;
    let buffer = frame.buffer_mut();
    for x in area.x..area.right() {
        buffer.set_string(x, bottom_y, half_upper, margin_style);
    }
}

fn menu_body_style(theme: &Theme) -> Style {
    Style::default().fg(theme.ui_text).bg(theme.ui_bg)
}

fn theme_select_list_style(theme: &Theme) -> Style {
    Style::default().fg(theme.ui_text).bg(theme.field_bg)
}

fn theme_list_width(themes: &[ThemeItem]) -> u16 {
    let longest_name = longest_theme_name_width(themes);

    let width_with_margin = longest_name.saturating_add(2);
    width_with_margin.min(u16::MAX as usize) as u16
}

fn longest_theme_name_width(themes: &[ThemeItem]) -> usize {
    if themes.is_empty() {
        "No themes available".chars().count()
    } else {
        themes
            .iter()
            .map(|theme| theme.theme.name.chars().count())
            .max()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GameOverTitleMode, StartTitleMode, choose_game_over_title_mode, choose_start_title_mode,
    };
    use crate::block_font::text_width;

    #[test]
    fn title_mode_uses_full_block_when_wide_enough() {
        let available_width = text_width("terminal") + 3 + text_width("snake") + 4;
        let mode = choose_start_title_mode(available_width, true);
        assert_eq!(mode, StartTitleMode::FullBlock);
    }

    #[test]
    fn title_mode_uses_overlap_when_only_snake_block_fits_and_top_is_blank() {
        let available_width = text_width("snake") + 4;
        let mode = choose_start_title_mode(available_width, true);
        assert_eq!(mode, StartTitleMode::MixedOverlap);
    }

    #[test]
    fn title_mode_uses_stacked_when_only_snake_block_fits_and_top_is_not_blank() {
        let available_width = text_width("snake") + 4;
        let mode = choose_start_title_mode(available_width, false);
        assert_eq!(mode, StartTitleMode::MixedStacked);
    }

    #[test]
    fn title_mode_uses_plain_upper_when_snake_block_does_not_fit() {
        let available_width = text_width("snake") + 3;
        let mode = choose_start_title_mode(available_width, true);
        assert_eq!(mode, StartTitleMode::PlainUpper);
    }

    #[test]
    fn game_over_title_mode_uses_full_block_when_space_allows() {
        let available_width = text_width("game") + 3 + text_width("over") + 4;
        let mode = choose_game_over_title_mode(available_width, 60, 10);
        assert_eq!(mode, GameOverTitleMode::FullBlock);
    }

    #[test]
    fn game_over_title_mode_uses_mixed_when_width_is_tight() {
        let available_width = text_width("over") + 4;
        let mode = choose_game_over_title_mode(available_width, 60, 10);
        assert_eq!(mode, GameOverTitleMode::MixedNarrow);
    }

    #[test]
    fn game_over_title_mode_uses_plain_when_height_is_too_small() {
        let available_width = text_width("game") + 3 + text_width("over") + 4;
        let mode = choose_game_over_title_mode(available_width, 20, 10);
        assert_eq!(mode, GameOverTitleMode::Plain);
    }
}

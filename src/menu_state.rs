use crate::config::{MAX_START_SPEED_LEVEL, MIN_START_SPEED_LEVEL};
use crate::input::{Direction, GameInput};
use crate::theme::ThemeCatalog;

/// Which menu screen is currently active as a sub-mode overlay.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MenuScreen {
    /// Top-level start menu (Start / Settings / Quit).
    StartMain,
    /// Settings sub-menu within the start screen.
    StartSettings,
    /// Speed adjustment sub-mode within settings.
    SpeedAdjust,
    /// Theme selection from the start menu's settings.
    StartThemeSelect,
    /// Pause menu (Resume / Theme / Quit).
    PauseMain,
    /// Theme selection from the pause menu.
    PauseThemeSelect,
    /// Game-over menu (Play Again / Quit).
    GameOver,
}

/// Result of processing a menu input event.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MenuAction {
    /// Input was consumed by the menu; no further action needed.
    Consumed,
    /// Start a new game.
    StartGame,
    /// Resume from pause.
    Resume,
    /// Resume a previously saved active session from start menu.
    ResumeSession,
    /// Restart after game over.
    Restart,
    /// Quit the application.
    Quit,
}

/// Menu item descriptor: a label and its associated action.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: &'static str,
    pub action: MenuItemAction,
}

/// What a menu item does when activated.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MenuItemAction {
    ResumeSession,
    Start,
    OpenSettings,
    Quit,
    SetSpeed,
    SetTheme,
    ToggleGrid,
    ToggleBorder,
    Back,
    Resume,
    Restart,
}

/// Start menu items, defined as data.
pub const START_MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        label: "Resume",
        action: MenuItemAction::ResumeSession,
    },
    MenuItem {
        label: "Start",
        action: MenuItemAction::Start,
    },
    MenuItem {
        label: "Settings",
        action: MenuItemAction::OpenSettings,
    },
    MenuItem {
        label: "Quit",
        action: MenuItemAction::Quit,
    },
];

/// Settings sub-menu items, defined as data.
pub const START_SETTINGS_ITEMS: &[MenuItem] = &[
    MenuItem {
        label: "Speed",
        action: MenuItemAction::SetSpeed,
    },
    MenuItem {
        label: "Theme",
        action: MenuItemAction::SetTheme,
    },
    MenuItem {
        label: "Grid",
        action: MenuItemAction::ToggleGrid,
    },
    MenuItem {
        label: "Border",
        action: MenuItemAction::ToggleBorder,
    },
    MenuItem {
        label: "Back",
        action: MenuItemAction::Back,
    },
];

/// Pause menu items, defined as data.
pub const PAUSE_MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        label: "Resume",
        action: MenuItemAction::Resume,
    },
    MenuItem {
        label: "Theme",
        action: MenuItemAction::SetTheme,
    },
    MenuItem {
        label: "Quit",
        action: MenuItemAction::Quit,
    },
];

/// Game-over menu items, defined as data.
pub const GAME_OVER_MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        label: "Play Again",
        action: MenuItemAction::Restart,
    },
    MenuItem {
        label: "Quit",
        action: MenuItemAction::Quit,
    },
];

/// Encapsulates all menu navigation state, replacing loose booleans and indices.
#[derive(Debug)]
pub struct MenuState {
    pub screen: MenuScreen,
    pub start_selected_idx: usize,
    pub start_settings_selected_idx: usize,
    pub pause_selected_idx: usize,
    pub game_over_selected_idx: usize,
    pub start_speed_level: u32,
    pub checkerboard_enabled: bool,
    pub game_border_enabled: bool,
    pub theme_dirty: bool,
    pub has_active_session: bool,
}

impl MenuState {
    /// Creates a new menu state starting at the start screen.
    #[must_use]
    pub fn new(start_speed_level: u32, checkerboard_enabled: bool) -> Self {
        Self {
            screen: MenuScreen::StartMain,
            start_selected_idx: 0,
            start_settings_selected_idx: 0,
            pause_selected_idx: 0,
            game_over_selected_idx: 0,
            start_speed_level,
            checkerboard_enabled,
            game_border_enabled: true,
            theme_dirty: false,
            has_active_session: false,
        }
    }

    /// Sets whether start menu should show a Resume entry.
    pub fn set_has_active_session(&mut self, has_active_session: bool) {
        self.has_active_session = has_active_session;
        if !self.has_active_session && self.start_selected_idx > 2 {
            self.start_selected_idx = 2;
        }
    }

    /// Returns the number of visible items in the start menu.
    #[must_use]
    pub fn start_menu_len(&self) -> usize {
        if self.has_active_session {
            START_MENU_ITEMS.len()
        } else {
            START_MENU_ITEMS.len() - 1
        }
    }

    /// Returns the visible start menu label at `index`.
    #[must_use]
    pub fn start_menu_label(&self, index: usize) -> &'static str {
        self.start_menu_item_at(index).label
    }

    /// Whether the settings sub-menu is currently open.
    #[must_use]
    pub fn is_settings_open(&self) -> bool {
        matches!(
            self.screen,
            MenuScreen::StartSettings | MenuScreen::SpeedAdjust | MenuScreen::StartThemeSelect
        )
    }

    /// Whether any speed-adjust mode is active.
    #[must_use]
    pub fn is_speed_adjust(&self) -> bool {
        self.screen == MenuScreen::SpeedAdjust
    }

    /// Whether any theme selection overlay is active.
    #[must_use]
    pub fn is_theme_selecting(&self) -> bool {
        matches!(
            self.screen,
            MenuScreen::StartThemeSelect | MenuScreen::PauseThemeSelect
        )
    }

    /// Handles input while on the start screen. Returns the resulting action.
    pub fn handle_start_input(
        &mut self,
        input: GameInput,
        themes: &mut ThemeCatalog,
        play_area_too_small: bool,
    ) -> MenuAction {
        match self.screen {
            MenuScreen::StartThemeSelect => {
                self.handle_theme_select_input(input, themes, MenuScreen::StartSettings)
            }
            MenuScreen::SpeedAdjust => self.handle_speed_adjust_input(input),
            MenuScreen::StartSettings => self.handle_start_settings_input(input),
            MenuScreen::StartMain => self.handle_start_main_input(input, play_area_too_small),
            _ => MenuAction::Consumed,
        }
    }

    /// Handles input while paused. Returns the resulting action.
    pub fn handle_pause_input(
        &mut self,
        input: GameInput,
        themes: &mut ThemeCatalog,
        play_area_too_small: bool,
    ) -> MenuAction {
        if self.screen == MenuScreen::PauseThemeSelect {
            return self.handle_theme_select_input(input, themes, MenuScreen::PauseMain);
        }

        match input {
            GameInput::Direction(Direction::Up) => {
                self.pause_selected_idx =
                    wrap_prev(self.pause_selected_idx, PAUSE_MENU_ITEMS.len());
            }
            GameInput::Direction(Direction::Down) => {
                self.pause_selected_idx =
                    wrap_next(self.pause_selected_idx, PAUSE_MENU_ITEMS.len());
            }
            GameInput::Confirm | GameInput::Direction(Direction::Right) => {
                let action = PAUSE_MENU_ITEMS[self.pause_selected_idx].action;
                match action {
                    MenuItemAction::Resume if !play_area_too_small => {
                        return MenuAction::Resume;
                    }
                    MenuItemAction::SetTheme => {
                        self.screen = MenuScreen::PauseThemeSelect;
                    }
                    MenuItemAction::Quit => return MenuAction::Quit,
                    _ => {}
                }
            }
            GameInput::Pause | GameInput::Direction(Direction::Left) if !play_area_too_small => {
                return MenuAction::Resume;
            }
            _ => {}
        }

        MenuAction::Consumed
    }

    /// Handles input on the game-over screen. Returns the resulting action.
    pub fn handle_game_over_input(&mut self, input: GameInput) -> MenuAction {
        match input {
            GameInput::Direction(Direction::Up) => {
                self.game_over_selected_idx =
                    wrap_prev(self.game_over_selected_idx, GAME_OVER_MENU_ITEMS.len());
            }
            GameInput::Direction(Direction::Down) => {
                self.game_over_selected_idx =
                    wrap_next(self.game_over_selected_idx, GAME_OVER_MENU_ITEMS.len());
            }
            GameInput::Confirm | GameInput::Direction(Direction::Right) => {
                let action = GAME_OVER_MENU_ITEMS[self.game_over_selected_idx].action;
                match action {
                    MenuItemAction::Restart => return MenuAction::Restart,
                    MenuItemAction::Quit => return MenuAction::Quit,
                    _ => {}
                }
            }
            _ => {}
        }

        MenuAction::Consumed
    }

    /// Resets menu state when entering pause from gameplay.
    pub fn enter_pause(&mut self) {
        self.screen = MenuScreen::PauseMain;
        self.pause_selected_idx = 0;
    }

    /// Resets menu state when entering game-over.
    pub fn enter_game_over(&mut self) {
        self.screen = MenuScreen::GameOver;
        self.game_over_selected_idx = 0;
    }

    /// Resets to start screen state.
    pub fn enter_start(&mut self) {
        self.screen = MenuScreen::StartMain;
    }

    /// Resets sub-mode state when entering gameplay.
    pub fn enter_playing(&mut self) {
        // Clear any overlay modes when game starts.
        if matches!(
            self.screen,
            MenuScreen::PauseThemeSelect | MenuScreen::StartThemeSelect
        ) {
            // Theme selection dismissed.
        }
    }

    fn handle_theme_select_input(
        &mut self,
        input: GameInput,
        themes: &mut ThemeCatalog,
        return_screen: MenuScreen,
    ) -> MenuAction {
        match input {
            GameInput::Direction(Direction::Up) => {
                themes.select_previous();
                self.theme_dirty = true;
            }
            GameInput::Direction(Direction::Down) | GameInput::CycleTheme => {
                themes.select_next();
                self.theme_dirty = true;
            }
            GameInput::Confirm
            | GameInput::Direction(Direction::Right)
            | GameInput::Pause
            | GameInput::Direction(Direction::Left) => {
                self.screen = return_screen;
            }
            _ => {}
        }
        MenuAction::Consumed
    }

    fn handle_speed_adjust_input(&mut self, input: GameInput) -> MenuAction {
        match input {
            GameInput::Direction(Direction::Up) => {
                self.start_speed_level = self
                    .start_speed_level
                    .saturating_add(1)
                    .min(MAX_START_SPEED_LEVEL);
            }
            GameInput::Direction(Direction::Down) => {
                self.start_speed_level = self
                    .start_speed_level
                    .saturating_sub(1)
                    .max(MIN_START_SPEED_LEVEL);
            }
            GameInput::Confirm
            | GameInput::Direction(Direction::Right)
            | GameInput::Direction(Direction::Left)
            | GameInput::Pause => {
                self.screen = MenuScreen::StartSettings;
            }
            _ => {}
        }
        MenuAction::Consumed
    }

    fn handle_start_settings_input(&mut self, input: GameInput) -> MenuAction {
        match input {
            GameInput::Direction(Direction::Up) => {
                self.start_settings_selected_idx =
                    wrap_prev(self.start_settings_selected_idx, START_SETTINGS_ITEMS.len());
            }
            GameInput::Direction(Direction::Down) => {
                self.start_settings_selected_idx =
                    wrap_next(self.start_settings_selected_idx, START_SETTINGS_ITEMS.len());
            }
            GameInput::Confirm | GameInput::Direction(Direction::Right) => {
                let action = START_SETTINGS_ITEMS[self.start_settings_selected_idx].action;
                match action {
                    MenuItemAction::SetSpeed => {
                        self.screen = MenuScreen::SpeedAdjust;
                    }
                    MenuItemAction::SetTheme => {
                        self.screen = MenuScreen::StartThemeSelect;
                    }
                    MenuItemAction::ToggleGrid => {
                        self.checkerboard_enabled = !self.checkerboard_enabled;
                    }
                    MenuItemAction::ToggleBorder => {
                        self.game_border_enabled = !self.game_border_enabled;
                    }
                    MenuItemAction::Back => {
                        self.screen = MenuScreen::StartMain;
                    }
                    _ => {}
                }
            }
            GameInput::Direction(Direction::Left) | GameInput::Pause => {
                self.screen = MenuScreen::StartMain;
            }
            _ => {}
        }
        MenuAction::Consumed
    }

    fn handle_start_main_input(
        &mut self,
        input: GameInput,
        play_area_too_small: bool,
    ) -> MenuAction {
        match input {
            GameInput::Direction(Direction::Up) => {
                self.start_selected_idx = wrap_prev(self.start_selected_idx, self.start_menu_len());
            }
            GameInput::Direction(Direction::Down) => {
                self.start_selected_idx = wrap_next(self.start_selected_idx, self.start_menu_len());
            }
            GameInput::Confirm | GameInput::Direction(Direction::Right) => {
                let action = self.start_menu_item_at(self.start_selected_idx).action;
                match action {
                    MenuItemAction::ResumeSession if !play_area_too_small => {
                        return MenuAction::ResumeSession;
                    }
                    MenuItemAction::Start if !play_area_too_small => {
                        return MenuAction::StartGame;
                    }
                    MenuItemAction::OpenSettings => {
                        self.screen = MenuScreen::StartSettings;
                        self.start_settings_selected_idx = 0;
                    }
                    MenuItemAction::Quit => return MenuAction::Quit,
                    _ => {}
                }
            }
            GameInput::Pause => {}
            _ => {}
        }
        MenuAction::Consumed
    }

    fn start_menu_item_at(&self, index: usize) -> &MenuItem {
        if self.has_active_session {
            &START_MENU_ITEMS[index]
        } else {
            &START_MENU_ITEMS[index + 1]
        }
    }
}

fn wrap_next(current: usize, len: usize) -> usize {
    (current + 1) % len
}

fn wrap_prev(current: usize, len: usize) -> usize {
    if current == 0 { len - 1 } else { current - 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_menu_navigates_down_and_wraps() {
        let mut ms = MenuState::new(1, true);
        ms.set_has_active_session(false);
        ms.handle_start_main_input(GameInput::Direction(Direction::Down), false);
        assert_eq!(ms.start_selected_idx, 1);
        ms.handle_start_main_input(GameInput::Direction(Direction::Down), false);
        assert_eq!(ms.start_selected_idx, 2);
        ms.handle_start_main_input(GameInput::Direction(Direction::Down), false);
        assert_eq!(ms.start_selected_idx, 0); // wraps
    }

    #[test]
    fn start_menu_navigates_up_and_wraps() {
        let mut ms = MenuState::new(1, true);
        ms.set_has_active_session(false);
        ms.handle_start_main_input(GameInput::Direction(Direction::Up), false);
        assert_eq!(ms.start_selected_idx, 2); // wraps to last
    }

    #[test]
    fn start_game_action_returned() {
        let mut ms = MenuState::new(1, true);
        ms.set_has_active_session(false);
        assert_eq!(ms.start_selected_idx, 0); // "Start"
        let action = ms.handle_start_main_input(GameInput::Confirm, false);
        assert_eq!(action, MenuAction::StartGame);
    }

    #[test]
    fn start_game_blocked_when_too_small() {
        let mut ms = MenuState::new(1, true);
        ms.set_has_active_session(false);
        let action = ms.handle_start_main_input(GameInput::Confirm, true);
        assert_eq!(action, MenuAction::Consumed);
    }

    #[test]
    fn settings_opens_and_returns() {
        let mut ms = MenuState::new(1, true);
        ms.set_has_active_session(false);
        // Navigate to Settings (index 1)
        ms.handle_start_main_input(GameInput::Direction(Direction::Down), false);
        let action = ms.handle_start_main_input(GameInput::Confirm, false);
        assert_eq!(action, MenuAction::Consumed);
        assert_eq!(ms.screen, MenuScreen::StartSettings);

        // Go back
        ms.handle_start_settings_input(GameInput::Pause);
        assert_eq!(ms.screen, MenuScreen::StartMain);
    }

    #[test]
    fn speed_adjust_mode_changes_speed() {
        let mut ms = MenuState::new(1, true);
        ms.screen = MenuScreen::SpeedAdjust;
        ms.handle_speed_adjust_input(GameInput::Direction(Direction::Up));
        assert_eq!(ms.start_speed_level, 2);
        ms.handle_speed_adjust_input(GameInput::Direction(Direction::Down));
        assert_eq!(ms.start_speed_level, 1);
        // Can't go below minimum
        ms.handle_speed_adjust_input(GameInput::Direction(Direction::Down));
        assert_eq!(ms.start_speed_level, 1);
    }

    #[test]
    fn speed_adjust_clamped_to_max() {
        let mut ms = MenuState::new(MAX_START_SPEED_LEVEL, true);
        ms.screen = MenuScreen::SpeedAdjust;
        ms.handle_speed_adjust_input(GameInput::Direction(Direction::Up));
        assert_eq!(ms.start_speed_level, MAX_START_SPEED_LEVEL);
    }

    #[test]
    fn pause_resume_action() {
        let mut ms = MenuState::new(1, true);
        ms.enter_pause();
        assert_eq!(ms.pause_selected_idx, 0); // "Resume"
        let mut themes = ThemeCatalog::load();
        let action = ms.handle_pause_input(GameInput::Confirm, &mut themes, false);
        assert_eq!(action, MenuAction::Resume);
    }

    #[test]
    fn pause_resume_blocked_when_too_small() {
        let mut ms = MenuState::new(1, true);
        ms.enter_pause();
        let mut themes = ThemeCatalog::load();
        let action = ms.handle_pause_input(GameInput::Confirm, &mut themes, true);
        assert_eq!(action, MenuAction::Consumed);
    }

    #[test]
    fn game_over_restart_action() {
        let mut ms = MenuState::new(1, true);
        ms.enter_game_over();
        assert_eq!(ms.game_over_selected_idx, 0); // "Play Again"
        let action = ms.handle_game_over_input(GameInput::Confirm);
        assert_eq!(action, MenuAction::Restart);
    }

    #[test]
    fn game_over_quit_action() {
        let mut ms = MenuState::new(1, true);
        ms.enter_game_over();
        ms.handle_game_over_input(GameInput::Direction(Direction::Down));
        assert_eq!(ms.game_over_selected_idx, 1); // "Quit"
        let action = ms.handle_game_over_input(GameInput::Confirm);
        assert_eq!(action, MenuAction::Quit);
    }

    #[test]
    fn toggle_checkerboard() {
        let mut ms = MenuState::new(1, true);
        ms.screen = MenuScreen::StartSettings;
        // Navigate to Grid (index 2)
        ms.start_settings_selected_idx = 2;
        ms.handle_start_settings_input(GameInput::Confirm);
        assert!(!ms.checkerboard_enabled);
        ms.handle_start_settings_input(GameInput::Confirm);
        assert!(ms.checkerboard_enabled);
    }

    #[test]
    fn toggle_border() {
        let mut ms = MenuState::new(1, true);
        ms.screen = MenuScreen::StartSettings;
        // Navigate to Border (index 3)
        ms.start_settings_selected_idx = 3;
        ms.handle_start_settings_input(GameInput::Confirm);
        assert!(!ms.game_border_enabled);
    }

    #[test]
    fn declarative_menu_item_counts() {
        assert_eq!(START_MENU_ITEMS.len(), 4);
        assert_eq!(START_SETTINGS_ITEMS.len(), 5);
        assert_eq!(PAUSE_MENU_ITEMS.len(), 3);
        assert_eq!(GAME_OVER_MENU_ITEMS.len(), 2);
    }

    #[test]
    fn start_menu_resume_session_action() {
        let mut ms = MenuState::new(1, true);
        ms.set_has_active_session(true);
        let action = ms.handle_start_main_input(GameInput::Confirm, false);
        assert_eq!(action, MenuAction::ResumeSession);
    }

    #[test]
    fn wrap_next_wraps_at_boundary() {
        assert_eq!(wrap_next(2, 3), 0);
        assert_eq!(wrap_next(0, 3), 1);
    }

    #[test]
    fn wrap_prev_wraps_at_zero() {
        assert_eq!(wrap_prev(0, 3), 2);
        assert_eq!(wrap_prev(2, 3), 1);
    }
}

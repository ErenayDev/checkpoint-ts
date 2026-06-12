use super::app::{CheckpointTUI, Scene};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl CheckpointTUI {
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        {
            self.quit();
            return;
        }

        match self.current_scene {
            Scene::Dashboard => self.handle_dashboard_keys(key),
            Scene::Profiler => self.handle_profiler_keys(key),
        }
    }

    fn handle_dashboard_keys(&mut self, key: KeyEvent) {
        use crate::tui::scenes::dashboard::FocusedPanel;
        use ratatui_textarea::Input;

        // Modal: detail popup
        if self.dashboard_state.show_detail_popup {
            match key.code {
                KeyCode::Up => self.dashboard_state.scroll_detail_up(),
                KeyCode::Down => self.dashboard_state.scroll_detail_down(),
                KeyCode::Char('d')
                | KeyCode::Char('D')
                | KeyCode::Esc
                | KeyCode::Char('q')
                | KeyCode::Char('Q') => {
                    self.dashboard_state.close_detail_popup();
                }
                _ => {}
            }
            return;
        }

        // Modal: skip popup
        if self.dashboard_state.show_skip_popup {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => {
                    self.dashboard_state.close_skip_popup();
                }
                (KeyCode::Char('s'), m) | (KeyCode::Char('S'), m)
                    if m.contains(KeyModifiers::CONTROL) =>
                {
                    self.dashboard_state.skip_with_textarea_value();
                }
                _ => {
                    let input: Input = key.into();
                    self.dashboard_state.skip_textarea.input(input);
                }
            }
            return;
        }

        // Modal: edit popup
        if self.dashboard_state.show_edit_popup {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => {
                    self.dashboard_state.close_edit_popup();
                }
                (KeyCode::Char('s'), m) | (KeyCode::Char('S'), m)
                    if m.contains(KeyModifiers::CONTROL) =>
                {
                    self.dashboard_state.submit_edited_args();
                }
                _ => {
                    let input: Input = key.into();
                    self.dashboard_state.edit_textarea.input(input);
                }
            }
            return;
        }

        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.dashboard_state.continue_execution();
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if self.dashboard_state.paused {
                    self.dashboard_state.open_skip_popup();
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.dashboard_state.toggle_detail_popup();
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if self.dashboard_state.paused {
                    self.dashboard_state.open_edit_popup();
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.navigate_to(Scene::Profiler);
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                self.navigate_back();
            }
            KeyCode::Tab => {
                self.dashboard_state.toggle_focus();
            }
            KeyCode::Up => match self.dashboard_state.focused_panel {
                FocusedPanel::Timeline => self.dashboard_state.scroll_timeline_up(),
                FocusedPanel::Logs => self.dashboard_state.scroll_logs_up(),
            },
            KeyCode::Down => match self.dashboard_state.focused_panel {
                FocusedPanel::Timeline => self.dashboard_state.scroll_timeline_down(),
                FocusedPanel::Logs => self.dashboard_state.scroll_logs_down(),
            },
            _ => {}
        }
    }

    fn handle_profiler_keys(&mut self, key: KeyEvent) {
        use crate::tui::scenes::profiler::SortKey;

        match key.code {
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.profile_state.set_sort(SortKey::Total);
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.profile_state.set_sort(SortKey::Avg);
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.profile_state.set_sort(SortKey::Calls);
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.profile_state.set_sort(SortKey::Memory);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.profile_state.set_sort(SortKey::Name);
            }
            KeyCode::Up => {
                self.profile_state.scroll_up();
            }
            KeyCode::Down => {
                self.profile_state.scroll_down(usize::MAX, 1);
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.navigate_back();
            }
            _ => {}
        }
    }
}

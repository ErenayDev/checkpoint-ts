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
            Scene::VariableEditor => self.handle_variable_editor_keys(key),
            Scene::FunctionSkip => self.handle_function_skip_keys(key),
            Scene::History => self.handle_history_keys(key),
            Scene::Profiler => self.handle_profiler_keys(key),
            Scene::CallStack => self.handle_call_stack_keys(key),
            Scene::ErrorDialog => self.handle_error_dialog_keys(key),
        }
    }

    fn handle_dashboard_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.dashboard_state.continue_execution();
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if self.dashboard_state.paused {
                    self.navigate_to(Scene::FunctionSkip);
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if self.dashboard_state.paused {
                    self.navigate_to(Scene::VariableEditor);
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.navigate_to(Scene::Profiler);
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.navigate_to(Scene::CallStack);
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.navigate_to(Scene::History);
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                self.navigate_back();
            }
            KeyCode::Up => self.dashboard_state.scroll_logs_up(),
            KeyCode::Down => self.dashboard_state.scroll_logs_down(),
            _ => {}
        }
    }

    fn handle_variable_editor_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.dashboard_state
                    .add_log("Applied variable changes".to_string());
            }
            KeyCode::Tab => {
                self.dashboard_state
                    .add_log("Next field selected".to_string());
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.dashboard_state
                    .add_log("Reset all variables".to_string());
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_function_skip_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.dashboard_state.skip_function(serde_json::Value::Null);
                self.navigate_back();
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.dashboard_state.continue_execution();
                self.navigate_back();
            }
            KeyCode::Up | KeyCode::Down => {
                self.dashboard_state
                    .add_log("Skip option changed".to_string());
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_history_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                self.dashboard_state
                    .add_log("History navigation".to_string());
            }
            KeyCode::Enter => {
                self.dashboard_state
                    .add_log("Go to selected checkpoint".to_string());
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.dashboard_state
                    .add_log("Replay from selected point".to_string());
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.dashboard_state
                    .add_log("Show execution details".to_string());
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.dashboard_state
                    .add_log("Save execution history".to_string());
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_profiler_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.profile_state.save_report();
                self.dashboard_state
                    .add_log("Performance report saved".to_string());
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.profile_state.export_csv();
                self.dashboard_state.add_log("Export to CSV".to_string());
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.dashboard_state.add_log("Filter functions".to_string());
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_call_stack_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                self.dashboard_state
                    .add_log("Navigate call stack".to_string());
            }
            KeyCode::Enter => {
                self.dashboard_state
                    .add_log("Jump to selected frame".to_string());
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.navigate_to(Scene::VariableEditor);
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                self.dashboard_state.add_log("Breakpoint set".to_string());
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.dashboard_state.continue_execution();
                self.navigate_back();
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_error_dialog_keys(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.navigate_to(Scene::VariableEditor);
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.dashboard_state.skip_function(serde_json::Value::Null);
                self.navigate_back();
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.dashboard_state
                    .add_log("Restart from last checkpoint".to_string());
                self.navigate_back();
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.navigate_back();
            }
            _ => {}
        }
    }
}

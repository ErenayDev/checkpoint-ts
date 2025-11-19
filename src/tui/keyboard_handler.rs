use super::app::{Scene, TuiApp};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl TuiApp {
    pub fn handle_key_event(&mut self, key: KeyEvent) {
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
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.quit();
            }
            (_, KeyCode::Char('c')) => {
                self.dashboard_state
                    .add_log("Continue execution".to_string());
            }
            (_, KeyCode::Char('s')) => {
                self.navigate_to(Scene::FunctionSkip);
            }
            (_, KeyCode::Char('e')) => {
                self.navigate_to(Scene::VariableEditor);
            }
            (_, KeyCode::Char('p')) => {
                self.navigate_to(Scene::Profiler);
            }
            (_, KeyCode::Char('v')) => {
                self.navigate_to(Scene::CallStack);
            }
            (_, KeyCode::Char('h')) => {
                self.navigate_to(Scene::History);
            }
            (_, KeyCode::Esc | KeyCode::Char('q')) => {
                self.quit();
            }
            _ => {}
        }
    }

    fn handle_variable_editor_keys(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Enter) => {
                self.dashboard_state
                    .add_log("Applied variable changes".to_string());
            }
            (_, KeyCode::Tab) => {
                self.dashboard_state
                    .add_log("Next field selected".to_string());
            }
            (_, KeyCode::Char('r')) => {
                self.dashboard_state
                    .add_log("Reset all variables".to_string());
            }
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_function_skip_keys(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Enter) => {
                self.dashboard_state
                    .add_log("Function skipped with selected option".to_string());
                self.navigate_back();
            }
            (_, KeyCode::Char('c')) => {
                self.dashboard_state
                    .add_log("Continue normal execution".to_string());
                self.navigate_back();
            }
            (_, KeyCode::Up | KeyCode::Down) => {
                self.dashboard_state
                    .add_log("Skip option changed".to_string());
            }
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_history_keys(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Up | KeyCode::Down) => {
                self.dashboard_state
                    .add_log("History navigation".to_string());
            }
            (_, KeyCode::Enter) => {
                self.dashboard_state
                    .add_log("Go to selected checkpoint".to_string());
            }
            (_, KeyCode::Char('r')) => {
                self.dashboard_state
                    .add_log("Replay from selected point".to_string());
            }
            (_, KeyCode::Char('d')) => {
                self.dashboard_state
                    .add_log("Show execution details".to_string());
            }
            (_, KeyCode::Char('s')) => {
                self.dashboard_state
                    .add_log("Save execution history".to_string());
            }
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_profiler_keys(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('s')) => {
                self.profile_state.save_report();
                self.dashboard_state
                    .add_log("Performance report saved".to_string());
            }
            (_, KeyCode::Char('e')) => {
                self.profile_state.export_csv();
                self.dashboard_state.add_log("Export to CSV".to_string());
            }
            (_, KeyCode::Char('f')) => {
                self.dashboard_state.add_log("Filter functions".to_string());
            }
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_call_stack_keys(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Up | KeyCode::Down) => {
                self.dashboard_state
                    .add_log("Navigate call stack".to_string());
            }
            (_, KeyCode::Enter) => {
                self.dashboard_state
                    .add_log("Jump to selected frame".to_string());
            }
            (_, KeyCode::Char('v')) => {
                self.navigate_to(Scene::VariableEditor);
            }
            (_, KeyCode::Char('b')) => {
                self.dashboard_state.add_log("Breakpoint set".to_string());
            }
            (_, KeyCode::Char('c')) => {
                self.dashboard_state
                    .add_log("Continue execution".to_string());
                self.navigate_back();
            }
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.navigate_back();
            }
            _ => {}
        }
    }

    fn handle_error_dialog_keys(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('f')) => {
                self.navigate_to(Scene::VariableEditor);
            }
            (_, KeyCode::Char('s')) => {
                self.dashboard_state
                    .add_log("Function skipped with default value".to_string());
                self.navigate_back();
            }
            (_, KeyCode::Char('r')) => {
                self.dashboard_state
                    .add_log("Restart from last checkpoint".to_string());
                self.navigate_back();
            }
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.navigate_back();
            }
            _ => {}
        }
    }
}

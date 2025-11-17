use super::events::EventHandler;
use super::scenes::dashboard::DashboardState;
use crate::tui::scenes::variable_editor::VariableEditorState;
use color_eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, Frame};

#[derive(Debug, Clone, PartialEq)]
pub enum Scene {
    Dashboard,
    VariableEditor,
    FunctionSkip,
    History,
    Profiler,
    CallStack,
    ErrorDialog,
}

#[derive(Debug)]
pub struct TuiApp {
    pub current_scene: Scene,
    pub scene_stack: Vec<Scene>,
    pub running: bool,
    event_handler: EventHandler,
    dashboard_state: DashboardState,
    variable_state: VariableEditorState,
}

impl TuiApp {
    pub fn new(file: Option<String>) -> Self {
        let mut dashboard_state = DashboardState::new();
        let variable_state = VariableEditorState::new();
        if let Some(f) = file {
            dashboard_state.set_file(f);
        }
        Self {
            current_scene: Scene::Dashboard,
            scene_stack: vec![],
            running: true,
            event_handler: EventHandler::new(),
            dashboard_state,
            variable_state,
        }
    }

    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(150));
        while self.running {
            tokio::select! {
                _ = interval.tick() => {
                    self.dashboard_state.tick_throbber();
                }
                _ = self.handle_events() => {}
            }
            terminal.draw(|frame| self.draw(frame))?;
        }
        Ok(())
    }

    async fn handle_events(&mut self) -> Result<()> {
        if let Some(event) = self.event_handler.next_event().await? {
            match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    self.handle_key_event(key);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn navigate_to(&mut self, new_scene: Scene) {
        self.scene_stack.push(self.current_scene.clone());
        self.current_scene = new_scene;
        self.dashboard_state
            .add_log(format!("Navigated to {:?}", self.current_scene));
    }

    fn navigate_back(&mut self) {
        if let Some(previous_scene) = self.scene_stack.pop() {
            self.current_scene = previous_scene;
            self.dashboard_state
                .add_log(format!("Navigated back to {:?}", self.current_scene));
        } else {
            self.quit();
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match self.current_scene {
            Scene::Dashboard => match (key.modifiers, key.code) {
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
            },

            Scene::VariableEditor => match (key.modifiers, key.code) {
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
            },

            Scene::FunctionSkip => match (key.modifiers, key.code) {
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
            },

            Scene::History => match (key.modifiers, key.code) {
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
            },

            Scene::Profiler => match (key.modifiers, key.code) {
                (_, KeyCode::Char('s')) => {
                    self.dashboard_state
                        .add_log("Performance report saved".to_string());
                }
                (_, KeyCode::Char('e')) => {
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
            },

            Scene::CallStack => match (key.modifiers, key.code) {
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
            },

            Scene::ErrorDialog => match (key.modifiers, key.code) {
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
            },
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }

    fn draw(&mut self, frame: &mut Frame) {
        match self.current_scene {
            Scene::Dashboard => self.draw_dashboard(frame),
            Scene::VariableEditor => self.draw_variable_editor(frame),
            Scene::FunctionSkip => self.draw_function_skip(frame),
            Scene::History => self.draw_history(frame),
            Scene::Profiler => self.draw_profiler(frame),
            Scene::CallStack => self.draw_call_stack(frame),
            Scene::ErrorDialog => self.draw_error_dialog(frame),
        }
    }

    fn draw_dashboard(&mut self, frame: &mut Frame) {
        crate::tui::scenes::dashboard::draw(frame, frame.area(), &mut self.dashboard_state);
    }

    fn draw_variable_editor(&self, frame: &mut Frame) {
        crate::tui::scenes::variable_editor::draw(frame, frame.area(), &self.variable_state);
    }

    fn draw_function_skip(&self, frame: &mut Frame) {
        // Placeholder
        use ratatui::widgets::{Block, Borders, Paragraph};
        frame.render_widget(
            Paragraph::new("Function Skip Dialog\n\n[Enter] Skip with Selected Option    [Esc] Cancel    [C] Continue Normal")
                .block(Block::default().borders(Borders::ALL).title("Skip Function")),
            frame.area(),
        );
    }

    fn draw_history(&self, frame: &mut Frame) {
        // Placeholder
        use ratatui::widgets::{Block, Borders, Paragraph};
        frame.render_widget(
            Paragraph::new("Execution History\n\n[↑↓] Navigate    [Enter] Go to Checkpoint    [R] Replay from Here\n[D] Show Details [S] Save History           [Esc] Back")
                .block(Block::default().borders(Borders::ALL).title("Execution History")),
            frame.area(),
        );
    }

    fn draw_profiler(&self, frame: &mut Frame) {
        // Placeholder
        use ratatui::widgets::{Block, Borders, Paragraph};
        frame.render_widget(
            Paragraph::new("Performance Profile\n\n[S] Save Report    [E] Export CSV    [F] Filter Functions    [Esc] Back")
                .block(Block::default().borders(Borders::ALL).title("Performance Profile")),
            frame.area(),
        );
    }

    fn draw_call_stack(&self, frame: &mut Frame) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        frame.render_widget(
            Paragraph::new("Call Stack Viewer\n\n[↑↓] Navigate Stack    [Enter] Jump to Frame    [V] View Frame Variables\n[B] Set Breakpoint     [C] Continue             [Esc] Back")
                .block(Block::default().borders(Borders::ALL).title("Call Stack")),
            frame.area(),
        );
    }

    fn draw_error_dialog(&self, frame: &mut Frame) {
        // Placeholder
        use ratatui::widgets::{Block, Borders, Paragraph};
        frame.render_widget(
            Paragraph::new("Execution Error\n\n[F] Fix variable and continue    [S] Skip function with default value\n[R] Restart from last checkpoint [Q] Quit execution")
                .block(Block::default().borders(Borders::ALL).title("Execution Error")),
            frame.area(),
        );
    }
}

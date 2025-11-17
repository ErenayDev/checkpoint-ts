use super::scenes::dashboard::DashboardState;

use super::events::EventHandler;
use color_eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, Frame};

#[derive(Debug, Clone)]
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
    pub running: bool,
    event_handler: EventHandler,
    dashboard_state: DashboardState,
}

impl TuiApp {
    pub fn new(file: Option<String>) -> Self {
        let mut dashboard_state = DashboardState::new();
        if let Some(f) = file {
            dashboard_state.set_file(f);
        }

        Self {
            current_scene: Scene::Dashboard,
            running: true,
            event_handler: EventHandler::new(),
            dashboard_state, // Değiştir
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

    fn handle_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.quit();
            }
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }

    fn draw(&mut self, frame: &mut Frame) {
        match self.current_scene {
            Scene::Dashboard => self.draw_dashboard(frame),
            Scene::VariableEditor => self.draw_variable_editor(frame),
            _ => {}
        }
    }

    fn draw_dashboard(&mut self, frame: &mut Frame) {
        crate::tui::scenes::dashboard::draw(frame, frame.area(), &mut self.dashboard_state);
    }

    fn draw_variable_editor(&self, frame: &mut Frame) {
        crate::tui::scenes::variable_editor::draw(frame, frame.area());
    }
}

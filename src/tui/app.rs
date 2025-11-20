use super::events::EventHandler;
use super::scenes::dashboard::DashboardState;
use super::scenes::profiler::PerformanceProfileState;
use super::scenes::variable_editor::VariableEditorState;
use color_eyre::Result;
use crossterm::event::{Event, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};

#[allow(dead_code)]
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
    pub dashboard_state: DashboardState,
    pub variable_state: VariableEditorState,
    pub profile_state: PerformanceProfileState,
}

impl TuiApp {
    pub fn new(file: Option<String>) -> Self {
        let mut dashboard_state = DashboardState::new();
        let variable_state = VariableEditorState::new();
        let profile_state = PerformanceProfileState::new();

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
            profile_state,
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

    pub fn navigate_to(&mut self, new_scene: Scene) {
        self.scene_stack.push(self.current_scene.clone());
        self.current_scene = new_scene;
        self.dashboard_state
            .add_log(format!("Navigated to {:?}", self.current_scene));
    }

    pub fn navigate_back(&mut self) {
        if let Some(previous_scene) = self.scene_stack.pop() {
            self.current_scene = previous_scene;
            self.dashboard_state
                .add_log(format!("Navigated back to {:?}", self.current_scene));
        } else {
            self.quit();
        }
    }

    pub fn quit(&mut self) {
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
        use ratatui::widgets::{Block, Borders, Paragraph};
        frame.render_widget(
            Paragraph::new("Function Skip Dialog\n\n[Enter] Skip with Selected Option    [Esc] Cancel    [C] Continue Normal")
                .block(Block::default().borders(Borders::ALL).title("Skip Function")),
            frame.area(),
        );
    }

    fn draw_history(&self, frame: &mut Frame) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        frame.render_widget(
            Paragraph::new("Execution History\n\n[↑↓] Navigate    [Enter] Go to Checkpoint    [R] Replay from Here\n[D] Show Details [S] Save History           [Esc] Back")
                .block(Block::default().borders(Borders::ALL).title("Execution History")),
            frame.area(),
        );
    }

    fn draw_profiler(&self, frame: &mut Frame) {
        crate::tui::scenes::profiler::draw(frame, frame.area(), &self.profile_state);
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
        use ratatui::widgets::{Block, Borders, Paragraph};
        frame.render_widget(
            Paragraph::new("Execution Error\n\n[F] Fix variable and continue    [S] Skip function with default value\n[R] Restart from last checkpoint [Q] Quit execution")
                .block(Block::default().borders(Borders::ALL).title("Execution Error")),
            frame.area(),
        );
    }
}

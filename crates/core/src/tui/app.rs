use super::events::EventHandler;
use super::scenes::dashboard::DashboardState;
use super::scenes::profiler::PerformanceProfileState;
use crate::utils::discord_rpc::RpcSession;
use color_eyre::Result;
use crossterm::event::{Event, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Scene {
    Dashboard,
    Profiler,
}

#[derive(Debug)]
pub struct CheckpointTUI {
    pub current_scene: Scene,
    pub scene_stack: Vec<Scene>,
    pub running: bool,
    event_handler: EventHandler,
    pub dashboard_state: DashboardState,
    pub profile_state: PerformanceProfileState,
    pub rpc: RpcSession,
}

impl CheckpointTUI {
    pub fn new(
        transformed_file: Option<String>,
        original_file: Option<String>,
        runtime_dir: Option<std::path::PathBuf>,
        rpc: RpcSession,
    ) -> Self {
        let mut dashboard_state = DashboardState::new();
        let profile_state = PerformanceProfileState::new();

        if let (Some(transformed), Some(original), Some(rt_dir)) =
            (transformed_file, original_file, runtime_dir)
        {
            let _ = dashboard_state.set_file(transformed, original, rt_dir);
        }

        Self {
            current_scene: Scene::Dashboard,
            scene_stack: vec![],
            running: true,
            event_handler: EventHandler::new(),
            dashboard_state,
            profile_state,
            rpc,
        }
    }

    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(150));
        while self.running {
            tokio::select! {
                _ = interval.tick() => {
                    self.dashboard_state.tick_throbber();
                }
                result = self.handle_events() => {
                                  result?;
                               }
            }
            terminal.draw(|frame| self.draw(frame))?;
            self.dashboard_state.poll_ipc_messages();
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
        self.current_scene = new_scene.clone();

        self.rpc.activity.state = Some(format!(" {:?}", new_scene));
        self.rpc.update();

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
            Scene::Profiler => self.draw_profiler(frame),
        }
    }

    fn draw_dashboard(&mut self, frame: &mut Frame) {
        crate::tui::scenes::dashboard::draw(frame, frame.area(), &mut self.dashboard_state);
    }

    fn draw_profiler(&mut self, frame: &mut Frame) {
        crate::tui::scenes::profiler::draw(
            frame,
            frame.area(),
            &mut self.profile_state,
            &self.dashboard_state,
        );
    }
}

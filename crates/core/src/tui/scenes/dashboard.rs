use crate::services::IpcBridge;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};
use throbber_widgets_tui::ThrobberState;

#[derive(Debug)]
pub struct DashboardState {
    pub file_path: Option<String>,
    pub transformed_path: Option<String>,
    pub ipc_bridge: Option<IpcBridge>,
    pub runtime: String,
    pub execution_time: String,
    pub status: String,
    pub current_function: Option<String>,
    pub current_line: Option<u32>,
    pub called_by: Option<String>,
    pub stack_depth: u32,
    pub timeline_functions: Vec<TimelineFunction>,
    pub logs: Vec<String>,
    pub log_visible_height: usize,
    pub log_scroll_offset: usize,
    pub throbber_state: ThrobberState,
    pub paused: bool,
    pub app_loading: bool,
    pub pending_checkpoints: Vec<CheckpointData>,
}

#[derive(Clone, Debug)]
pub struct CheckpointData {
    pub id: u64,
    pub function_name: String,
    pub args: Vec<serde_json::Value>,
    pub context: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TimelineFunction {
    pub name: String,
    pub status: FunctionStatus,
    pub duration: Option<String>,
    pub checkpoint_id: u64,
}

#[derive(Clone, Debug)]
pub enum FunctionStatus {
    Completed,
    Skipped,
    Current,
    Pending,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self {
            file_path: None,
            transformed_path: None,
            ipc_bridge: None,
            runtime: "Bun v1.3.10".to_string(),
            execution_time: "0s".to_string(),
            status: "Ready".to_string(),
            current_function: None,
            current_line: None,
            called_by: None,
            stack_depth: 0,
            throbber_state: ThrobberState::default(),
            paused: false,
            timeline_functions: vec![],
            logs: vec!["System initialized".to_string()],
            log_scroll_offset: 0,
            log_visible_height: 20,
            app_loading: false,
            pending_checkpoints: Vec::new(),
        }
    }
}

impl DashboardState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scroll_logs_up(&mut self) {
        if self.log_scroll_offset > 0 {
            self.log_scroll_offset -= 1;
        }
    }

    pub fn scroll_logs_down(&mut self) {
        let max_scroll = self.logs.len().saturating_sub(self.log_visible_height);
        if self.log_scroll_offset < max_scroll {
            self.log_scroll_offset += 1;
        }
    }

    pub fn scroll_logs_to_bottom(&mut self) {
        self.log_scroll_offset = self.logs.len().saturating_sub(self.log_visible_height);
    }

    pub fn poll_ipc_messages(&mut self) {
        while let Some(checkpoint) = if let Some(ref mut bridge) = self.ipc_bridge {
            bridge.receive_checkpoint_json::<serde_json::Value>(1)
        } else {
            None
        } {
            self.handle_checkpoint_message(checkpoint);
        }

        if let Some(message) = if let Some(ref mut bridge) = self.ipc_bridge {
            bridge.receive_status_json::<serde_json::Value>(10)
        } else {
            None
        } {
            self.handle_status_message(message);
        }
    }

    fn handle_status_message(&mut self, message: serde_json::Value) {
        if let Some(log_msg) = message.get("log").and_then(|v| v.as_str()) {
            // skip duplicate "runtime_ready" logs
            if !log_msg.contains("Runtime ready, waiting for commands") {
                self.add_log(log_msg.to_string());
            }
        }

        if let Some(msg_type) = message.get("type").and_then(|v| v.as_str()) {
            match msg_type {
                "runtime_ready" => {
                    // we already logged in spawn_runtime func. so just skip the logging in here
                }
                "version" => {
                    if let Some(value) = message.get("value") {
                        if let Some(lv) = value.get("lv").and_then(|v| v.as_str()) {
                            self.runtime = lv.to_string();
                            self.add_log(format!("Runtime version: {}", lv));
                        }
                    }
                }
                "error" => {
                    if let Some(error_msg) = message.get("message").and_then(|v| v.as_str()) {
                        self.add_log(format!("ERROR: {}", error_msg));
                    } else if let Some(error_msg) = message.get("log").and_then(|v| v.as_str()) {
                        self.add_log(format!("ERROR: {}", error_msg));
                    }
                }
                _ => {
                    self.add_log(format!("Status: {}", msg_type));
                }
            }
        }

        if self.app_loading {
            if let Some(log_msg) = message.get("log").and_then(|v| v.as_str()) {
                if log_msg.contains("Application loaded and ready") {
                    self.app_loading = false;
                    self.status = "Running".to_string();
                    self.add_log("Application fully loaded, ready for debugging".to_string());
                }
            }
        }
    }

    fn handle_checkpoint_message(&mut self, checkpoint: serde_json::Value) {
        let checkpoint_id = checkpoint.get("id").and_then(|v| v.as_u64()).unwrap_or(0);

        if let Some(payload) = checkpoint.get("payload") {
            let func_name = payload
                .get("functionName")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let args = payload
                .get("args")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            let context = payload
                .get("context")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            self.add_log(format!("Checkpoint: {}", func_name));

            self.pending_checkpoints.push(CheckpointData {
                id: checkpoint_id,
                function_name: func_name.to_string(),
                args,
                context,
            });

            if self.pending_checkpoints.len() == 1 {
                self.current_function = Some(func_name.to_string());
                self.paused = true;
                self.status = "Paused".to_string();
            }

            self.timeline_functions.push(TimelineFunction {
                name: func_name.to_string(),
                status: FunctionStatus::Pending,
                checkpoint_id,
                duration: None,
            });
        }
    }

    pub fn continue_execution(&mut self) {
        if !self.pending_checkpoints.is_empty() {
            let checkpoint = self.pending_checkpoints.remove(0);

            self.add_log(format!(
                "→ Sending continue response for checkpoint ID {}",
                checkpoint.id
            ));

            let result = if let Some(ref bridge) = self.ipc_bridge {
                bridge.send_checkpoint_response_json(&serde_json::json!({
                    "id": checkpoint.id,
                    "action": "continue"
                }))
            } else {
                return;
            };

            if let Err(e) = result {
                self.add_log(format!("ERROR: Failed to send continue: {}", e));
                return;
            }

            self.add_log("Continue response sent successfully".to_string());

            if let Some(func) = self
                .timeline_functions
                .iter_mut()
                .find(|f| f.checkpoint_id == checkpoint.id)
            {
                func.status = FunctionStatus::Completed;
            }

            if self.pending_checkpoints.is_empty() {
                self.paused = false;
                self.status = "Running".to_string();
                self.current_function = None;
            } else {
                self.current_function = Some(self.pending_checkpoints[0].function_name.clone());
            }

            self.add_log("Execution continued".to_string());
        }
    }

    pub fn skip_function(&mut self, return_value: serde_json::Value) {
        if !self.pending_checkpoints.is_empty() {
            let checkpoint = self.pending_checkpoints.remove(0);

            if let Some(ref bridge) = self.ipc_bridge {
                if let Err(e) = bridge.send_checkpoint_response_json(&serde_json::json!({
                "id": checkpoint.id,
                "action": "skip",
                "returnValue": return_value
                            }))
                {
                    self.add_log(format!("ERROR: Failed to send skip response: {}", e));
                    return;
                }
            }

            if let Some(func) = self
                .timeline_functions
                .iter_mut()
                .find(|f| f.name == checkpoint.function_name)
            {
                func.status = FunctionStatus::Skipped;
            }

            self.add_log("Function skipped".to_string());

            if self.pending_checkpoints.is_empty() {
                self.paused = false;
                self.status = "Running".to_string();
                self.current_function = None;
            } else {
                self.current_function = Some(self.pending_checkpoints[0].function_name.clone());
            }
        }
    }

    pub fn set_file(
        &mut self,
        transformed_file: String,
        original_file: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.file_path = Some(original_file);
        self.transformed_path = Some(transformed_file.clone());

        match IpcBridge::spawn_runtime(&transformed_file) {
            Ok(mut bridge) => {
                bridge.set_log_callback(|_message| {});
                self.status = "Starting".to_string();
                self.add_log("Runtime started, loading application...".to_string());

                if let Err(e) = bridge.load_app() {
                    self.status = "Error".to_string();
                    self.add_log(format!("Failed to load application: {}", e));
                    return Err(e);
                }

                self.ipc_bridge = Some(bridge);
                self.app_loading = true; // Set flag
                self.status = "Loading".to_string();

                Ok(())
            }
            Err(e) => {
                self.status = "Error".to_string();
                self.add_log(format!("Failed to start runtime: {}", e));
                Err(e)
            }
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(ref mut bridge) = self.ipc_bridge {
            bridge.shutdown();
        }
        self.ipc_bridge = None;
        self.status = "Stopped".to_string();
    }

    pub fn tick_throbber(&mut self) {
        self.throbber_state.calc_next();
    }

    pub fn add_log(&mut self, message: String) {
        let log_line = format!("[{}] {}", chrono::Local::now().format("%H:%M:%S"), message);
        self.logs.push(log_line);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
        self.scroll_logs_to_bottom();
    }
}

impl Drop for DashboardState {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &mut DashboardState) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(frame, main_layout[0], state);
    draw_current_checkpoint(frame, main_layout[1], state);
    draw_timeline(frame, main_layout[2], state);
    draw_status_logs(frame, main_layout[3], state);
    draw_quick_actions(frame, main_layout[4]);
}

fn draw_header(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let file_display = state
        .file_path
        .as_ref()
        .map(|f| f.trim_matches('"'))
        .unwrap_or("No file");

    let total_width = area.width.saturating_sub(2) as usize;
    let half_width = total_width / 2;

    let line1_left = format!("File: {}", file_display);
    let line1_right = format!("Runtime: {}", state.runtime);
    let line1 = format!("{:<width$}{}", line1_left, line1_right, width = half_width);

    let status_display = if state.paused {
        format!("⏸ Paused ({} queued)", state.pending_checkpoints.len())
    } else {
        state.status.clone()
    };

    let line2_left = format!("Status: {}", status_display);
    let line2_right = format!("Execution: {}", state.execution_time);
    let line2 = format!("{:<width$}{}", line2_left, line2_right, width = half_width);

    let combined_text = format!("{}\n{}", line1, line2);

    frame.render_widget(
        Paragraph::new(combined_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        "[ ".into(),
                        "Dashboard".blue().bold(),
                        " ]".into(),
                    ])),
            )
            .style(if state.paused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
        area,
    );
}

fn draw_current_checkpoint(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let total_width = area.width.saturating_sub(2) as usize;
    let half_width = total_width / 2;

    let line1_left = if let Some(checkpoint) = state.pending_checkpoints.first() {
        format!("Function: {}()", checkpoint.function_name)
    } else {
        "Function: None".to_string()
    };

    let line1_right = format!("Queue: {}", state.pending_checkpoints.len());
    let line1 = format!("{:<width$}{}", line1_left, line1_right, width = half_width);

    let line2_left = match &state.called_by {
        Some(caller) => format!("Called by: {}()", caller),
        None => "Called by: <root>".to_string(),
    };

    let line2_right = format!("Stack depth: {}", state.stack_depth);
    let line2 = format!("{:<width$}{}", line2_left, line2_right, width = half_width);

    let combined_text = format!("{}\n{}", line1, line2);

    frame.render_widget(
        Paragraph::new(combined_text).block(Block::default().borders(Borders::ALL).title(
            Line::from(vec![
                "[ ".into(),
                "Current Checkpoint".yellow().bold(),
                " ]".into(),
            ]),
        )),
        area,
    );
}

fn draw_timeline(frame: &mut Frame, area: Rect, state: &mut DashboardState) {
    let functions = &state.timeline_functions;

    if functions.is_empty() {
        frame.render_widget(
            Paragraph::new("No functions executed yet").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        "[ ".into(),
                        "Execution Timeline".yellow().bold(),
                        " ]".into(),
                    ])),
            ),
            area,
        );
        return;
    }

    let timeline_items: Vec<ListItem> = functions
        .iter()
        .map(|func| {
            let status_symbol = match func.status {
                FunctionStatus::Completed => "✓",
                FunctionStatus::Skipped => "⊘",
                FunctionStatus::Current => "►",
                FunctionStatus::Pending => "○",
            };

            let style = match func.status {
                FunctionStatus::Completed => Style::default().fg(Color::Green),
                FunctionStatus::Skipped => Style::default().fg(Color::Yellow),
                FunctionStatus::Current => Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                FunctionStatus::Pending => Style::default().fg(Color::Gray),
            };

            let duration_str = func
                .duration
                .as_ref()
                .map(|d| format!(" ({})", d))
                .unwrap_or_default();

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", status_symbol), style),
                Span::styled(func.name.clone(), style),
                Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(timeline_items).block(Block::default().borders(Borders::ALL).title(
        Line::from(vec![
            "[ ".into(),
            "Execution Timeline".yellow().bold(),
            " ]".into(),
        ]),
    ));

    frame.render_widget(list, area);
}

fn draw_status_logs(frame: &mut Frame, area: Rect, state: &mut DashboardState) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let total_logs = state.logs.len();

    state.log_visible_height = visible_height;

    let start_idx = state.log_scroll_offset;
    let end_idx = (start_idx + visible_height).min(total_logs);

    let display_logs: Vec<_> = state.logs[start_idx..end_idx].iter().collect();

    let log_items: Vec<ListItem> = display_logs
        .iter()
        .map(|log| {
            let style = if log.contains("ERROR") || log.contains("error") {
                Style::default().fg(Color::Red)
            } else if log.contains("WARN") || log.contains("warn") {
                Style::default().fg(Color::Yellow)
            } else if log.contains("→") || log.contains("←") {
                Style::default().fg(Color::Cyan)
            } else if log.contains("Checkpoint:") {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default().fg(Color::Gray)
            };

            ListItem::new(Line::from(Span::styled(log.as_str(), style)))
        })
        .collect();

    let list = List::new(log_items).block(Block::default().borders(Borders::ALL).title(
        Line::from(vec![
            "[ ".into(),
            "Status & Logs".cyan().bold(),
            format!(" ({}/{}) ", end_idx, total_logs).into(),
            " ]".into(),
        ]),
    ));

    frame.render_widget(list, area);

    if total_logs > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(total_logs.saturating_sub(visible_height))
            .position(state.log_scroll_offset);

        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

fn draw_quick_actions(frame: &mut Frame, area: Rect) {
    let actions_text =
        "[C] Continue    [S] Skip Function    [↑↓] Scroll Logs    [H] History    [Q] Quit";

    frame.render_widget(
        Paragraph::new(actions_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        "[ ".into(),
                        "Quick Actions".green().bold(),
                        " ]".into(),
                    ]))
                    .title_bottom(
                        Line::from(vec![
                            "[ ".into(),
                            "Made by ErenayDev <3".magenta().italic(),
                            " ]".into(),
                        ])
                        .alignment(Alignment::Right),
                    ),
            )
            .alignment(Alignment::Center),
        area,
    );
}

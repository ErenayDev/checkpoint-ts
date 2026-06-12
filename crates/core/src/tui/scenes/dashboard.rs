use crate::services::IpcBridge;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};
use ratatui_textarea::TextArea;
use throbber_widgets_tui::{BRAILLE_SIX, Throbber, ThrobberState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusedPanel {
    Timeline,
    Logs,
}

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
    pub timeline_scroll_offset: usize,
    pub timeline_visible_height: usize,
    pub logs: Vec<String>,
    pub log_visible_height: usize,
    pub log_scroll_offset: usize,
    pub throbber_state: ThrobberState,
    pub paused: bool,
    pub app_loading: bool,
    pub pending_checkpoints: Vec<CheckpointData>,
    pub focused_panel: FocusedPanel,
    pub promoted_functions: std::collections::HashSet<String>,
    pub show_detail_popup: bool,
    pub detail_scroll_offset: u16,
    pub detail_content_lines: usize,
    pub detail_visible_height: u16,
    pub show_skip_popup: bool,
    pub skip_textarea: TextArea<'static>,
    pub show_edit_popup: bool,
    pub edit_textarea: TextArea<'static>,
    pub verbose: bool,
    pub profile_stats: std::collections::HashMap<String, ProfileStats>,
}

#[derive(Clone, Debug)]
pub struct CheckpointData {
    pub id: u64,
    pub function_name: String,
    pub args: Vec<serde_json::Value>,
    pub context: Option<String>,
    pub stack_depth: u32,
    pub caller_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TimelineFunction {
    pub name: String,
    pub status: FunctionStatus,
    pub duration: Option<String>,
    pub checkpoint_id: u64,
    pub return_preview: Option<String>,
    pub return_type: Option<String>,
    pub return_value: Option<serde_json::Value>,
    pub error_preview: Option<String>,
    pub is_promoted: bool,
    pub stack_depth: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ProfileStats {
    pub call_count: u32,
    pub durations_ms: Vec<f64>, // sorted on insert for fast percentile
    pub total_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub mem_deltas: Vec<i64>,
}

impl ProfileStats {
    pub fn record(&mut self, duration_ms: f64, mem_delta: i64) {
        self.call_count += 1;
        self.total_ms += duration_ms;

        if self.call_count == 1 {
            self.min_ms = duration_ms;
            self.max_ms = duration_ms;
        } else {
            if duration_ms < self.min_ms {
                self.min_ms = duration_ms;
            }
            if duration_ms > self.max_ms {
                self.max_ms = duration_ms;
            }
        }

        // insert sorted
        let pos = self
            .durations_ms
            .binary_search_by(|x| {
                x.partial_cmp(&duration_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_else(|p| p);
        self.durations_ms.insert(pos, duration_ms);

        self.mem_deltas.push(mem_delta);
    }

    pub fn avg_ms(&self) -> f64 {
        if self.call_count == 0 {
            0.0
        } else {
            self.total_ms / self.call_count as f64
        }
    }

    pub fn p95_ms(&self) -> f64 {
        if self.durations_ms.is_empty() {
            return 0.0;
        }
        let n = self.durations_ms.len();
        let idx = ((n as f64 * 0.95) as usize).min(n - 1);
        self.durations_ms[idx]
    }

    pub fn avg_mem_delta(&self) -> i64 {
        if self.mem_deltas.is_empty() {
            0
        } else {
            let sum: i64 = self.mem_deltas.iter().sum();
            sum / self.mem_deltas.len() as i64
        }
    }
}

#[derive(Clone, Debug)]
pub enum FunctionStatus {
    Completed,
    Failed,
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
            runtime: "Unknown".to_string(),
            execution_time: "0ms".to_string(),
            status: "Unknown".to_string(),
            current_function: None,
            current_line: None,
            called_by: None,
            stack_depth: 0,
            throbber_state: ThrobberState::default(),
            paused: false,
            timeline_functions: vec![],
            timeline_scroll_offset: 0,
            timeline_visible_height: 10,
            logs: vec!["System initialized".to_string()],
            log_scroll_offset: 0,
            log_visible_height: 20,
            app_loading: false,
            pending_checkpoints: Vec::new(),
            focused_panel: FocusedPanel::Logs, // default for logs panel
            promoted_functions: std::collections::HashSet::new(),
            show_detail_popup: false,
            detail_scroll_offset: 0,
            detail_content_lines: 0,
            detail_visible_height: 0,
            show_skip_popup: false,
            skip_textarea: TextArea::default(),
            show_edit_popup: false,
            edit_textarea: TextArea::default(),
            verbose: false,
            profile_stats: std::collections::HashMap::new(),
        }
    }
}

impl DashboardState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
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

    pub fn scroll_timeline_up(&mut self) {
        if self.timeline_scroll_offset > 0 {
            self.timeline_scroll_offset -= 1;
        }
    }

    pub fn scroll_timeline_down(&mut self) {
        let max_scroll = self
            .timeline_functions
            .len()
            .saturating_sub(self.timeline_visible_height);
        if self.timeline_scroll_offset < max_scroll {
            self.timeline_scroll_offset += 1;
        }
    }

    pub fn scroll_timeline_to_bottom(&mut self) {
        self.timeline_scroll_offset = self
            .timeline_functions
            .len()
            .saturating_sub(self.timeline_visible_height);
    }

    pub fn toggle_focus(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Timeline => FocusedPanel::Logs,
            FocusedPanel::Logs => FocusedPanel::Timeline,
        };
    }

    pub fn toggle_detail_popup(&mut self) {
        if self.show_detail_popup {
            self.close_detail_popup();
        } else if !self.pending_checkpoints.is_empty() {
            self.show_detail_popup = true;
            self.detail_scroll_offset = 0;
        }
    }

    pub fn close_detail_popup(&mut self) {
        self.show_detail_popup = false;
        self.detail_scroll_offset = 0;
    }

    pub fn scroll_detail_up(&mut self) {
        self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(1);
    }

    pub fn scroll_detail_down(&mut self) {
        let max_scroll = self
            .detail_content_lines
            .saturating_sub(self.detail_visible_height as usize) as u16;
        if self.detail_scroll_offset < max_scroll {
            self.detail_scroll_offset += 1;
        }
    }

    pub fn open_skip_popup(&mut self) {
        if self.pending_checkpoints.is_empty() {
            return;
        }
        self.skip_textarea = TextArea::default();
        self.show_skip_popup = true;
    }

    pub fn close_skip_popup(&mut self) {
        self.show_skip_popup = false;
        self.skip_textarea = TextArea::default();
    }

    pub fn skip_textarea_value(&self) -> String {
        self.skip_textarea.lines().join("\n")
    }

    pub fn skip_with_textarea_value(&mut self) {
        let raw = self.skip_textarea_value();
        let trimmed = raw.trim();

        let return_value = if trimmed.is_empty() {
            serde_json::Value::Null
        } else {
            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    self.add_log(format!("ERROR: Invalid JSON, skip aborted: {}", e));
                    return;
                }
            }
        };

        self.skip_function(return_value);
        self.close_skip_popup();
    }
    pub fn open_edit_popup(&mut self) {
        let checkpoint = match self.pending_checkpoints.first() {
            Some(c) => c,
            None => return,
        };

        let args_pretty =
            serde_json::to_string_pretty(&checkpoint.args).unwrap_or_else(|_| "[]".to_string());

        let lines: Vec<String> = args_pretty.lines().map(|l| l.to_string()).collect();
        self.edit_textarea = TextArea::new(lines);
        self.show_edit_popup = true;
    }

    pub fn close_edit_popup(&mut self) {
        self.show_edit_popup = false;
        self.edit_textarea = TextArea::default();
    }

    pub fn edit_textarea_value(&self) -> String {
        self.edit_textarea.lines().join("\n")
    }

    pub fn submit_edited_args(&mut self) {
        let raw = self.edit_textarea_value();
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            self.add_log("ERROR: Args cannot be empty".to_string());
            return;
        }

        let parsed: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                self.add_log(format!("ERROR: Invalid JSON, edit aborted: {}", e));
                return;
            }
        };

        let new_args = match parsed {
            serde_json::Value::Array(arr) => arr,
            _ => {
                self.add_log("ERROR: Args must be a JSON array".to_string());
                return;
            }
        };

        self.continue_with_args(new_args);
        self.close_edit_popup();
    }

    pub fn continue_with_args(&mut self, new_args: Vec<serde_json::Value>) {
        if self.pending_checkpoints.is_empty() {
            return;
        }

        let checkpoint = self.pending_checkpoints.remove(0);

        self.add_log_verbose(format!(
            "→ Sending continue_with_args for checkpoint ID {}",
            checkpoint.id
        ));

        let result = if let Some(ref bridge) = self.ipc_bridge {
            bridge.send_checkpoint_response_json(&serde_json::json!({
                "id": checkpoint.id,
                "action": "continue_with_args",
                "args": new_args,
            }))
        } else {
            return;
        };

        if let Err(e) = result {
            self.add_log(format!("ERROR: Failed to send continue_with_args: {}", e));
            return;
        }

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
            self.stack_depth = 0;
            self.called_by = None;
        } else {
            let next = &self.pending_checkpoints[0];
            self.current_function = Some(next.function_name.clone());
            self.stack_depth = next.stack_depth;
            self.called_by = next.caller_name.clone();
        }

        self.add_log_verbose("Execution continued with edited args".to_string());
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
                "execution_complete" => {
                    self.status = "Completed".to_string();
                    self.add_log("Execution completed".to_string());
                }
                "error" => {
                    if let Some(error_msg) = message.get("message").and_then(|v| v.as_str()) {
                        self.add_log(format!("ERROR: {}", error_msg));
                    } else if let Some(error_msg) = message.get("log").and_then(|v| v.as_str()) {
                        self.add_log(format!("ERROR: {}", error_msg));
                    }
                }
                _ => {
                    self.add_log_verbose(format!("Status: {}", msg_type));
                }
            }
        }

        if self.app_loading {
            if let Some(log_msg) = message.get("log").and_then(|v| v.as_str()) {
                if log_msg.contains("Application loaded and ready") {
                    self.app_loading = false;
                    self.status = "Running".to_string();
                }
            }
        }
    }

    pub fn set_promoted_functions(&mut self, promoted: std::collections::HashSet<String>) {
        self.promoted_functions = promoted;
    }

    fn handle_checkpoint_message(&mut self, message: serde_json::Value) {
        let msg_type = message
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("checkpoint");

        match msg_type {
            "checkpoint" => self.handle_checkpoint_pause(message),
            "checkpoint_complete" => self.handle_checkpoint_complete(message),
            other => {
                self.add_log(format!("Unknown checkpoint message type: {}", other));
            }
        }
    }

    fn handle_checkpoint_pause(&mut self, message: serde_json::Value) {
        let checkpoint_id = message.get("id").and_then(|v| v.as_u64()).unwrap_or(0);

        if let Some(payload) = message.get("payload") {
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

            let stack_depth = payload
                .get("stackDepth")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;

            let caller_name = payload
                .get("callerName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            self.add_log(format!("Checkpoint: {}", func_name));

            self.pending_checkpoints.push(CheckpointData {
                id: checkpoint_id,
                function_name: func_name.to_string(),
                args,
                context,
                stack_depth,
                caller_name: caller_name.clone(),
            });

            if self.pending_checkpoints.len() == 1 {
                self.current_function = Some(func_name.to_string());
                self.paused = true;
                self.status = "Paused".to_string();
                self.stack_depth = stack_depth;
                self.called_by = caller_name;
            }

            self.timeline_functions.push(TimelineFunction {
                name: func_name.to_string(),
                status: FunctionStatus::Pending,
                checkpoint_id,
                duration: None,
                return_preview: None,
                return_type: None,
                return_value: None,
                error_preview: None,
                is_promoted: self.promoted_functions.contains(func_name),
                stack_depth,
            });

            self.scroll_timeline_to_bottom();
        }
    }

    fn handle_checkpoint_complete(&mut self, message: serde_json::Value) {
        let checkpoint_id = message.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let payload = match message.get("payload") {
            Some(p) => p,
            None => return,
        };

        let function_name = payload
            .get("functionName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let status_str = payload
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("ok");
        let duration_ms = payload
            .get("durationMs")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let memory_delta = payload
            .get("memoryDeltaBytes")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let return_preview = payload
            .get("returnValue")
            .and_then(|v| v.get("preview"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let return_type = payload
            .get("returnValue")
            .and_then(|v| v.get("type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let return_value = payload.get("returnValue").cloned();

        let error_preview = payload
            .get("error")
            .and_then(|v| v.get("preview"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Aggregate profile stats (skip skipped functions)
        let was_skipped = self
            .timeline_functions
            .iter()
            .find(|f| f.checkpoint_id == checkpoint_id)
            .map(|f| matches!(f.status, FunctionStatus::Skipped))
            .unwrap_or(false);

        if !was_skipped {
            let stats = self.profile_stats.entry(function_name.clone()).or_default();
            stats.record(duration_ms, memory_delta);
        }

        let total_real_ms: f64 = self.profile_stats.values().map(|s| s.total_ms).sum();
        self.execution_time = format_duration(total_real_ms);

        if let Some(func) = self
            .timeline_functions
            .iter_mut()
            .find(|f| f.checkpoint_id == checkpoint_id)
        {
            func.duration = Some(format_duration(duration_ms));
            func.return_preview = return_preview.clone();
            func.return_type = return_type;
            func.return_value = return_value;
            func.error_preview = error_preview.clone();

            match status_str {
                "ok" => {
                    if !matches!(func.status, FunctionStatus::Skipped) {
                        func.status = FunctionStatus::Completed;
                    }
                }
                "error" => {
                    func.status = FunctionStatus::Failed;
                }
                _ => {}
            }
        }

        match status_str {
            "ok" => {
                if let Some(preview) = return_preview {
                    self.add_log(format!(
                        "← {} returned {}",
                        function_name,
                        truncate(&preview, 60)
                    ));
                }
            }
            "error" => {
                if let Some(err) = error_preview {
                    self.add_log(format!("✗ {} threw {}", function_name, truncate(&err, 60)));
                }
            }
            _ => {}
        }
    }

    pub fn continue_execution(&mut self) {
        if !self.pending_checkpoints.is_empty() {
            let checkpoint = self.pending_checkpoints.remove(0);

            self.add_log_verbose(format!(
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

            self.add_log_verbose("Continue response sent successfully".to_string());

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
                self.stack_depth = 0;
                self.called_by = None;
            } else {
                let next = &self.pending_checkpoints[0];
                self.current_function = Some(next.function_name.clone());
                self.stack_depth = next.stack_depth;
                self.called_by = next.caller_name.clone();
            }

            self.add_log_verbose("Execution continued".to_string());
        }
    }

    pub fn skip_function(&mut self, return_value: serde_json::Value) {
        // TODO: Validate return value type against function's declared return type
        // when type metadata becomes available from the transformer. Currently
        // any JSON value is accepted, which can lead to runtime type mismatches
        // downstream of the skipped call.
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

            self.add_log(format!("⊘ {} skipped", checkpoint.function_name));

            if self.pending_checkpoints.is_empty() {
                self.paused = false;
                self.status = "Running".to_string();
                self.current_function = None;
                self.stack_depth = 0;
                self.called_by = None;
            } else {
                let next = &self.pending_checkpoints[0];
                self.current_function = Some(next.function_name.clone());
                self.stack_depth = next.stack_depth;
                self.called_by = next.caller_name.clone();
            }
        }
    }

    pub fn set_file(
        &mut self,
        transformed_file: String,
        original_file: String,
        runtime_dir: std::path::PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.file_path = Some(original_file);
        self.transformed_path = Some(transformed_file.clone());

        match IpcBridge::spawn_runtime(&transformed_file, &runtime_dir) {
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

    pub fn add_log_verbose(&mut self, message: String) {
        if self.verbose {
            self.add_log(message);
        }
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
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(frame, main_layout[0], state);
    draw_current_checkpoint(frame, main_layout[1], state);
    draw_timeline(frame, main_layout[2], state);
    draw_status_logs(frame, main_layout[3], state);
    draw_quick_actions(frame, main_layout[4]);

    if state.show_detail_popup {
        draw_detail_popup(frame, area, state);
    }

    if state.show_skip_popup {
        draw_skip_popup(frame, area, state);
    }

    if state.show_edit_popup {
        draw_edit_popup(frame, area, state);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, state: &mut DashboardState) {
    let file_display = state
        .file_path
        .as_ref()
        .map(|f| f.trim_matches('"'))
        .unwrap_or("No file");

    let block_style = if state.paused {
        Style::default().fg(Color::Yellow)
    } else if state.status == "Completed" {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            "[ ".into(),
            "Dashboard".blue().bold(),
            " ]".into(),
        ]))
        .style(block_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let row1 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    frame.render_widget(Paragraph::new(format!("File: {}", file_display)), row1[0]);
    frame.render_widget(
        Paragraph::new(format!("Runtime: {}", state.runtime)).alignment(Alignment::Left),
        row1[1],
    );

    let row2 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let status_display = if state.paused {
        format!("⏸ Paused ({} queued)", state.pending_checkpoints.len())
    } else {
        state.status.clone()
    };

    let status_text = format!("Status: {}", status_display);
    let status_text_width = status_text.chars().count() as u16;

    let status_section = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(status_text_width + 1),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(row2[0]);

    frame.render_widget(Paragraph::new(status_text), status_section[0]);

    let show_throbber = matches!(state.status.as_str(), "Starting" | "Loading" | "Running");

    if show_throbber {
        let throbber = Throbber::default()
            .style(Style::default().fg(Color::Cyan))
            .throbber_set(BRAILLE_SIX);
        frame.render_stateful_widget(throbber, status_section[1], &mut state.throbber_state);
    }

    frame.render_widget(
        Paragraph::new(format!("Execution: {}", state.execution_time)),
        row2[1],
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
    let visible_height = area.height.saturating_sub(2) as usize;
    state.timeline_visible_height = visible_height;

    let border_style = if state.focused_panel == FocusedPanel::Timeline {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Line::from(vec![
            "[ ".into(),
            "Execution Timeline".yellow().bold(),
            format!(
                " ({}/{}) ",
                (state.timeline_scroll_offset + visible_height).min(state.timeline_functions.len()),
                state.timeline_functions.len()
            )
            .into(),
            " ]".into(),
        ]));

    if state.timeline_functions.is_empty() {
        frame.render_widget(
            Paragraph::new("No functions executed yet").block(block),
            area,
        );
        return;
    }

    let total = state.timeline_functions.len();
    let start_idx = state.timeline_scroll_offset;
    let end_idx = (start_idx + visible_height).min(total);

    let timeline_items: Vec<ListItem> = state.timeline_functions[start_idx..end_idx]
        .iter()
        .map(|func| {
            let status_symbol = match func.status {
                FunctionStatus::Completed => "✓",
                FunctionStatus::Failed => "✗",
                FunctionStatus::Skipped => "⊘",
                FunctionStatus::Current => "►",
                FunctionStatus::Pending => "○",
            };

            let style = match func.status {
                FunctionStatus::Completed => Style::default().fg(Color::Green),
                FunctionStatus::Failed => Style::default().fg(Color::Red),
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

            let return_str = match func.status {
                FunctionStatus::Completed => func
                    .return_preview
                    .as_ref()
                    .map(|p| format!(" → {}", truncate(p, 50)))
                    .unwrap_or_default(),
                FunctionStatus::Failed => func
                    .error_preview
                    .as_ref()
                    .map(|p| format!(" ⚠ {}", truncate(p, 50)))
                    .unwrap_or_default(),
                _ => String::new(),
            };

            let promoted_badge = if func.is_promoted { " (async*)" } else { "" };

            let indent = "  ".repeat(func.stack_depth as usize);
            let display_name = format!("{}{}", indent, func.name);

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", status_symbol), style),
                Span::styled(display_name, style),
                Span::styled(
                    promoted_badge,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::DIM),
                ),
                Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
                Span::styled(
                    return_str,
                    Style::default().fg(if matches!(func.status, FunctionStatus::Failed) {
                        Color::Red
                    } else {
                        Color::Cyan
                    }),
                ),
            ]))
        })
        .collect();

    let list = List::new(timeline_items).block(block);
    frame.render_widget(list, area);

    // Scrollbar
    if total > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(total.saturating_sub(visible_height))
            .position(state.timeline_scroll_offset);

        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
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

    let border_style = if state.focused_panel == FocusedPanel::Logs {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list = List::new(log_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Line::from(vec![
                "[ ".into(),
                "Status & Logs".cyan().bold(),
                format!(" ({}/{}) ", end_idx, total_logs).into(),
                " ]".into(),
            ])),
    );

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
        "[C]ontinue  [S]kip  [E]dit  [D]etail  [Tab] Switch  [↑↓] Scroll  [P]rofiler  [Q]uit";

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

fn draw_detail_popup(frame: &mut Frame, area: Rect, state: &mut DashboardState) {
    let checkpoint = match state.pending_checkpoints.first() {
        Some(c) => c,
        None => return,
    };

    let popup_area = centered_rect(40, 40, area);
    frame.render_widget(Clear, popup_area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Line::from(vec![
            "[ ".into(),
            "Checkpoint Detail".cyan().bold(),
            format!(" — {}() ", checkpoint.function_name).into(),
            " ]".into(),
        ]))
        .title_bottom(
            Line::from(vec![" [↑↓] Scroll   [D/Esc] Close ".dim()]).alignment(Alignment::Center),
        );

    let inner = outer_block.inner(popup_area);
    frame.render_widget(outer_block, popup_area);

    let args_pretty = serde_json::to_string_pretty(&checkpoint.args)
        .unwrap_or_else(|_| "<failed to serialize args>".to_string());

    let context_str = checkpoint
        .context
        .as_ref()
        .map(|c| format!("\n\nContext: {}", c))
        .unwrap_or_default();

    let body_text = format!(
        "ID: {}\n\nArgs:\n{}{}",
        checkpoint.id, args_pretty, context_str
    );
    let total_lines = body_text.lines().count();

    state.detail_content_lines = total_lines;
    state.detail_visible_height = inner.height;

    let paragraph = Paragraph::new(body_text)
        .wrap(Wrap { trim: false })
        .scroll((state.detail_scroll_offset, 0));

    frame.render_widget(paragraph, inner);

    if total_lines > inner.height as usize {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state =
            ScrollbarState::new(total_lines.saturating_sub(inner.height as usize))
                .position(state.detail_scroll_offset as usize);

        frame.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }
}

fn draw_skip_popup(frame: &mut Frame, area: Rect, state: &mut DashboardState) {
    let checkpoint = match state.pending_checkpoints.first() {
        Some(c) => c,
        None => return,
    };

    let popup_area = centered_rect(60, 60, area);
    frame.render_widget(Clear, popup_area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Line::from(vec![
            "[ ".into(),
            "Skip Function".yellow().bold(),
            format!(" — {}() ", checkpoint.function_name).into(),
            " ]".into(),
        ]))
        .title_bottom(
            Line::from(vec![" [Ctrl+S] Skip   [Esc] Cancel ".dim()]).alignment(Alignment::Center),
        );

    let inner = outer_block.inner(popup_area);
    frame.render_widget(outer_block, popup_area);

    // Layout: header (info) + textarea + status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // info
            Constraint::Min(3),    // textarea
            Constraint::Length(2), // validation status
        ])
        .split(inner);

    // Info: args preview
    let args_preview =
        serde_json::to_string(&checkpoint.args).unwrap_or_else(|_| "<unserializable>".to_string());
    let args_display = truncate(&args_preview, (chunks[0].width as usize).saturating_sub(8));
    let info_text = format!("Args: {}\nReturn value (JSON):", args_display);
    frame.render_widget(
        Paragraph::new(info_text).style(Style::default().fg(Color::Gray)),
        chunks[0],
    );

    // Textarea
    state.skip_textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    state.skip_textarea.set_placeholder_text("(empty = null)");
    frame.render_widget(&state.skip_textarea, chunks[1]);

    // Validation status
    let raw = state.skip_textarea_value();
    let trimmed = raw.trim();
    let (status_text, status_style) = if trimmed.is_empty() {
        (
            "→ Will skip with: null".to_string(),
            Style::default().fg(Color::DarkGray),
        )
    } else {
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(_) => (
                "✓ Valid JSON".to_string(),
                Style::default().fg(Color::Green),
            ),
            Err(e) => (
                format!("✗ {}", truncate(&e.to_string(), 80)),
                Style::default().fg(Color::Red),
            ),
        }
    };

    frame.render_widget(Paragraph::new(status_text).style(status_style), chunks[2]);
}

fn draw_edit_popup(frame: &mut Frame, area: Rect, state: &mut DashboardState) {
    let checkpoint = match state.pending_checkpoints.first() {
        Some(c) => c,
        None => return,
    };

    let popup_area = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup_area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Line::from(vec![
            "[ ".into(),
            "Edit Args".magenta().bold(),
            format!(" — {}() ", checkpoint.function_name).into(),
            " ]".into(),
        ]))
        .title_bottom(
            Line::from(vec![" [Ctrl+S] Run with new args   [Esc] Cancel ".dim()])
                .alignment(Alignment::Center),
        );

    let inner = outer_block.inner(popup_area);
    frame.render_widget(outer_block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(inner);

    let info_text = "Args (JSON array):";
    frame.render_widget(
        Paragraph::new(info_text).style(Style::default().fg(Color::Gray)),
        chunks[0],
    );

    state.edit_textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    state.edit_textarea.set_placeholder_text("[]");
    frame.render_widget(&state.edit_textarea, chunks[1]);

    let raw = state.edit_textarea_value();
    let trimmed = raw.trim();
    let (status_text, status_style) = if trimmed.is_empty() {
        (
            "✗ Args cannot be empty".to_string(),
            Style::default().fg(Color::Red),
        )
    } else {
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(serde_json::Value::Array(arr)) => (
                format!("✓ Valid JSON array ({} args)", arr.len()),
                Style::default().fg(Color::Green),
            ),
            Ok(_) => (
                "✗ Args must be a JSON array".to_string(),
                Style::default().fg(Color::Red),
            ),
            Err(e) => (
                format!("✗ {}", truncate(&e.to_string(), 80)),
                Style::default().fg(Color::Red),
            ),
        }
    };

    frame.render_widget(Paragraph::new(status_text).style(status_style), chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_duration(ms: f64) -> String {
    if ms < 1.0 {
        format!("{:.2}ms", ms)
    } else if ms < 1000.0 {
        format!("{:.1}ms", ms)
    } else {
        format!("{:.2}s", ms / 1000.0)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{}…", truncated)
    }
}

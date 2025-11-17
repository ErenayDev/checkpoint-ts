use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use throbber_widgets_tui::{BRAILLE_SIX, Throbber, ThrobberState};

#[derive(Debug)]
pub struct DashboardState {
    pub file_path: Option<String>,
    pub runtime: String,
    pub execution_time: String,
    pub status: String,
    pub current_function: Option<String>,
    pub current_line: Option<u32>,
    pub called_by: Option<String>,
    pub stack_depth: u32,
    pub timeline_functions: Vec<TimelineFunction>,
    pub logs: Vec<String>,
    pub throbber_state: ThrobberState,
}

#[derive(Clone, Debug)]
pub struct TimelineFunction {
    pub name: String,
    pub status: FunctionStatus,
    pub duration: Option<String>,
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
            runtime: "Bun v1.3.2".to_string(),
            execution_time: "0s".to_string(),
            status: "Ready".to_string(),
            current_function: None,
            current_line: None,
            called_by: None,
            stack_depth: 0,
            throbber_state: ThrobberState::default(),
            timeline_functions: vec![
                TimelineFunction {
                    name: "initApp".to_string(),
                    status: FunctionStatus::Completed,
                    duration: Some("2ms".to_string()),
                },
                TimelineFunction {
                    name: "loadConfig".to_string(),
                    status: FunctionStatus::Completed,
                    duration: Some("15ms".to_string()),
                },
                TimelineFunction {
                    name: "connectDB".to_string(),
                    status: FunctionStatus::Completed,
                    duration: Some("120ms".to_string()),
                },
                TimelineFunction {
                    name: "fetchUser".to_string(),
                    status: FunctionStatus::Completed,
                    duration: Some("45ms".to_string()),
                },
                TimelineFunction {
                    name: "calculateTax".to_string(),
                    status: FunctionStatus::Current,
                    duration: None,
                },
            ],
            logs: vec![
                "System initialized".to_string(),
                "Ready to start debugging...".to_string(),
                "Waiting for user input...".to_string(),
            ],
        }
    }
}

impl DashboardState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_file(&mut self, file: String) {
        self.file_path = Some(file);
        self.add_log("File loaded successfully".to_string());
    }

    pub fn tick_throbber(&mut self) {
        self.throbber_state.calc_next();
    }

    pub fn set_current_checkpoint(
        &mut self,
        function: String,
        line: u32,
        called_by: Option<String>,
    ) {
        self.current_function = Some(function.clone());
        self.current_line = Some(line);
        self.called_by = called_by;
        self.add_log(format!("Paused at {}() line {}", function, line));
    }

    pub fn add_timeline_function(&mut self, function: TimelineFunction) {
        self.timeline_functions.push(function);
    }

    pub fn update_function_status(
        &mut self,
        name: &str,
        status: FunctionStatus,
        duration: Option<String>,
    ) {
        if let Some(func) = self.timeline_functions.iter_mut().find(|f| f.name == name) {
            func.status = status;
            func.duration = duration;
        }
    }

    pub fn add_log(&mut self, message: String) {
        self.logs.push(format!(
            "[{}] {}",
            chrono::Local::now().format("%H:%M:%S"),
            message
        ));
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn update_execution_time(&mut self, time: String) {
        self.execution_time = time;
    }

    pub fn set_status(&mut self, status: String) {
        self.status = status.clone();
        self.add_log(format!("Status: {}", status));
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &mut DashboardState) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header
            Constraint::Length(4), // Current Checkpoint
            Constraint::Length(4), // Timeline
            Constraint::Min(4),    // Status/Logs
            Constraint::Length(3), // Quick Actions
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

    let line2_left = format!("Status: {}", state.status);
    let line2_right = format!("Execution: {}", state.execution_time);
    let line2 = format!("{:<width$}{}", line2_left, line2_right, width = half_width);

    let combined_text = format!("{}\n{}", line1, line2);

    frame.render_widget(
        Paragraph::new(combined_text).block(Block::default().borders(Borders::ALL).title(
            Line::from(vec!["[ ".into(), "Dashboard".blue().bold(), " ]".into()]),
        )),
        area,
    );
}

fn draw_current_checkpoint(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let total_width = area.width.saturating_sub(2) as usize;
    let half_width = total_width / 2;

    let line1_left = match &state.current_function {
        Some(func) => format!("Function: {}()", func),
        None => "Function: None".to_string(),
    };
    let line1_right = match state.current_line {
        Some(line) => format!("Line: {}", line),
        None => "Line: -".to_string(),
    };
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

    let display_functions: Vec<_> = functions.iter().rev().take(5).rev().collect();
    let constraints: Vec<Constraint> = (0..display_functions.len())
        .map(|_| Constraint::Percentage(100 / display_functions.len() as u16))
        .collect();

    let timeline_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, func) in display_functions.iter().enumerate() {
        let borders = if i == 0 {
            Borders::ALL
        } else {
            Borders::TOP | Borders::RIGHT | Borders::BOTTOM
        };

        let block = if i == 0 {
            Block::default().borders(borders).title(Line::from(vec![
                "[ ".into(),
                format!("Execution Timeline (Last {})", display_functions.len())
                    .yellow()
                    .bold(),
                " ]".into(),
            ]))
        } else {
            Block::default().borders(borders)
        };

        match func.status {
            FunctionStatus::Current => {
                let inner_area = block.inner(timeline_layout[i]);
                frame.render_widget(block, timeline_layout[i]);
                let label = format!("{}()", func.name);
                let throbber = Throbber::default()
                    .throbber_set(BRAILLE_SIX)
                    .label(&label)
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow));

                frame.render_stateful_widget(throbber, inner_area, &mut state.throbber_state);
            }
            _ => {
                let status_symbol = match func.status {
                    FunctionStatus::Completed => "✓",
                    FunctionStatus::Skipped => "⏭",
                    FunctionStatus::Pending => "○",
                    FunctionStatus::Current => unreachable!(),
                };

                let default_duration = "?ms".to_string();
                let duration = func.duration.as_ref().unwrap_or(&default_duration);
                let content = format!("{} {}()\n{}", status_symbol, func.name, duration);

                frame.render_widget(
                    Paragraph::new(content)
                        .block(block)
                        .alignment(ratatui::layout::Alignment::Center),
                    timeline_layout[i],
                );
            }
        }
    }
}

fn draw_status_logs(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let display_logs: Vec<_> = state.logs.iter().rev().take(10).rev().collect();
    let logs_text = display_logs
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    frame.render_widget(
        Paragraph::new(logs_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        "[ ".into(),
                        "Status & Logs".cyan().bold(),
                        " ]".into(),
                    ])),
            )
            .scroll((0, 0)),
        area,
    );
}

fn draw_quick_actions(frame: &mut Frame, area: Rect) {
    let actions_text = "[C] Continue    [S] Skip Function    [E] Edit Variables    [P] Profile    [V] View Stack    [H] History    [Q] Quit";
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

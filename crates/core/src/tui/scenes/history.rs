use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ExecutionHistoryState {
    pub total_functions: u32,
    pub skipped: u32,
    pub executed: u32,
    pub total_time: f32,
    pub function_calls: Vec<FunctionCall>,
    pub selected_index: usize,
}

#[derive(Clone, Debug)]
pub struct FunctionCall {
    pub time: String,
    pub function: String,
    pub status: CallStatus,
    pub duration: String,
    pub return_value: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum CallStatus {
    Exec,
    Skip,
    Pause,
}

impl Default for ExecutionHistoryState {
    fn default() -> Self {
        Self {
            total_functions: 23,
            skipped: 5,
            executed: 18,
            total_time: 1.2,
            function_calls: vec![
                FunctionCall {
                    time: "0.0ms".to_string(),
                    function: "initApp()".to_string(),
                    status: CallStatus::Exec,
                    duration: "2ms".to_string(),
                    return_value: "undefined".to_string(),
                },
                FunctionCall {
                    time: "2.1ms".to_string(),
                    function: "loadConfig()".to_string(),
                    status: CallStatus::Exec,
                    duration: "15ms".to_string(),
                    return_value: "{...}".to_string(),
                },
                FunctionCall {
                    time: "17.3ms".to_string(),
                    function: "connectDB()".to_string(),
                    status: CallStatus::Exec,
                    duration: "120ms".to_string(),
                    return_value: "Connection".to_string(),
                },
                FunctionCall {
                    time: "137.8ms".to_string(),
                    function: "fetchUser()".to_string(),
                    status: CallStatus::Exec,
                    duration: "45ms".to_string(),
                    return_value: "User{id:123}".to_string(),
                },
                FunctionCall {
                    time: "183.2ms".to_string(),
                    function: "validateUser()".to_string(),
                    status: CallStatus::Skip,
                    duration: "0ms".to_string(),
                    return_value: "true (injected)".to_string(),
                },
                FunctionCall {
                    time: "183.3ms".to_string(),
                    function: "loadPermissions()".to_string(),
                    status: CallStatus::Skip,
                    duration: "0ms".to_string(),
                    return_value: "['read','write']".to_string(),
                },
                FunctionCall {
                    time: "183.4ms".to_string(),
                    function: "processOrder()".to_string(),
                    status: CallStatus::Exec,
                    duration: "25ms".to_string(),
                    return_value: "order{...}".to_string(),
                },
                FunctionCall {
                    time: "208.7ms".to_string(),
                    function: "calculateTax()".to_string(),
                    status: CallStatus::Pause,
                    duration: "?".to_string(),
                    return_value: "?".to_string(),
                },
            ],
            selected_index: 0,
        }
    }
}

impl ExecutionHistoryState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &ExecutionHistoryState) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(5),
        ])
        .split(area);

    draw_summary(frame, main_layout[0], state);
    draw_function_timeline(frame, main_layout[1], state);
    draw_navigation(frame, main_layout[2]);
}

fn draw_summary(frame: &mut Frame, area: Rect, state: &ExecutionHistoryState) {
    let summary_text = format!(
        "Total Functions: {}    Skipped: {}    Executed: {}    Total Time: {}s",
        state.total_functions, state.skipped, state.executed, state.total_time
    );
    frame.render_widget(
        Paragraph::new(summary_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Execution History".blue().bold()),
        ),
        area,
    );
}

fn draw_function_timeline(frame: &mut Frame, area: Rect, state: &ExecutionHistoryState) {
    let header = Row::new(vec![
        Cell::from("Time").style(ratatui::style::Style::default().bold()),
        Cell::from("Function").style(ratatui::style::Style::default().bold()),
        Cell::from("Status").style(ratatui::style::Style::default().bold()),
        Cell::from("Duration").style(ratatui::style::Style::default().bold()),
        Cell::from("Return Value").style(ratatui::style::Style::default().bold()),
    ]);

    let rows: Vec<Row> = state
        .function_calls
        .iter()
        .map(|call| {
            let status_symbol = match call.status {
                CallStatus::Exec => "✓ Exec",
                CallStatus::Skip => "⊘ Skip",
                CallStatus::Pause => "⏸ Pause",
            };
            Row::new(vec![
                Cell::from(call.time.clone()),
                Cell::from(call.function.clone()),
                Cell::from(status_symbol),
                Cell::from(call.duration.clone()),
                Cell::from(call.return_value.clone()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Percentage(25),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Percentage(35),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Function Call Timeline".yellow().bold()),
    );

    frame.render_widget(table, area);
}

fn draw_navigation(frame: &mut Frame, area: Rect) {
    let navigation_text = "[↑↓] Navigate    [Enter] Go to Checkpoint    [R] Replay from Here    [D] Show Details    [S] Save History    [Esc] Back to Dashboard";
    frame.render_widget(
        Paragraph::new(navigation_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Navigation".green().bold()),
        ),
        area,
    );
}

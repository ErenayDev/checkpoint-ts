use crate::utils::num_utils;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

#[derive(Debug, Clone)]
pub struct PerformanceProfileState {
    pub total_execution: Option<f64>,
    pub function_count: Option<u32>,
    pub average: Option<u32>,
    pub functions: Vec<Function>,
    pub memory_usage: MemoryUsage,
}

#[derive(Clone, Debug)]
pub struct MemoryUsage {
    pub peak_ram: f64,
    pub current_ram: f64,
    pub objects_created: i32,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub ms: u32,
    pub call_count: u32,
    pub avg: u32,
}

impl Default for PerformanceProfileState {
    fn default() -> Self {
        Self {
            total_execution: Some(1.2),
            function_count: Some(23),
            average: Some(52),
            functions: vec![
                Function {
                    name: "connectDB".to_string(),
                    ms: 120,
                    call_count: 12,
                    avg: 23,
                },
                Function {
                    name: "fetchUser".to_string(),
                    ms: 45,
                    call_count: 10,
                    avg: 5,
                },
                Function {
                    name: "processOrder".to_string(),
                    ms: 25,
                    call_count: 8,
                    avg: 1,
                },
                Function {
                    name: "loadConfig".to_string(),
                    ms: 15,
                    call_count: 6,
                    avg: 1,
                },
                Function {
                    name: "initApp".to_string(),
                    ms: 2,
                    call_count: 1,
                    avg: 1,
                },
                Function {
                    name: "calculateTax".to_string(),
                    ms: 23,
                    call_count: 12,
                    avg: 23,
                },
                Function {
                    name: "validateInput".to_string(),
                    ms: 5,
                    call_count: 10,
                    avg: 5,
                },
                Function {
                    name: "formatCurrency".to_string(),
                    ms: 1,
                    call_count: 8,
                    avg: 1,
                },
                Function {
                    name: "logEvent".to_string(),
                    ms: 1,
                    call_count: 6,
                    avg: 1,
                },
            ],
            memory_usage: MemoryUsage {
                peak_ram: 45.2,
                current_ram: 32.1,
                objects_created: 1247,
            },
        }
    }
}

impl PerformanceProfileState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save_report(&mut self) {}

    pub fn export_csv(&mut self) {}

    pub fn _filter_functions(&mut self, _functions: Vec<Function>) {}
}

pub fn draw(frame: &mut Frame, area: Rect, state: &PerformanceProfileState) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Min(6),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    draw_performance_stats(frame, main_layout[0], state);
    draw_slowest_functions(frame, main_layout[1], state);
    draw_call_frequency(frame, main_layout[2], state);
    draw_memory_usage(frame, main_layout[3], state);
    draw_actions(frame, main_layout[4]);
}

fn draw_performance_stats(frame: &mut Frame, area: Rect, state: &PerformanceProfileState) {
    let total_execution = state.total_execution.unwrap_or(0.0);
    let function_count = state.function_count.unwrap_or(0);
    let average = state.average.unwrap_or(0);

    let stats_text = format!(
        "Total Execution: {:.1}s         Functions: {}         Average: {}ms per function",
        total_execution, function_count, average
    );

    frame.render_widget(
        Paragraph::new(stats_text).block(Block::default().borders(Borders::ALL).title(Line::from(
            vec![
                "[ ".into(),
                "Performance Profile".blue().bold(),
                " ]".into(),
            ],
        ))),
        area,
    );
}

fn draw_slowest_functions(frame: &mut Frame, area: Rect, state: &PerformanceProfileState) {
    let mut functions = state.functions.clone();
    functions.sort_by(|a, b| b.ms.cmp(&a.ms));

    let max_duration = functions.first().map(|f| f.ms).unwrap_or(0);
    let available_width = area.width.saturating_sub(20);

    let function_lines: Vec<Line> = functions
        .iter()
        .take(area.height.saturating_sub(2) as usize)
        .map(|func| {
            let bar_width = if max_duration > 0 {
                ((func.ms as f64 / max_duration as f64) * (available_width as f64 * 0.7)) as u16
            } else {
                0
            };

            let bar = "█".repeat(bar_width as usize);
            let duration_text = format!("{}ms", func.ms);
            let function_name = format!("{}()", func.name);
            let name_width = 18;

            Line::from(vec![
                Span::raw(format!("{:<width$}", function_name, width = name_width)),
                Span::styled(bar, Style::default().fg(Color::Blue)),
                Span::raw(
                    " ".repeat(
                        area.width
                            .saturating_sub(name_width as u16)
                            .saturating_sub(bar_width)
                            .saturating_sub(duration_text.len() as u16)
                            .saturating_sub(5) as usize,
                    ),
                ),
                Span::styled(duration_text, Style::default().fg(Color::Yellow)),
            ])
        })
        .collect();

    let stats_text = Text::from(function_lines);
    frame.render_widget(
        Paragraph::new(stats_text).block(Block::default().borders(Borders::ALL).title(Line::from(
            vec!["[ ".into(), "Slowest Functions".green().bold(), " ]".into()],
        ))),
        area,
    );
}

fn draw_call_frequency(frame: &mut Frame, area: Rect, state: &PerformanceProfileState) {
    let mut functions = state.functions.clone();
    functions.sort_by(|a, b| b.call_count.cmp(&a.call_count));

    let max_calls = functions.first().map(|f| f.call_count).unwrap_or(0);
    let available_width = area.width.saturating_sub(35);

    let function_lines: Vec<Line> = functions
        .iter()
        .take(area.height.saturating_sub(2) as usize)
        .map(|func| {
            let bar_width = if max_calls > 0 {
                ((func.call_count as f64 / max_calls as f64) * (available_width as f64 * 0.6))
                    as u16
            } else {
                0
            };

            let bar = "█".repeat(bar_width as usize);
            let function_name = format!("{}()", func.name);
            let calls_text = format!("{} calls", func.call_count);
            let avg_text = format!("avg: {}ms", func.avg);
            let name_width = 18;
            let calls_text_len = calls_text.len();
            let avg_text_len = avg_text.len();

            Line::from(vec![
                Span::raw(format!("{:<width$}", function_name, width = name_width)),
                Span::styled(bar, Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::raw(calls_text),
                Span::raw(
                    " ".repeat(
                        area.width
                            .saturating_sub(name_width as u16)
                            .saturating_sub(bar_width)
                            .saturating_sub(calls_text_len as u16)
                            .saturating_sub(avg_text_len as u16)
                            .saturating_sub(10) as usize,
                    ),
                ),
                Span::styled(avg_text, Style::default().fg(Color::Yellow)),
            ])
        })
        .collect();

    let stats_text = Text::from(function_lines);
    frame.render_widget(
        Paragraph::new(stats_text).block(Block::default().borders(Borders::ALL).title(Line::from(
            vec!["[ ".into(), "Call Frequency".green().bold(), " ]".into()],
        ))),
        area,
    );
}

fn draw_memory_usage(frame: &mut Frame, area: Rect, state: &PerformanceProfileState) {
    let memory_usage = &state.memory_usage;
    let peak = memory_usage.peak_ram;
    let current = memory_usage.current_ram;
    let objects_created = memory_usage.objects_created;

    let memory_usage_text = format!(
        "Peak: {:.1} MB       Current: {:.1} MB       Objects Created: {}",
        peak,
        current,
        num_utils::format_with_commas(objects_created)
    );

    frame.render_widget(
        Paragraph::new(memory_usage_text).block(Block::default().borders(Borders::ALL).title(
            Line::from(vec![
                "[ ".into(),
                "Memory Usage (Approximate)".blue().bold(),
                " ]".into(),
            ]),
        )),
        area,
    );
}

fn draw_actions(frame: &mut Frame, area: Rect) {
    let actions_text = "[S] Save Report    [E] Export CSV    [F] Filter Functions    [Esc] Back";

    frame.render_widget(
        Paragraph::new(actions_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        "[ ".into(),
                        "Actions".green().bold(),
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

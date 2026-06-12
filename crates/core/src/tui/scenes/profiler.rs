use crate::tui::scenes::dashboard::{DashboardState, ProfileStats};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Total,
    Avg,
    Calls,
    Memory,
    Name,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct PerformanceProfileState {
    pub sort_key: SortKey,
    pub sort_direction: SortDirection,
    pub scroll_offset: usize,
}

impl Default for PerformanceProfileState {
    fn default() -> Self {
        Self {
            sort_key: SortKey::Total,
            sort_direction: SortDirection::Desc,
            scroll_offset: 0,
        }
    }
}

impl PerformanceProfileState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_sort(&mut self, key: SortKey) {
        if self.sort_key == key {
            self.sort_direction = match self.sort_direction {
                SortDirection::Asc => SortDirection::Desc,
                SortDirection::Desc => SortDirection::Asc,
            };
        } else {
            self.sort_key = key;
            self.sort_direction = match key {
                SortKey::Name => SortDirection::Asc,
                _ => SortDirection::Desc,
            };
        }
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self, total: usize, visible: usize) {
        let max = total.saturating_sub(visible);
        if self.scroll_offset < max {
            self.scroll_offset += 1;
        }
    }
}

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    state: &mut PerformanceProfileState,
    dashboard: &DashboardState,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // summary
            Constraint::Min(5),    // table
            Constraint::Length(2), // disclaimer
            Constraint::Length(3), // actions
        ])
        .split(area);

    draw_summary(frame, layout[0], dashboard);
    draw_table(frame, layout[1], state, dashboard);
    draw_disclaimer(frame, layout[2]);
    draw_actions(frame, layout[3], state);
}

fn draw_summary(frame: &mut Frame, area: Rect, dashboard: &DashboardState) {
    let stats = &dashboard.profile_stats;

    let unique_functions = stats.len();
    let total_calls: u32 = stats.values().map(|s| s.call_count).sum();
    let total_time: f64 = stats.values().map(|s| s.total_ms).sum();
    let total_mem: i64 = stats.values().flat_map(|s| s.mem_deltas.iter()).sum();

    let summary_text = format!(
        "Functions: {}     Calls: {}     Total Time: {}     Total Mem Δ: {}",
        unique_functions,
        total_calls,
        format_duration(total_time),
        format_bytes(total_mem),
    );

    frame.render_widget(
        Paragraph::new(summary_text).block(Block::default().borders(Borders::ALL).title(
            Line::from(vec![
                "[ ".into(),
                "Performance Profile".blue().bold(),
                " ]".into(),
            ]),
        )),
        area,
    );
}

fn draw_table(
    frame: &mut Frame,
    area: Rect,
    state: &mut PerformanceProfileState,
    dashboard: &DashboardState,
) {
    let stats = &dashboard.profile_stats;

    if stats.is_empty() {
        frame.render_widget(
            Paragraph::new("No profile data yet — execute some checkpoints first.").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        "[ ".into(),
                        "Functions".green().bold(),
                        " ]".into(),
                    ])),
            ),
            area,
        );
        return;
    }

    let mut entries: Vec<(&String, &ProfileStats)> = stats.iter().collect();
    sort_entries(&mut entries, state.sort_key, state.sort_direction);

    let visible_height = area.height.saturating_sub(3) as usize;
    let total = entries.len();

    if state.scroll_offset >= total {
        state.scroll_offset = total.saturating_sub(visible_height);
    }

    let end = (state.scroll_offset + visible_height).min(total);
    let visible_entries = &entries[state.scroll_offset..end];

    let header_cells = vec![
        format!("Function {}", arrow(state, SortKey::Name)),
        format!("Calls {}", arrow(state, SortKey::Calls)),
        format!("Total {}", arrow(state, SortKey::Total)),
        format!("Avg {}", arrow(state, SortKey::Avg)),
        "Min".to_string(),
        "Max".to_string(),
        "P95".to_string(),
        format!("Mem Δ Avg {}", arrow(state, SortKey::Memory)),
    ];

    let header = Row::new(
        header_cells
            .into_iter()
            .map(|h| {
                Cell::from(h).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect::<Vec<_>>(),
    )
    .height(1);

    let rows: Vec<Row> = visible_entries
        .iter()
        .map(|(name, stats)| {
            let mem_avg = stats.avg_mem_delta();
            let mem_style = mem_color(mem_avg);

            Row::new(vec![
                Cell::from((*name).clone()),
                Cell::from(stats.call_count.to_string()),
                Cell::from(format_duration(stats.total_ms)),
                Cell::from(format_duration(stats.avg_ms())),
                Cell::from(format_duration(stats.min_ms)),
                Cell::from(format_duration(stats.max_ms)),
                Cell::from(format_duration(stats.p95_ms())),
                Cell::from(format_bytes(mem_avg)).style(mem_style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(28),
        Constraint::Length(7),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(13),
    ];

    let title = format!(" Functions ({}/{}) ", end, total);
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(vec![
                    "[".into(),
                    title.green().bold(),
                    "]".into(),
                ])),
        )
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn draw_disclaimer(frame: &mut Frame, area: Rect) {
    let text =
        "Note: Memory deltas are indicative only — affected by GC and concurrent allocations.";
    frame.render_widget(
        Paragraph::new(text)
            .style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )
            .alignment(Alignment::Center),
        area,
    );
}

fn draw_actions(frame: &mut Frame, area: Rect, state: &PerformanceProfileState) {
    let active = format!(
        "Sort: {} {}",
        match state.sort_key {
            SortKey::Total => "Total",
            SortKey::Avg => "Avg",
            SortKey::Calls => "Calls",
            SortKey::Memory => "Memory",
            SortKey::Name => "Name",
        },
        match state.sort_direction {
            SortDirection::Asc => "↑",
            SortDirection::Desc => "↓",
        }
    );

    let actions_text = format!(
        "{}    [t] Total  [a] Avg  [c] Calls  [m] Memory  [n] Name    [↑↓] Scroll    [Esc] Back",
        active
    );

    frame.render_widget(
        Paragraph::new(actions_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        "[ ".into(),
                        "Actions".green().bold(),
                        " ]".into(),
                    ])),
            )
            .alignment(Alignment::Center),
        area,
    );
}

fn arrow(state: &PerformanceProfileState, key: SortKey) -> &'static str {
    if state.sort_key == key {
        match state.sort_direction {
            SortDirection::Asc => "↑",
            SortDirection::Desc => "↓",
        }
    } else {
        ""
    }
}

fn sort_entries(
    entries: &mut Vec<(&String, &ProfileStats)>,
    key: SortKey,
    direction: SortDirection,
) {
    match key {
        SortKey::Total => {
            entries.sort_by(|a, b| {
                a.1.total_ms
                    .partial_cmp(&b.1.total_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortKey::Avg => {
            entries.sort_by(|a, b| {
                a.1.avg_ms()
                    .partial_cmp(&b.1.avg_ms())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortKey::Calls => {
            entries.sort_by_key(|e| e.1.call_count);
        }
        SortKey::Memory => {
            entries.sort_by_key(|e| e.1.avg_mem_delta());
        }
        SortKey::Name => {
            entries.sort_by(|a, b| a.0.cmp(b.0));
        }
    }

    if direction == SortDirection::Desc {
        entries.reverse();
    }
}

fn mem_color(bytes: i64) -> Style {
    if bytes > 1024 {
        Style::default().fg(Color::Green)
    } else if bytes < -1024 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
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

fn format_bytes(bytes: i64) -> String {
    let abs = bytes.unsigned_abs();
    let sign = if bytes < 0 {
        "-"
    } else if bytes > 0 {
        "+"
    } else {
        ""
    };

    if abs < 1024 {
        format!("{}{} B", sign, abs)
    } else if abs < 1024 * 1024 {
        format!("{}{:.1} KB", sign, abs as f64 / 1024.0)
    } else if abs < 1024 * 1024 * 1024 {
        format!("{}{:.1} MB", sign, abs as f64 / (1024.0 * 1024.0))
    } else {
        format!("{}{:.2} GB", sign, abs as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}


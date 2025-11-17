use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

pub fn draw(_frame: &mut Frame, area: Rect) {
    let _main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(area);

    // Sen buraya layout'ları ekleyeceksin
}

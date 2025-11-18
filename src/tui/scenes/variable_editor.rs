use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

#[derive(Debug, Clone)]
pub struct VariableEditorState {
    pub current_function: Option<String>,
    pub current_line: Option<u32>,
    pub function_parameters: Vec<Variable>,
    pub local_variables: Vec<Variable>,
    pub selected_section: SelectedSection,
    pub selected_index: usize,
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub name: String,
    pub var_type: String,
    pub value: String,
    pub is_editing: bool,
}

#[derive(Clone, Debug)]
pub enum SelectedSection {
    Parameters,
    LocalVariables,
    Actions,
}

impl Default for VariableEditorState {
    fn default() -> Self {
        Self {
            current_function: Some("calculateTax".to_string()),
            current_line: Some(45),
            function_parameters: vec![
                Variable {
                    name: "price".to_string(),
                    var_type: "number".to_string(),
                    value: "99.99".to_string(),
                    is_editing: false,
                },
                Variable {
                    name: "quantity".to_string(),
                    var_type: "number".to_string(),
                    value: "3".to_string(),
                    is_editing: false,
                },
                Variable {
                    name: "region".to_string(),
                    var_type: "string".to_string(),
                    value: "\"US\"".to_string(),
                    is_editing: false,
                },
            ],
            local_variables: vec![
                Variable {
                    name: "subtotal".to_string(),
                    var_type: "number".to_string(),
                    value: "299.97".to_string(),
                    is_editing: false,
                },
                Variable {
                    name: "taxRate".to_string(),
                    var_type: "number".to_string(),
                    value: "undefined".to_string(),
                    is_editing: false,
                },
                Variable {
                    name: "finalAmount".to_string(),
                    var_type: "number".to_string(),
                    value: "undefined".to_string(),
                    is_editing: false,
                },
            ],
            selected_section: SelectedSection::Parameters,
            selected_index: 0,
        }
    }
}

impl VariableEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_function(&mut self, function: String, line: u32) {
        self.current_function = Some(function);
        self.current_line = Some(line);
    }

    pub fn add_parameter(&mut self, variable: Variable) {
        self.function_parameters.push(variable);
    }

    pub fn add_local_variable(&mut self, variable: Variable) {
        self.local_variables.push(variable);
    }

    pub fn update_variable_value(
        &mut self,
        section: SelectedSection,
        index: usize,
        new_value: String,
    ) {
        match section {
            SelectedSection::Parameters => {
                if let Some(var) = self.function_parameters.get_mut(index) {
                    var.value = new_value;
                }
            }
            SelectedSection::LocalVariables => {
                if let Some(var) = self.local_variables.get_mut(index) {
                    var.value = new_value;
                }
            }
            _ => {}
        }
    }

    pub fn start_editing(&mut self, section: SelectedSection, index: usize) {
        match section {
            SelectedSection::Parameters => {
                if let Some(var) = self.function_parameters.get_mut(index) {
                    var.is_editing = true;
                }
            }
            SelectedSection::LocalVariables => {
                if let Some(var) = self.local_variables.get_mut(index) {
                    var.is_editing = true;
                }
            }
            _ => {}
        }
    }

    pub fn stop_editing(&mut self) {
        for var in &mut self.function_parameters {
            var.is_editing = false;
        }
        for var in &mut self.local_variables {
            var.is_editing = false;
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &VariableEditorState) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(area);

    draw_function_info(frame, main_layout[0], &state);
    draw_parameters_table(frame, main_layout[1], &state);
    draw_local_variables_table(frame, main_layout[2], &state);
    draw_actions(frame, main_layout[3]);
}

fn draw_function_info(frame: &mut Frame, area: Rect, state: &VariableEditorState) {
    let function_name = state.current_function.as_deref().unwrap_or("Unknown");
    let line_number = state.current_line.unwrap_or(0);

    let info_text = format!(
        "Function: {}()                            Line: {}",
        function_name, line_number
    );

    frame.render_widget(
        Paragraph::new(info_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Variable Editor".blue().bold()),
        ),
        area,
    );
}

fn draw_parameters_table(frame: &mut Frame, area: Rect, state: &VariableEditorState) {
    let header = Row::new(vec![
        Cell::from("Name").style(ratatui::style::Style::default().bold()),
        Cell::from("Type").style(ratatui::style::Style::default().bold()),
        Cell::from("Current Value").style(ratatui::style::Style::default().bold()),
        Cell::from("Actions").style(ratatui::style::Style::default().bold()),
    ]);

    let rows: Vec<Row> = state
        .function_parameters
        .iter()
        .map(|var| {
            let actions = if var.is_editing {
                "[Editing...] [Enter] [Esc]"
            } else {
                "[Edit] [Reset] [Watch]"
            };

            Row::new(vec![
                Cell::from(var.name.clone()),
                Cell::from(var.var_type.clone()),
                Cell::from(var.value.clone()),
                Cell::from(actions),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(15),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Function Parameters".yellow().bold()),
    );

    frame.render_widget(table, area);
}

fn draw_local_variables_table(frame: &mut Frame, area: Rect, state: &VariableEditorState) {
    let header = Row::new(vec![
        Cell::from("Name").style(ratatui::style::Style::default().bold()),
        Cell::from("Type").style(ratatui::style::Style::default().bold()),
        Cell::from("Current Value").style(ratatui::style::Style::default().bold()),
        Cell::from("Actions").style(ratatui::style::Style::default().bold()),
    ]);

    let rows: Vec<Row> = state
        .local_variables
        .iter()
        .map(|var| {
            let actions = if var.is_editing {
                "[Editing...] [Enter] [Esc]"
            } else {
                "[Edit] [Reset] [Watch]"
            };

            Row::new(vec![
                Cell::from(var.name.clone()),
                Cell::from(var.var_type.clone()),
                Cell::from(var.value.clone()),
                Cell::from(actions),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(15),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Local Variables".cyan().bold()),
    );

    frame.render_widget(table, area);
}

fn draw_actions(frame: &mut Frame, area: Rect) {
    let actions_text = "[Enter] Apply Changes    [Esc] Cancel    [Tab] Next Field    [R] Reset All";

    frame.render_widget(
        Paragraph::new(actions_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Actions".green().bold()),
        ),
        area,
    );
}

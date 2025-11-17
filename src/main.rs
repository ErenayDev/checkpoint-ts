use color_eyre::Result;
mod analyzer;
mod cli;
mod instrumenter;
mod runtime;
mod state;
mod tui;
mod utils;
use tui::TuiApp;
use utils::entry_finder;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let entry_file = entry_finder::find_entry_file();

    if entry_file.is_none() {
        println!("No entry file found. Usage: `checkpoint <file>`");
        std::process::exit(1);
    }

    let terminal = ratatui::init();
    let mut app = TuiApp::new(entry_file);
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}

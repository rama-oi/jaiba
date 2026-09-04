mod app;
mod clipboard;
mod config;
mod db;
mod export;
mod import;
mod input;
mod theme;
mod ui;
mod util;

use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};

fn main() -> io::Result<()> {
    let slim_mode = handle_cli_flags();

    let mut terminal = ratatui::init();
    execute!(io::stdout(), EnableMouseCapture)?;

    let result = app::run(&mut terminal, slim_mode);

    execute!(io::stdout(), DisableMouseCapture)?;
    ratatui::restore();

    result
}

fn handle_cli_flags() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args
        .iter()
        .any(|a| a == "--version" || a == "-V" || a == "-v")
    {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--help" || a == "-H" || a == "-h") {
        println!(
            "{} {}\n{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_DESCRIPTION")
        );
        std::process::exit(0);
    }

    args.iter().any(|a| a == "--slim")
}

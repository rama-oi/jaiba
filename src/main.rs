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

fn main() -> io::Result<()> {
    if handle_cli_flags() {
        return Ok(());
    }

    let mut terminal = ratatui::init();

    app::run(&mut terminal)?;

    ratatui::restore();

    Ok(())
}

fn handle_cli_flags() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args
        .iter()
        .any(|a| a == "--version" || a == "-V" || a == "-v")
    {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return true;
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("A terminal password manager built on the KeePass (.kdbx) file format");
        return true;
    }

    false
}

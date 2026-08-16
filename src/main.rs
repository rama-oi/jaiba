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

    if args.iter().any(|a| a == "--version" || a == "-V" || a == "-v") {
        println!("jaiba {}", env!("CARGO_PKG_VERSION"));
        return true;
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("jaiba {}\n\nA terminal password manager built on the KeePass (.kdbx) file format.\n\nUSAGE:\n    jaiba [OPTIONS]\n\nOPTIONS:\n    -V, --version    Print version information\n    -h, --help       Print this help message", env!("CARGO_PKG_VERSION"));
        return true;
    }

    false
}

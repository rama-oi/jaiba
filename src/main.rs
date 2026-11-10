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
use std::path::PathBuf;

use clap::{ArgAction, Parser};

use crate::util::expand_tilde;

#[derive(Debug, Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    disable_help_flag = true,
    disable_version_flag = true
)]
struct Cli {
    /// Open PATH as the KeePass vault for this session.
    #[arg(long, value_name = "PATH", value_parser = parse_vault_path)]
    vault: Option<PathBuf>,

    /// Open an existing .kdbx vault for this session.
    #[arg(value_name = "VAULT", value_parser = parse_existing_vault_path)]
    positional_vault: Option<PathBuf>,

    /// Use the slim interface.
    #[arg(long)]
    slim: bool,

    /// Print version information.
    #[arg(
        short = 'v',
        visible_short_alias = 'V',
        long = "version",
        action = ArgAction::Version
    )]
    version: Option<bool>,

    /// Print help information.
    #[arg(
        short = 'h',
        visible_short_alias = 'H',
        long = "help",
        action = ArgAction::Help
    )]
    help: Option<bool>,
}

struct CliOptions {
    slim_mode: bool,
    vault_path: Option<PathBuf>,
}

fn main() -> io::Result<()> {
    let options = match parse_cli_args(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    let mut terminal = ratatui::init();

    app::run(&mut terminal, options.slim_mode, options.vault_path)?;

    ratatui::restore();

    Ok(())
}

fn parse_cli_args(args: impl IntoIterator<Item = String>) -> Result<CliOptions, String> {
    let args = std::iter::once(env!("CARGO_PKG_NAME").to_string()).chain(args);
    let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;

    if cli.vault.is_some() && cli.positional_vault.is_some() {
        return Err(
            "error: provide the vault either positionally or with --vault, not both".to_string(),
        );
    }

    Ok(CliOptions {
        slim_mode: cli.slim,
        vault_path: cli.vault.or(cli.positional_vault),
    })
}

fn parse_vault_path(value: &str) -> Result<PathBuf, String> {
    if value.is_empty() {
        return Err("vault path cannot be empty".to_string());
    }

    Ok(expand_tilde(value))
}

fn parse_existing_vault_path(value: &str) -> Result<PathBuf, String> {
    let path = parse_vault_path(value)?;

    if !path.is_file() {
        return Err(format!("vault file does not exist: {}", path.display()));
    }

    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("kdbx"))
    {
        return Err(format!(
            "vault file must have a .kdbx extension: {}",
            path.display()
        ));
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::parse_cli_args;
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> Result<(bool, Option<PathBuf>), String> {
        parse_cli_args(args.iter().map(|arg| (*arg).to_string()))
            .map(|options| (options.slim_mode, options.vault_path))
    }

    #[test]
    fn parses_vault_and_slim_options() {
        assert_eq!(
            parse(&["--vault", "/tmp/work vault.kdbx", "--slim"]),
            Ok((true, Some(PathBuf::from("/tmp/work vault.kdbx"))))
        );
    }

    #[test]
    fn parses_existing_positional_vault() {
        let path =
            std::env::temp_dir().join(format!("jaiba-cli-test-{}-vault.kdbx", std::process::id()));
        std::fs::write(&path, []).expect("test vault should be created");

        assert_eq!(
            parse(&[path.to_str().expect("path should be valid UTF-8")]),
            Ok((false, Some(path.clone())))
        );

        std::fs::remove_file(path).expect("test vault should be removed");
    }

    #[test]
    fn rejects_invalid_vault_options() {
        assert!(parse(&["--vault"]).is_err());
        assert!(parse(&["--vault", "--slim"]).is_err());
        assert!(parse(&["--vault", "/tmp/a.kdbx", "--vault", "/tmp/b.kdbx"]).is_err());
        assert!(parse(&["--unknown"]).is_err());
        assert!(parse(&["vault.kdbx"]).is_err());
        assert!(parse(&["/tmp/missing.kdbx"]).is_err());
        assert!(parse(&["/tmp"]).is_err());
        assert!(parse(&["--vault", "/tmp/a.kdbx", "/tmp/b.kdbx"]).is_err());
    }
}

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::util::expand_tilde;

const SAMPLE_CONFIG: &str = include_str!("../config.toml.sample");

pub struct Config {
    pub default_database: Option<PathBuf>,
    pub keyfile: Option<PathBuf>,
    pub auto_lock: Duration,
    pub clipboard_timeout: Duration,
    pub theme: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_database: None,
            keyfile: None,
            auto_lock: Duration::from_secs(300),
            clipboard_timeout: Duration::from_secs(15),
            theme: None,
        }
    }
}

#[derive(Deserialize, Default)]
struct ConfigFile {
    default_database: Option<String>,
    keyfile: Option<String>,
    auto_lock: Option<u64>,
    clipboard_timeout: Option<u64>,
    theme: Option<String>,
}

pub fn load_config() -> anyhow::Result<Config> {
    let path = expand_tilde("~/.config/rama/jaiba_config.toml");

    if !path.is_file() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("couldn't create {}", parent.display()))?;
        }

        fs::write(&path, SAMPLE_CONFIG)
            .with_context(|| format!("couldn't write {}", path.display()))?;
    }

    let text =
        fs::read_to_string(&path).with_context(|| format!("couldn't read {}", path.display()))?;
    parse_config(&text)
}

fn parse_config(text: &str) -> anyhow::Result<Config> {
    let raw: ConfigFile = toml::from_str(text)?;

    let defaults = Config::default();

    Ok(Config {
        default_database: raw.default_database.map(|s| expand_tilde(&s)),
        keyfile: raw.keyfile.map(|s| expand_tilde(&s)),
        auto_lock: raw
            .auto_lock
            .map(Duration::from_secs)
            .unwrap_or(defaults.auto_lock),
        clipboard_timeout: raw
            .clipboard_timeout
            .map(Duration::from_secs)
            .unwrap_or(defaults.clipboard_timeout),
        theme: raw.theme,
    })
}

#[derive(Serialize)]
struct ConfigFileOut<'a> {
    default_database: Option<String>,
    keyfile: Option<String>,
    auto_lock: u64,
    clipboard_timeout: u64,
    theme: Option<&'a str>,
}

pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let path = expand_tilde("~/.config/rama/jaiba_config.toml");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("couldn't create {}", parent.display()))?;
    }

    let text = serialize_config(config)?;

    fs::write(&path, text).with_context(|| format!("couldn't write {}", path.display()))?;

    Ok(())
}

fn serialize_config(config: &Config) -> anyhow::Result<String> {
    let out = ConfigFileOut {
        default_database: config
            .default_database
            .as_ref()
            .map(|p| p.display().to_string()),
        keyfile: config.keyfile.as_ref().map(|p| p.display().to_string()),
        auto_lock: config.auto_lock.as_secs(),
        clipboard_timeout: config.clipboard_timeout.as_secs(),
        theme: config.theme.as_deref(),
    };

    toml::to_string_pretty(&out).context("couldn't serialize config")
}

#[cfg(test)]
mod tests {
    use super::{Config, parse_config, serialize_config};
    use std::path::PathBuf;

    #[test]
    fn parses_optional_keyfile_path() {
        let config = parse_config(
            r#"
                default_database = "/tmp/passwords.kdbx"
                keyfile = "/tmp/passwords.keyx"
            "#,
        )
        .expect("config should parse");

        assert_eq!(
            config.default_database,
            Some(PathBuf::from("/tmp/passwords.kdbx"))
        );
        assert_eq!(config.keyfile, Some(PathBuf::from("/tmp/passwords.keyx")));
    }

    #[test]
    fn keyfile_remains_optional() {
        let config = parse_config("default_database = \"/tmp/passwords.kdbx\"")
            .expect("config should parse");

        assert_eq!(config.keyfile, None);

        let text = serialize_config(&config).expect("config should serialize");
        assert!(!text.contains("keyfile"));
    }

    #[test]
    fn serializes_keyfile_path() {
        let config = Config {
            keyfile: Some(PathBuf::from("/tmp/passwords.keyx")),
            ..Config::default()
        };

        let text = serialize_config(&config).expect("config should serialize");
        assert!(text.contains("keyfile = \"/tmp/passwords.keyx\""));
    }
}

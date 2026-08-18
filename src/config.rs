use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::util::expand_tilde;

const LEGACY_VAULT_NAME: &str = "default";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultConfig {
    pub name: String,
    pub path: PathBuf,
    pub keyfile: Option<PathBuf>,
}

pub struct Config {
    pub vaults: Vec<VaultConfig>,
    pub default_vault: Option<String>,
    pub auto_lock: Duration,
    pub clipboard_timeout: Duration,
    pub theme: Option<String>,
}

impl Config {
    pub fn default_vault(&self) -> Option<&VaultConfig> {
        self.default_vault
            .as_deref()
            .and_then(|name| self.vault(name))
            .or_else(|| self.vaults.first())
    }

    pub fn vault(&self, name: &str) -> Option<&VaultConfig> {
        self.vaults.iter().find(|vault| vault.name == name)
    }

    pub fn vault_mut(&mut self, name: &str) -> Option<&mut VaultConfig> {
        self.vaults.iter_mut().find(|vault| vault.name == name)
    }

    pub fn ensure_default_vault(&mut self, path: PathBuf) -> &mut VaultConfig {
        if self.vaults.is_empty() {
            self.vaults.push(VaultConfig {
                name: LEGACY_VAULT_NAME.to_string(),
                path,
                keyfile: None,
            });
        }

        let name = self
            .default_vault
            .clone()
            .filter(|name| self.vault(name).is_some())
            .unwrap_or_else(|| self.vaults[0].name.clone());
        self.default_vault = Some(name.clone());
        self.vault_mut(&name)
            .expect("default vault must refer to a configured vault")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vaults: Vec::new(),
            default_vault: None,
            auto_lock: Duration::from_secs(300),
            clipboard_timeout: Duration::from_secs(15),
            theme: None,
        }
    }
}

#[derive(Deserialize, Default)]
struct ConfigFile {
    #[serde(alias = "active_vault")]
    default_vault: Option<String>,
    #[serde(default)]
    vaults: Vec<VaultFile>,

    // Pre-multi-vault config. These are migrated to a vault named "default".
    default_database: Option<String>,
    keyfile: Option<String>,

    auto_lock: Option<u64>,
    clipboard_timeout: Option<u64>,
    theme: Option<String>,
}

#[derive(Deserialize)]
struct VaultFile {
    name: String,
    path: String,
    keyfile: Option<String>,
}

pub fn load_config() -> anyhow::Result<Config> {
    let path = expand_tilde("~/.config/jaiba/config.toml");
    let text =
        fs::read_to_string(&path).with_context(|| format!("couldn't read {}", path.display()))?;
    parse_config(&text)
}

fn parse_config(text: &str) -> anyhow::Result<Config> {
    let raw: ConfigFile = toml::from_str(text)?;
    let defaults = Config::default();

    let mut vaults = raw
        .vaults
        .into_iter()
        .map(|vault| VaultConfig {
            name: vault.name,
            path: expand_tilde(&vault.path),
            keyfile: vault.keyfile.map(|path| expand_tilde(&path)),
        })
        .collect::<Vec<_>>();

    if vaults.is_empty()
        && let Some(path) = raw.default_database
    {
        vaults.push(VaultConfig {
            name: LEGACY_VAULT_NAME.to_string(),
            path: expand_tilde(&path),
            keyfile: raw.keyfile.map(|path| expand_tilde(&path)),
        });
    }

    validate_vaults(&vaults)?;

    let default_vault = if vaults.is_empty() {
        None
    } else if let Some(name) = raw.default_vault {
        if vaults.iter().any(|vault| vault.name == name) {
            Some(name)
        } else {
            anyhow::bail!("default_vault {name:?} does not match a configured vault");
        }
    } else {
        Some(vaults[0].name.clone())
    };

    Ok(Config {
        vaults,
        default_vault,
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

fn validate_vaults(vaults: &[VaultConfig]) -> anyhow::Result<()> {
    let mut names = HashSet::new();

    for vault in vaults {
        if vault.name.trim().is_empty() {
            anyhow::bail!("vault names cannot be empty");
        }
        if !names.insert(vault.name.as_str()) {
            anyhow::bail!("vault name {:?} is configured more than once", vault.name);
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct ConfigFileOut<'a> {
    default_vault: Option<&'a str>,
    auto_lock: u64,
    clipboard_timeout: u64,
    theme: Option<&'a str>,
    vaults: Vec<VaultFileOut<'a>>,
}

#[derive(Serialize)]
struct VaultFileOut<'a> {
    name: &'a str,
    path: String,
    keyfile: Option<String>,
}

pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let path = expand_tilde("~/.config/jaiba/config.toml");

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
        default_vault: config.default_vault.as_deref(),
        auto_lock: config.auto_lock.as_secs(),
        clipboard_timeout: config.clipboard_timeout.as_secs(),
        theme: config.theme.as_deref(),
        vaults: config
            .vaults
            .iter()
            .map(|vault| VaultFileOut {
                name: &vault.name,
                path: display_path(&vault.path),
                keyfile: vault.keyfile.as_deref().map(display_path),
            })
            .collect(),
    };

    toml::to_string_pretty(&out).context("couldn't serialize config")
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::{Config, VaultConfig, parse_config, serialize_config};
    use std::path::PathBuf;

    #[test]
    fn parses_named_vaults_and_default() {
        let config = parse_config(
            r#"
                default_vault = "work"

                [[vaults]]
                name = "personal"
                path = "/tmp/personal.kdbx"

                [[vaults]]
                name = "work"
                path = "/tmp/work.kdbx"
                keyfile = "/tmp/work.keyx"
            "#,
        )
        .expect("config should parse");

        assert_eq!(config.default_vault.as_deref(), Some("work"));
        assert_eq!(config.vaults.len(), 2);
        assert_eq!(config.vaults[0].name, "personal");
        assert_eq!(config.vaults[0].keyfile, None);
        assert_eq!(config.vaults[1].path, PathBuf::from("/tmp/work.kdbx"));
        assert_eq!(
            config.vaults[1].keyfile,
            Some(PathBuf::from("/tmp/work.keyx"))
        );
    }

    #[test]
    fn accepts_active_vault_as_an_alias_for_default_vault() {
        let config = parse_config(
            r#"
                active_vault = "work"
                [[vaults]]
                name = "work"
                path = "/tmp/work.kdbx"
            "#,
        )
        .expect("active_vault alias should parse");

        assert_eq!(config.default_vault.as_deref(), Some("work"));
    }

    #[test]
    fn migrates_legacy_database_and_keyfile_to_named_vault() {
        let config = parse_config(
            r#"
                default_database = "/tmp/passwords.kdbx"
                keyfile = "/tmp/passwords.keyx"
            "#,
        )
        .expect("legacy config should parse");

        assert_eq!(config.default_vault.as_deref(), Some("default"));
        assert_eq!(config.vaults.len(), 1);
        assert_eq!(config.vaults[0].name, "default");
        assert_eq!(config.vaults[0].path, PathBuf::from("/tmp/passwords.kdbx"));
        assert_eq!(
            config.vaults[0].keyfile,
            Some(PathBuf::from("/tmp/passwords.keyx"))
        );

        let text = serialize_config(&config).expect("migrated config should serialize");
        assert!(text.contains("default_vault = \"default\""));
        assert!(text.contains("[[vaults]]"));
        assert!(!text.contains("default_database"));
    }

    #[test]
    fn legacy_keyfile_remains_optional() {
        let config = parse_config("default_database = \"/tmp/passwords.kdbx\"")
            .expect("legacy config should parse");

        assert_eq!(config.vaults[0].keyfile, None);

        let text = serialize_config(&config).expect("config should serialize");
        assert!(!text.contains("keyfile"));
    }

    #[test]
    fn rejects_unknown_default_and_duplicate_names() {
        let unknown = parse_config(
            r#"
                default_vault = "missing"
                [[vaults]]
                name = "personal"
                path = "/tmp/personal.kdbx"
            "#,
        );
        assert!(unknown.is_err());

        let duplicate = parse_config(
            r#"
                [[vaults]]
                name = "personal"
                path = "/tmp/one.kdbx"
                [[vaults]]
                name = "personal"
                path = "/tmp/two.kdbx"
            "#,
        );
        assert!(duplicate.is_err());
    }

    #[test]
    fn serializes_all_named_vaults() {
        let config = Config {
            vaults: vec![
                VaultConfig {
                    name: "personal".to_string(),
                    path: PathBuf::from("/tmp/personal.kdbx"),
                    keyfile: None,
                },
                VaultConfig {
                    name: "work".to_string(),
                    path: PathBuf::from("/tmp/work.kdbx"),
                    keyfile: Some(PathBuf::from("/tmp/work.keyx")),
                },
            ],
            default_vault: Some("personal".to_string()),
            ..Config::default()
        };

        let text = serialize_config(&config).expect("config should serialize");
        assert!(text.contains("name = \"personal\""));
        assert!(text.contains("name = \"work\""));
        assert!(text.contains("keyfile = \"/tmp/work.keyx\""));

        let reparsed = parse_config(&text).expect("serialized config should parse again");
        assert_eq!(reparsed.vaults, config.vaults);
        assert_eq!(reparsed.default_vault, config.default_vault);
    }
}

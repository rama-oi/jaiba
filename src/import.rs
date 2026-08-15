use std::fs;
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

use crate::db::{Entry, unlock_database};

pub enum ImportFormat {
    Kdbx,
    Csv,
    Json,
}

pub fn detect_format(path: &Path) -> anyhow::Result<ImportFormat> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());

    match ext.as_deref() {
        Some("kdbx") => Ok(ImportFormat::Kdbx),
        Some("csv") => Ok(ImportFormat::Csv),
        Some("json") => Ok(ImportFormat::Json),
        _ => anyhow::bail!("unrecognized file extension, expected .kdbx, .csv, or .json"),
    }
}

pub fn import_kdbx(path: &Path, password: &str) -> anyhow::Result<Vec<Entry>> {
    let (_, _, entries) = unlock_database(path, password, None)?;
    Ok(entries)
}

pub fn import_csv(path: &Path) -> anyhow::Result<Vec<Entry>> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("couldn't open {}", path.display()))?;

    let headers = reader.headers()?.clone();

    let find_col = |names: &[&str]| -> Option<usize> {
        headers.iter().position(|h| {
            let h = h.trim().to_ascii_lowercase();
            names.contains(&h.as_str())
        })
    };

    let name_col = find_col(&["name", "title"]);
    let user_col = find_col(&["username", "user", "login", "login_username", "email"]);
    let password_col = find_col(&["password", "login_password"]);
    let url_col = find_col(&["url", "uri", "website", "login_uri"]);
    let totp_col = find_col(&["totp", "otp", "otpauth", "login_totp"]);

    let get = |record: &csv::StringRecord, idx: Option<usize>| -> String {
        idx.and_then(|i| record.get(i))
            .unwrap_or("")
            .trim()
            .to_string()
    };

    let mut entries = Vec::new();

    for record in reader.records() {
        let record = record.context("couldn't read a row")?;

        let entry = Entry {
            id: None,
            name: get(&record, name_col),
            user: get(&record, user_col),
            password: get(&record, password_col),
            url: get(&record, url_col),
            totp: get(&record, totp_col),
            date_last_modify: String::new(),
            password_reuse_count: 0,
            duplicate_user_count: 0,
        };

        if entry.name.is_empty() && entry.user.is_empty() && entry.password.is_empty() {
            continue;
        }

        entries.push(entry);
    }

    if name_col.is_none() && user_col.is_none() && password_col.is_none() {
        anyhow::bail!("couldn't find name/username/password columns in the CSV header");
    }

    Ok(entries)
}

pub fn import_json(path: &Path) -> anyhow::Result<Vec<Entry>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("couldn't read {}", path.display()))?;

    // Bitwarden-style export: { "items": [ { "name", "login": { ... } }, ... ] }
    if let Ok(export) = serde_json::from_str::<BitwardenExport>(&text) {
        if !export.items.is_empty() {
            return Ok(export.items.into_iter().map(Entry::from).collect());
        }
    }

    // Fall back to a flat array of entries.
    let flat: Vec<FlatEntry> = serde_json::from_str(&text).context(
        "couldn't parse JSON: expected a Bitwarden-style export or a flat array of entries",
    )?;

    Ok(flat.into_iter().map(Entry::from).collect())
}

#[derive(Deserialize)]
struct BitwardenExport {
    #[serde(default)]
    items: Vec<BitwardenItem>,
}

#[derive(Deserialize)]
struct BitwardenItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    login: Option<BitwardenLogin>,
}

#[derive(Deserialize, Default)]
struct BitwardenLogin {
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    totp: Option<String>,
    #[serde(default)]
    uris: Vec<BitwardenUri>,
}

#[derive(Deserialize)]
struct BitwardenUri {
    #[serde(default)]
    uri: Option<String>,
}

impl From<BitwardenItem> for Entry {
    fn from(item: BitwardenItem) -> Self {
        let login = item.login.unwrap_or_default();

        Entry {
            id: None,
            name: item.name,
            user: login.username.unwrap_or_default(),
            password: login.password.unwrap_or_default(),
            url: login
                .uris
                .into_iter()
                .find_map(|u| u.uri)
                .unwrap_or_default(),
            totp: login.totp.unwrap_or_default(),
            date_last_modify: String::new(),
            password_reuse_count: 0,
            duplicate_user_count: 0,
        }
    }
}

#[derive(Deserialize)]
struct FlatEntry {
    #[serde(alias = "title", default)]
    name: String,
    #[serde(alias = "user", alias = "login", alias = "email", default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(alias = "uri", alias = "website", default)]
    url: String,
    #[serde(alias = "otp", default)]
    totp: String,
}

impl From<FlatEntry> for Entry {
    fn from(f: FlatEntry) -> Self {
        Entry {
            id: None,
            name: f.name,
            user: f.username,
            password: f.password,
            url: f.url,
            totp: f.totp,
            date_last_modify: String::new(),
            password_reuse_count: 0,
            duplicate_user_count: 0,
        }
    }
}

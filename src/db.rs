use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::Utc;
use keepass::db::{EntryId, EntryMut, EntryRef, Times, fields};
use keepass::{Database, DatabaseKey};

fn resolve_totp(e: &EntryRef) -> String {
    if let Some(otp) = e.get_raw_otp_value() {
        if !otp.trim().is_empty() {
            return otp.to_string();
        }
    }

    let Some(seed) = e.get("TOTP Seed") else {
        return String::new();
    };
    let seed = seed.trim();
    if seed.is_empty() {
        return String::new();
    }

    let (period, digits) = e
        .get("TOTP Settings")
        .and_then(|settings| settings.split_once(';'))
        .and_then(|(p, d)| Some((p.trim().parse::<u32>().ok()?, d.trim().parse::<u32>().ok()?)))
        .unwrap_or((30, 6));

    let label = e.get_title().unwrap_or("");
    let user = e.get_username().unwrap_or("");
    let encoded_label = urlencoding_encode(&format!("{label}:{user}"));
    let encoded_issuer = urlencoding_encode(label);

    format!(
        "otpauth://totp/{encoded_label}?secret={seed}&period={period}&digits={digits}&issuer={encoded_issuer}"
    )
}

pub struct TotpCode {
    pub code: String,
    pub valid_for: std::time::Duration,
}

pub fn current_totp_code(raw: &str) -> Option<TotpCode> {
    if raw.trim().is_empty() {
        return None;
    }

    let totp = parse_totp(raw)?;
    let otp_code = totp.value_now().ok()?;

    Some(TotpCode {
        code: otp_code.code,
        valid_for: otp_code.valid_for,
    })
}

fn parse_totp(raw: &str) -> Option<keepass::db::TOTP> {
    let raw = raw.trim();
    let mut uri = url::Url::parse(raw).ok()?;

    if uri.scheme() == "otpauth" && uri.host_str() == Some("totp") {
        let mut has_digits = false;
        let mut query = uri
            .query_pairs()
            .map(|(key, value)| {
                let key = key.into_owned();
                let value = match key.as_str() {
                    "secret" => value
                        .chars()
                        .filter(|character| !character.is_ascii_whitespace() && *character != '-')
                        .collect::<String>()
                        .to_ascii_uppercase(),
                    "algorithm" => value.to_ascii_uppercase(),
                    "digits" => {
                        has_digits = true;
                        value.into_owned()
                    }
                    _ => value.into_owned(),
                };
                (key, value)
            })
            .collect::<Vec<_>>();

        if !has_digits {
            query.push(("digits".to_string(), "6".to_string()));
        }

        uri.query_pairs_mut().clear().extend_pairs(&query);
    }

    uri.as_str().parse().ok()
}

fn urlencoding_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Clone, Default)]
pub struct Entry {
    pub id: Option<EntryId>,
    pub name: String,
    pub user: String,
    pub password: String,
    pub url: String,
    pub totp: String,
    pub notes: String,
    pub date_last_modify: String,
    pub password_reuse_count: u32,
    pub duplicate_user_count: u32,
}

fn format_days_ago(dt: chrono::NaiveDateTime) -> String {
    let days = (Utc::now().naive_utc() - dt).num_days();

    match days {
        d if d <= 0 => "today".to_string(),
        1 => "1 day ago".to_string(),
        d => format!("{d} days ago"),
    }
}

pub fn unlock_database(
    path: &Path,
    password: &str,
    keyfile_path: Option<&Path>,
) -> anyhow::Result<(Database, DatabaseKey, Vec<Entry>)> {
    let mut file =
        fs::File::open(path).with_context(|| format!("couldn't open {}", path.display()))?;

    let key = build_database_key(password, keyfile_path)?;
    let db = Database::open(&mut file, key.clone()).map_err(|err| match keyfile_path {
        Some(_) => anyhow::anyhow!("unlock failed with password + keyfile: {err}"),
        None => anyhow::anyhow!("unlock failed with password: {err}"),
    })?;

    let entries = db
        .iter_all_entries()
        .map(|e| {
            let date_last_modify = e
                .times
                .last_modification
                .map(format_days_ago)
                .unwrap_or_default();

            Entry {
                id: Some(e.id()),
                name: e.get_title().unwrap_or("(no title)").to_string(),
                user: e.get_username().unwrap_or_default().to_string(),
                password: e.get_password().unwrap_or_default().to_string(),
                url: e.get_url().unwrap_or_default().to_string(),
                totp: resolve_totp(&e),
                notes: e.get(fields::NOTES).unwrap_or_default().to_string(),
                date_last_modify,
                password_reuse_count: 0,
                duplicate_user_count: 0,
            }
        })
        .collect();

    Ok((db, key, entries))
}

pub fn create_database(
    path: &Path,
    password: &str,
    keyfile_path: Option<&Path>,
) -> anyhow::Result<(Database, DatabaseKey, Vec<Entry>)> {
    if path.exists() {
        anyhow::bail!("a file already exists at {}", path.display());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("couldn't create {}", parent.display()))?;
    }

    let db = Database::new();
    let key = build_database_key(password, keyfile_path)?;

    write_to_disk(path, &key, &db)?;

    Ok((db, key, Vec::new()))
}

pub fn build_database_key(
    password: &str,
    keyfile_path: Option<&Path>,
) -> anyhow::Result<DatabaseKey> {
    let mut key = DatabaseKey::new().with_password(password);

    if let Some(path) = keyfile_path {
        let mut keyfile = fs::File::open(path)
            .with_context(|| format!("couldn't open keyfile {}", path.display()))?;

        if keyfile
            .metadata()
            .with_context(|| format!("couldn't inspect keyfile {}", path.display()))?
            .len()
            == 0
        {
            anyhow::bail!("keyfile {} is empty", path.display());
        }

        key = key
            .with_keyfile(&mut keyfile)
            .with_context(|| format!("couldn't read keyfile {}", path.display()))?;
    }

    Ok(key)
}

pub fn save_database(
    path: &Path,
    key: &DatabaseKey,
    db: &mut Database,
    entries: &mut [Entry],
) -> anyhow::Result<()> {
    for entry in entries.iter_mut() {
        write_entry(db, entry)?;
    }

    write_to_disk(path, key, db)
}

fn write_to_disk(path: &Path, key: &DatabaseKey, db: &Database) -> anyhow::Result<()> {
    let tmp_path = sibling_tmp_path(path);

    {
        let mut file = fs::File::create(&tmp_path)
            .with_context(|| format!("couldn't create {}", tmp_path.display()))?;

        db.save(&mut file, key.clone())
            .map_err(|err| anyhow::anyhow!("failed to write database: {err}"))?;
    }

    fs::rename(&tmp_path, path).with_context(|| format!("couldn't replace {}", path.display()))?;

    Ok(())
}

fn write_entry(db: &mut Database, entry: &mut Entry) -> anyhow::Result<()> {
    match entry.id {
        Some(id) => {
            let mut e = db
                .entry_mut(id)
                .context("entry no longer exists in the database")?;
            apply_fields(&mut e, entry);
        }
        None => {
            let mut root = db.root_mut();
            let mut e = root.add_entry();
            apply_fields(&mut e, entry);
            entry.id = Some(e.id());
        }
    }

    Ok(())
}

pub fn delete_entry(
    path: &Path,
    key: &DatabaseKey,
    db: &mut Database,
    id: EntryId,
) -> anyhow::Result<()> {
    db.entry_mut(id)
        .context("entry no longer exists in the database")?
        .remove();

    write_to_disk(path, key, db)
}

fn apply_fields(e: &mut EntryMut<'_>, entry: &Entry) {
    e.set_unprotected(fields::TITLE, entry.name.clone());
    e.set_unprotected(fields::USERNAME, entry.user.clone());
    e.set_protected(fields::PASSWORD, entry.password.clone());
    e.set_unprotected(fields::URL, entry.url.clone());
    e.set_unprotected(fields::OTP, entry.totp.clone());
    e.set_unprotected(fields::NOTES, entry.notes.clone());
    e.times.last_modification = Some(Times::now());
}

fn sibling_tmp_path(path: &Path) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(".tmp");
    PathBuf::from(name)
}

pub fn calculate_warnings(entries: &mut Vec<Entry>) {
    let mut password_counts: HashMap<String, u32> = HashMap::new();
    let mut user_counts: HashMap<String, u32> = HashMap::new();

    for entry in entries.iter() {
        if !entry.password.trim().is_empty() {
            *password_counts.entry(entry.password.clone()).or_insert(0) += 1;
        }

        if !entry.user.trim().is_empty() {
            *user_counts.entry(entry.user.clone()).or_insert(0) += 1;
        }
    }

    for entry in entries.iter_mut() {
        entry.password_reuse_count = if entry.password.trim().is_empty() {
            0
        } else {
            password_counts.get(&entry.password).copied().unwrap_or(0)
        };

        entry.duplicate_user_count = if entry.user.trim().is_empty() {
            0
        } else {
            user_counts.get(&entry.user).copied().unwrap_or(0)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_database_key, create_database, current_totp_code, save_database, unlock_database,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    const XML_V1_KEYFILE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<KeyFile>
    <Meta>
        <Version>1.00</Version>
    </Meta>
    <Key>
        <Data>NXyYiJMHg3ls+eBmjbAjWec9lcOToJiofbhNiFMTJMw=</Data>
    </Key>
</KeyFile>"#;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("jaiba-test-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn join(&self, path: impl AsRef<Path>) -> PathBuf {
            self.0.join(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn password_only_databases_still_unlock() {
        let dir = TestDir::new();
        let database_path = dir.join("password-only.kdbx");

        create_database(&database_path, "correct horse", None)
            .expect("password-only database should be created");
        unlock_database(&database_path, "correct horse", None)
            .expect("password-only database should unlock");
    }

    #[test]
    fn password_and_keyfile_databases_require_both_factors() {
        let dir = TestDir::new();
        let database_path = dir.join("composite.kdbx");
        let keyfile_path = dir.join("database.keyx");
        fs::write(&keyfile_path, XML_V1_KEYFILE).expect("XML v1 keyfile should be written");

        create_database(&database_path, "correct horse", Some(&keyfile_path))
            .expect("composite-key database should be created");
        unlock_database(&database_path, "correct horse", Some(&keyfile_path))
            .expect("database should unlock with password and keyfile");

        assert!(unlock_database(&database_path, "correct horse", None).is_err());
        assert!(unlock_database(&database_path, "wrong password", Some(&keyfile_path)).is_err());
    }

    #[test]
    fn password_change_keeps_keyfile_requirement() {
        let dir = TestDir::new();
        let database_path = dir.join("composite.kdbx");
        let keyfile_path = dir.join("database.keyx");
        fs::write(&keyfile_path, [42_u8; 32]).expect("keyfile should be written");

        let (mut database, _, mut entries) =
            create_database(&database_path, "old password", Some(&keyfile_path))
                .expect("composite-key database should be created");
        let new_key = build_database_key("new password", Some(&keyfile_path))
            .expect("new composite key should be built");

        save_database(&database_path, &new_key, &mut database, &mut entries)
            .expect("database should be re-encrypted");

        unlock_database(&database_path, "new password", Some(&keyfile_path))
            .expect("new password and keyfile should unlock");
        assert!(unlock_database(&database_path, "new password", None).is_err());
        assert!(unlock_database(&database_path, "old password", Some(&keyfile_path)).is_err());
    }

    #[test]
    fn missing_and_empty_keyfiles_have_clear_errors() {
        let dir = TestDir::new();
        let database_path = dir.join("placeholder.kdbx");
        let missing_keyfile = dir.join("missing.keyx");
        let empty_keyfile = dir.join("empty.keyx");
        fs::write(&database_path, []).expect("placeholder database should be written");
        fs::write(&empty_keyfile, []).expect("empty keyfile should be written");

        let missing_error = unlock_database(&database_path, "secret", Some(&missing_keyfile))
            .err()
            .expect("missing keyfile should fail")
            .to_string();
        assert!(missing_error.contains("couldn't open keyfile"));
        assert!(missing_error.contains(&missing_keyfile.display().to_string()));

        let empty_error = unlock_database(&database_path, "secret", Some(&empty_keyfile))
            .err()
            .expect("empty keyfile should fail")
            .to_string();
        assert!(empty_error.contains("keyfile"));
        assert!(empty_error.contains("is empty"));
    }

    #[test]
    fn wrong_keyfile_error_preserves_the_root_cause() {
        let dir = TestDir::new();
        let database_path = dir.join("composite.kdbx");
        let correct_keyfile = dir.join("correct.keyx");
        let wrong_keyfile = dir.join("wrong.keyx");
        fs::write(&correct_keyfile, [7_u8; 32]).expect("correct keyfile should be written");
        fs::write(&wrong_keyfile, [9_u8; 32]).expect("wrong keyfile should be written");

        create_database(&database_path, "secret", Some(&correct_keyfile))
            .expect("composite-key database should be created");

        let error = unlock_database(&database_path, "secret", Some(&wrong_keyfile))
            .err()
            .expect("wrong keyfile should fail")
            .to_string();
        assert!(error.contains("password + keyfile"));
        assert!(error.contains("Incorrect key"));
    }

    #[test]
    fn lowercase_otpauth_secret_uses_six_digit_default() {
        let uri = concat!(
            "otpauth://totp/Example%3Auser%40example.com?",
            "secret=jbswy3dpehpk3pxp&issuer=Example"
        );

        let code = current_totp_code(uri).expect("lowercase Base32 secret should be accepted");

        assert_eq!(code.code.len(), 6);
        assert!(
            code.code
                .chars()
                .all(|character| character.is_ascii_digit())
        );
    }

    #[test]
    fn explicit_totp_digit_count_is_preserved() {
        let uri = concat!(
            "otpauth://totp/Example%3Auser%40example.com?",
            "secret=JBSWY3DPEHPK3PXP&digits=8&issuer=Example"
        );

        let code = current_totp_code(uri).expect("valid TOTP URI should generate a code");

        assert_eq!(code.code.len(), 8);
    }
}

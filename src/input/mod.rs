pub mod command;
pub mod edit;
pub mod index;
pub mod login;
pub mod settings;
pub mod vaults;

use crossterm::event::KeyCode;

pub fn normalize_shortcut(key: KeyCode) -> KeyCode {
    match key {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        other => other,
    }
}

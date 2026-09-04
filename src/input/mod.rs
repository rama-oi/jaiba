pub mod command;
pub mod edit;
pub mod index;
pub mod login;
pub mod open;
pub mod settings;

use crossterm::event::KeyCode;

pub fn normalize_shortcut(key: KeyCode) -> KeyCode {
    match key {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        other => other,
    }
}

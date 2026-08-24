use std::path::PathBuf;

fn strip_surrounding_quotes(path: &str) -> &str {
    let bytes = path.as_bytes();

    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];

        if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
            return &path[1..path.len() - 1];
        }
    }

    path
}

pub fn expand_tilde(path: &str) -> PathBuf {
    let path = strip_surrounding_quotes(path.trim());

    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

pub fn default_new_database_path() -> PathBuf {
    expand_tilde("~/.local/share/rama/default.kdbx")
}

pub fn wrap_help_items(items: &[&str], width: u16) -> Vec<String> {
    let width = width as usize;
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for item in items {
        let extra = if current.is_empty() { 0 } else { 2 };

        if !current.is_empty() && current.len() + extra + item.len() > width {
            lines.push(std::mem::take(&mut current));
        }

        if !current.is_empty() {
            current.push_str("  ");
        }
        current.push_str(item);
    }

    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }

    lines
}

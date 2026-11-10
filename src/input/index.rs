use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::input::command::{handle_shortcut, preview_entry};

pub fn handle_index_input(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('o') => {
                app.reset_open_database();
                app.screen = crate::app::Screen::OpenDatabase;
                return;
            }
            KeyCode::Char('w') => {
                app.close_active_tab();
                return;
            }
            KeyCode::Left => {
                switch_relative(app, -1);
                return;
            }
            KeyCode::Right | KeyCode::Tab => {
                switch_relative(app, 1);
                return;
            }
            _ => {}
        }

        if let KeyCode::Char(c) = key.code {
            app.status = None;
            handle_shortcut(app, c.to_ascii_lowercase());
        }
        return;
    }

    match key.code {
        KeyCode::Char(c) => {
            app.status = None;

            if app.query.len() < app.max_len {
                app.query.push(c);
                app.refresh_filter();
            }
        }

        KeyCode::Backspace => {
            app.status = None;
            app.query.pop();
            app.refresh_filter();
        }

        KeyCode::Enter => {
            app.status = None;
            preview_entry(app);
        }

        KeyCode::Esc => {
            app.should_quit = true;
        }

        KeyCode::Down => {
            let row_count = app.filtered.len();

            if row_count == 0 {
                return;
            }

            app.status = None;

            let selected = app.index_state.selected().unwrap_or(0);

            let next = if selected >= row_count - 1 {
                0
            } else {
                selected + 1
            };

            app.index_state.select(Some(next));
        }

        KeyCode::Up => {
            let row_count = app.filtered.len();

            if row_count == 0 {
                return;
            }

            app.status = None;

            let selected = app.index_state.selected().unwrap_or(0);

            let prev = if selected == 0 {
                row_count - 1
            } else {
                selected - 1
            };

            app.index_state.select(Some(prev));
        }

        _ => {}
    }
}

fn switch_relative(app: &mut App, delta: isize) {
    if app.tabs.len() < 2 {
        return;
    }

    let count = app.tabs.len() as isize;
    let target = (app.active_tab as isize + delta).rem_euclid(count) as usize;
    app.switch_tab(target);
}

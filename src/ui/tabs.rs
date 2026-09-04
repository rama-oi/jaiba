use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Tabs},
};
use unicode_width::UnicodeWidthStr;

use crate::app::App;

pub fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    if app.tabs.len() < 2 {
        return;
    }

    let titles = app
        .tabs
        .iter()
        .map(|tab| Line::from(tab.label.clone()))
        .collect::<Vec<_>>();
    let tabs = Tabs::new(titles)
        .select(app.active_tab)
        .highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg)
                .bold(),
        )
        .style(Style::default().fg(app.theme.text))
        .divider(" | ")
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(tabs, area);
}

pub fn tab_at_column(app: &App, column: u16) -> Option<usize> {
    if app.tabs.len() < 2 {
        return None;
    }

    let mut start = 1u16;
    for (index, tab) in app.tabs.iter().enumerate() {
        let width = UnicodeWidthStr::width(tab.label.as_str()) as u16;
        if (start..start.saturating_add(width)).contains(&column) {
            return Some(index);
        }
        start = start.saturating_add(width).saturating_add(3);
    }

    None
}

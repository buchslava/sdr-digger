use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::AppState;
use crate::i18n::{t, Msg};
use crate::ui::theme::UiPalette;

pub fn draw(frame: &mut Frame, area: Rect, app: &AppState, palette: &UiPalette) {
    let title = t(app.lang, Msg::CoachTitle);
    let lines: Vec<Line> = app
        .coach_lines
        .iter()
        .map(|s| Line::from(Span::styled(s.as_str(), palette.text_style())))
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(palette.accent_style())
        .style(
            palette
                .text_style()
                .bg(palette.coach_bg),
        );
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );

    // Morse strip at bottom of coach if enabled
    if app.case.readout.morse {
        let morse_area = Rect {
            x: area.x + 1,
            y: area.bottom().saturating_sub(2),
            width: area.width.saturating_sub(2),
            height: 1,
        };
        let text = if app.morse.status.text.is_empty() {
            format!("{}: …", t(app.lang, Msg::MorseStrip))
        } else {
            format!(
                "{}: {}",
                t(app.lang, Msg::MorseStrip),
                app.morse.status.text
            )
        };
        frame.render_widget(
            Paragraph::new(text).style(
                palette
                    .accent_style()
                    .add_modifier(Modifier::BOLD),
            ),
            morse_area,
        );
    }
}

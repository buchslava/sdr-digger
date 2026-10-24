use ratatui::layout::{Constraint, Flex, Layout, Rect};

pub fn centered_dialog_rect(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(2));
    let h = height.min(area.height.saturating_sub(2));
    let [rect] = Layout::horizontal([Constraint::Length(w)])
        .flex(Flex::Center)
        .areas(area);
    let [rect] = Layout::vertical([Constraint::Length(h)])
        .flex(Flex::Center)
        .areas(rect);
    rect
}

pub fn paint_dim(f: &mut ratatui::Frame, area: Rect) {
    use ratatui::style::{Color, Style};
    use ratatui::widgets::Block;
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(0, 0, 0))),
        area,
    );
}

use ratatui::style::{Color, Style};

#[derive(Debug, Clone)]
pub struct UiPalette {
    pub accent: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub text: Color,
    pub dim: Color,
    pub trace: Color,
    pub center_line: Color,
    pub coach_bg: Color,
    pub alert: Color,
    pub ok: Color,
}

impl UiPalette {
    pub fn digger() -> Self {
        Self {
            accent: Color::Rgb(80, 160, 255),
            header_bg: Color::Rgb(20, 24, 40),
            header_fg: Color::Rgb(200, 210, 230),
            text: Color::Rgb(220, 225, 235),
            dim: Color::Rgb(120, 130, 150),
            trace: Color::Rgb(80, 200, 255),
            center_line: Color::Rgb(255, 80, 80),
            coach_bg: Color::Rgb(18, 22, 36),
            alert: Color::Rgb(255, 100, 100),
            ok: Color::Rgb(100, 220, 140),
        }
    }

    pub fn title_style(&self) -> Style {
        Style::default().fg(self.header_fg).bg(self.header_bg)
    }

    pub fn text_style(&self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn dim_style(&self) -> Style {
        Style::default().fg(self.dim)
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }
}

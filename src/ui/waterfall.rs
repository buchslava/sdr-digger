use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

use crate::app::AppState;
use crate::dsp::FFT_SIZE;
use crate::i18n::{t, Msg};
use crate::ui::theme::UiPalette;

pub fn draw(
    frame: &mut Frame,
    title_area: Rect,
    body: Rect,
    app: &AppState,
    palette: &UiPalette,
) {
    frame.render_widget(
        Paragraph::new(format!(" {} ", t(app.lang, Msg::WaterfallTitle)))
            .centered()
            .style(palette.title_style().add_modifier(Modifier::BOLD)),
        title_area,
    );

    let widget = WaterfallWidget {
        app,
        y_min: app.y_min as f32,
        y_max: app.y_max as f32,
    };
    frame.render_widget(widget, body);
}

struct WaterfallWidget<'a> {
    app: &'a AppState,
    y_min: f32,
    y_max: f32,
}

impl Widget for WaterfallWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let rows = area.height as usize;
        let cols = area.width as usize;
        for row in 0..rows {
            let Some(data) = self.app.waterfall.get(row) else {
                break;
            };
            let down = downsample_row(data, cols, FFT_SIZE / 2);
            for (col, &mag) in down.iter().enumerate() {
                let color = magnitude_to_rgb(mag, self.y_min, self.y_max);
                let x = area.x + col as u16;
                let y = area.y + row as u16;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol("▀");
                    cell.set_style(Style::default().fg(color));
                }
            }
        }
    }
}

fn downsample_row(data: &[f32], target_width: usize, center_bin: usize) -> Vec<f32> {
    if target_width == 0 || data.is_empty() {
        return Vec::new();
    }
    let n = data.len() as isize;
    let chunk_size = (data.len() / target_width).max(1);
    let half = (target_width / 2) as isize;
    let cs = chunk_size as isize;
    let start = center_bin as isize - half * cs;
    (0..target_width)
        .map(|i| {
            let cstart = start + (i as isize) * cs;
            let cend = cstart + cs;
            let lo = cstart.max(0).min(n) as usize;
            let hi = cend.max(0).min(n) as usize;
            if lo >= hi {
                f32::NEG_INFINITY
            } else {
                data[lo..hi]
                    .iter()
                    .cloned()
                    .fold(f32::NEG_INFINITY, f32::max)
            }
        })
        .collect()
}

fn magnitude_to_rgb(mag: f32, min_db: f32, max_db: f32) -> Color {
    let range = max_db - min_db;
    let t = if range <= 0.0 {
        0.0
    } else {
        ((mag - min_db) / range).clamp(0.0, 1.0)
    };
    let (r, g, b) = if t < 0.25 {
        let s = t / 0.25;
        (0.0, s, 1.0)
    } else if t < 0.5 {
        let s = (t - 0.25) / 0.25;
        (0.0, 1.0, 1.0 - s)
    } else if t < 0.75 {
        let s = (t - 0.5) / 0.25;
        (s, 1.0, 0.0)
    } else {
        let s = (t - 0.75) / 0.25;
        (1.0, 1.0 - s, 0.0)
    };
    Color::Rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

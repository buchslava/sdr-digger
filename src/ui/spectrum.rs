use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph};
use ratatui::Frame;

use crate::app::AppState;
use crate::dsp::FFT_SIZE;
use crate::i18n::{t, Msg};
use crate::ui::theme::UiPalette;
use crate::ui::util::{column_freq_hz, downsample, format_mhz};

pub fn draw(
    frame: &mut Frame,
    title_area: Rect,
    chart_area: Rect,
    app: &AppState,
    palette: &UiPalette,
) {
    frame.render_widget(
        Paragraph::new(format!(" {} ", t(app.lang, Msg::SpectrumTitle)))
            .centered()
            .style(palette.title_style().add_modifier(Modifier::BOLD)),
        title_area,
    );

    let chart_body = Rect {
        height: chart_area.height.saturating_sub(1),
        ..chart_area
    };
    let ruler_row = Rect {
        x: chart_area.x,
        y: chart_area.y + chart_area.height.saturating_sub(1),
        width: chart_area.width,
        height: 1,
    };

    let y_label_width = 4u16;
    let plot_cols = chart_body.width.saturating_sub(y_label_width + 1) as usize;
    let center_hz = app.case.tune.frequency_hz as f64;
    let sample_rate = app.case.window.sample_rate_hz as f64;
    let points = downsample(
        &app.spectrum,
        plot_cols,
        FFT_SIZE / 2,
        center_hz,
        sample_rate,
    );
    let (x_min, x_max) = match (points.first(), points.last()) {
        (Some((a, _)), Some((b, _))) if b > a => (*a, *b),
        _ => (center_hz / 1e6 - 0.5, center_hz / 1e6 + 0.5),
    };

    let dataset = Dataset::default()
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(palette.trace))
        .data(&points);

    let y_mid = (app.y_min + app.y_max) / 2.0;
    let chart = Chart::new(vec![dataset])
        .x_axis(
            Axis::default()
                .bounds([x_min, x_max])
                .style(Style::default().fg(palette.dim)),
        )
        .y_axis(
            Axis::default()
                .bounds([app.y_min, app.y_max])
                .labels(vec![
                    format!("{:.0}", app.y_min),
                    format!("{y_mid:.0}"),
                    format!("{:.0}", app.y_max),
                ])
                .style(Style::default().fg(palette.dim)),
        );

    frame.render_widget(chart, chart_body);

    draw_freq_ruler(
        frame,
        ruler_row,
        y_label_width + 1,
        plot_cols,
        center_hz,
        sample_rate,
        palette,
    );

    if app.show_overlay {
        let chart_inner = Rect {
            x: chart_body.x + y_label_width + 1,
            y: chart_body.y,
            width: plot_cols as u16,
            height: chart_body.height,
        };
        if chart_inner.width > 0 {
            let cx = chart_inner.x + chart_inner.width / 2;
            for y in chart_inner.y..chart_inner.y + chart_inner.height {
                if let Some(cell) = frame.buffer_mut().cell_mut((cx, y)) {
                    cell.set_char('│');
                    cell.set_style(Style::default().fg(palette.center_line));
                }
            }
        }
    }
}

fn draw_freq_ruler(
    frame: &mut Frame,
    area: Rect,
    x_offset: u16,
    plot_cols: usize,
    center_hz: f64,
    sample_rate: f64,
    palette: &UiPalette,
) {
    if plot_cols < 8 || area.height == 0 || area.width <= x_offset {
        return;
    }
    let plot_w = plot_cols as u16;
    let origin = area.x + x_offset;
    let ticks = 5usize;
    let style = Style::default().fg(palette.dim);
    for i in 0..ticks {
        let col = if ticks == 1 {
            0
        } else {
            i * (plot_cols - 1) / (ticks - 1)
        };
        let hz = column_freq_hz(
            col,
            FFT_SIZE,
            plot_cols,
            FFT_SIZE / 2,
            center_hz,
            sample_rate,
        );
        let label = format_mhz(hz);
        let w = label.chars().count() as u16;
        let mut x = origin + col as u16;
        x = x.saturating_sub(w / 2);
        if x < origin {
            x = origin;
        }
        let right = origin + plot_w;
        if x + w > right {
            x = right.saturating_sub(w);
        }
        if x + w > area.x + area.width {
            continue;
        }
        frame.render_widget(
            Paragraph::new(label).style(style),
            Rect {
                x,
                y: area.y,
                width: w,
                height: 1,
            },
        );
    }
}

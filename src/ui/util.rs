/// Chunk layout used by the spectrum plot: DC (`center_bin`) sits in the
/// geometric middle, and each column covers `chunk` FFT bins.
pub fn chunk_layout(len: usize, target_width: usize, center_bin: usize) -> (isize, isize) {
    let chunk = (len / target_width.max(1)).max(1) as isize;
    let half = (target_width / 2) as isize;
    let start = center_bin as isize - half * chunk;
    (start, chunk)
}

/// RF frequency (Hz) represented by plot column `col`.
/// The middle column is exactly `center_hz` (the tuned dial), and each
/// column steps by one FFT-chunk of bandwidth.
pub fn column_freq_hz(
    col: usize,
    data_len: usize,
    target_width: usize,
    _center_bin: usize,
    center_hz: f64,
    sample_rate_hz: f64,
) -> f64 {
    let (_, chunk) = chunk_layout(data_len.max(1), target_width.max(1), 0);
    let df = if data_len == 0 {
        0.0
    } else {
        sample_rate_hz / data_len as f64
    };
    let mid = target_width / 2;
    let bins = col as isize - mid as isize;
    center_hz + bins as f64 * chunk as f64 * df
}

/// Downsample spectrum with DC pinned to the geometric middle.
/// X values are frequency in MHz so the chart axis matches the air.
pub fn downsample(
    data: &[f32],
    target_width: usize,
    center_bin: usize,
    center_hz: f64,
    sample_rate_hz: f64,
) -> Vec<(f64, f64)> {
    if target_width == 0 || data.is_empty() {
        return Vec::new();
    }
    let n = data.len() as isize;
    let (start, cs) = chunk_layout(data.len(), target_width, center_bin);

    (0..target_width)
        .map(|i| {
            let cstart = start + (i as isize) * cs;
            let cend = cstart + cs;
            let lo = cstart.max(0).min(n) as usize;
            let hi = cend.max(0).min(n) as usize;
            let v = if lo >= hi {
                f64::NEG_INFINITY
            } else {
                data[lo..hi]
                    .iter()
                    .cloned()
                    .fold(f32::NEG_INFINITY, f32::max) as f64
            };
            let mhz = column_freq_hz(
                i,
                data.len(),
                target_width,
                center_bin,
                center_hz,
                sample_rate_hz,
            ) / 1e6;
            (mhz, v)
        })
        .collect()
}

/// Compact frequency label. Wide windows use MHz; narrow ones keep more digits.
pub fn format_mhz(hz: f64) -> String {
    let mhz = hz / 1e6;
    if mhz.abs() >= 100.0 {
        format!("{mhz:.3}")
    } else if mhz.abs() >= 10.0 {
        format!("{mhz:.4}")
    } else {
        format!("{mhz:.5}")
    }
}

#[cfg(test)]
mod tests {
    use super::{column_freq_hz, format_mhz};

    #[test]
    fn centre_column_is_the_tuned_frequency() {
        let center = 101_500_000.0;
        let sr = 1_024_000.0;
        let cols = 80;
        let hz = column_freq_hz(cols / 2, 1024, cols, 512, center, sr);
        assert!((hz - center).abs() < 1.0, "{hz}");
        assert!(format_mhz(center).starts_with("101.5"));
    }
}

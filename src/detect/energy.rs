use super::cfar::{measure_channel, ChannelStat};
use super::plan::ScanPlan;
use super::spectrum::{cmp_f32, BandSpectrum};
use super::Hit;
use crate::radio::Family;

/// Peak energy finder for digital-watch / lab window mode.
pub fn detect(spectrum: &BandSpectrum, plan: &ScanPlan, family: Family) -> Vec<Hit> {
    if spectrum.power_db.is_empty() {
        return vec![];
    }
    // Coarse grid every 50 kHz across the spectrum span.
    let step = 50_000u64;
    let start = spectrum.start_hz.round() as u64;
    let end = (spectrum.start_hz + spectrum.bin_hz * (spectrum.power_db.len() as f64)) as u64;
    let mut stats: Vec<ChannelStat> = Vec::new();
    let mut f = start;
    while f <= end {
        stats.push(measure_channel(spectrum, f, plan));
        f = f.saturating_add(step);
    }
    stats.sort_by(|a, b| cmp_f32(b.snr_db, a.snr_db));
    stats
        .into_iter()
        .filter(|s| s.eligible(plan))
        .take(10)
        .map(|s| Hit {
            frequency_hz: s.frequency_hz,
            snr_db: s.snr_db,
            occupancy: s.occupancy,
            bandwidth_hz: s.bandwidth_hz,
            label: None,
            family,
        })
        .collect()
}

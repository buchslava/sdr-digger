//! Shared OS-CFAR + radiometer + equivalent rectangular bandwidth.

use super::plan::ScanPlan;
use super::spectrum::{percentile, BandSpectrum, GRID_HZ};

#[derive(Debug, Clone, Copy)]
pub struct ChannelStat {
    pub frequency_hz: u64,
    pub snr_db: f32,
    pub occupancy: f32,
    pub bandwidth_hz: f32,
}

impl ChannelStat {
    pub fn rejected(frequency_hz: u64) -> Self {
        Self {
            frequency_hz,
            snr_db: f32::NEG_INFINITY,
            occupancy: 0.0,
            bandwidth_hz: 0.0,
        }
    }

    pub fn eligible(self, plan: &ScanPlan) -> bool {
        if plan.needle {
            // Needle: high SNR, low occupancy / narrow Beq.
            self.snr_db >= plan.snr_threshold_db
                && self.bandwidth_hz >= plan.min_beq_hz
                && self.bandwidth_hz <= plan.max_beq_hz
                && self.occupancy <= 0.35
        } else {
            self.snr_db >= plan.snr_threshold_db
                && self.occupancy >= plan.min_occupancy
                && self.bandwidth_hz >= plan.min_beq_hz
                && self.bandwidth_hz <= plan.max_beq_hz
        }
    }
}

const OCCUPANCY_BIN_DB: f32 = 3.0;
const NOISE_PERCENTILE: f32 = 0.25;

pub fn measure_channel(spectrum: &BandSpectrum, frequency_hz: u64, plan: &ScanPlan) -> ChannelStat {
    let center_hz = frequency_hz as f64;
    let Some(center_idx) = spectrum.index_of(center_hz) else {
        return ChannelStat::rejected(frequency_hz);
    };
    if (spectrum.freq_hz(center_idx) - center_hz).abs() > spectrum.bin_hz {
        return ChannelStat::rejected(frequency_hz);
    }

    let test_radius = bins_for(plan.test_half_hz);
    let guard = bins_for(plan.guard_hz);
    let train = bins_for(plan.train_hz);

    let lo = center_idx.saturating_sub(test_radius);
    let hi = (center_idx + test_radius + 1).min(spectrum.power_db.len());
    if hi <= lo {
        return ChannelStat::rejected(frequency_hz);
    }

    let test = &spectrum.power_db[lo..hi];
    let noise = os_cfar_noise(&spectrum.power_db, center_idx, guard, train);

    let mut occupied = 0usize;
    let mut sum_lin = 0.0f32;
    let mut sum_excess = 0.0f32;
    let mut sum_excess2 = 0.0f32;
    for &p in test {
        let excess = p - noise;
        let lin = 10.0f32.powf(excess / 10.0);
        sum_lin += lin;
        if excess >= OCCUPANCY_BIN_DB {
            occupied += 1;
        }
        let excess_lin = (lin - 1.0).max(0.0);
        sum_excess += excess_lin;
        sum_excess2 += excess_lin * excess_lin;
    }

    let snr_db = 10.0 * (sum_lin / test.len() as f32).max(1e-12).log10();
    let occupancy = occupied as f32 / test.len() as f32;
    let bandwidth_hz = if sum_excess2 > 1e-12 {
        GRID_HZ as f32 * (sum_excess * sum_excess) / sum_excess2
    } else {
        0.0
    };

    ChannelStat {
        frequency_hz,
        snr_db,
        occupancy,
        bandwidth_hz,
    }
}

fn bins_for(hz: f64) -> usize {
    (hz / GRID_HZ).round().max(1.0) as usize
}

fn os_cfar_noise(psd_db: &[f32], center: usize, guard: usize, train: usize) -> f32 {
    let n = psd_db.len();
    let mut cells = Vec::with_capacity(train.saturating_mul(2));
    if center > guard {
        let left_end = center - guard;
        let left_start = left_end.saturating_sub(train);
        cells.extend_from_slice(&psd_db[left_start..left_end]);
    }
    let right_start = (center + guard).min(n);
    let right_end = (right_start + train).min(n);
    if right_start < right_end {
        cells.extend_from_slice(&psd_db[right_start..right_end]);
    }
    if cells.len() < 8 {
        return percentile(psd_db, NOISE_PERCENTILE);
    }
    percentile(&cells, NOISE_PERCENTILE)
}

/// Measure every raster channel in the plan band.
pub fn measure_raster(spectrum: &BandSpectrum, plan: &ScanPlan) -> Vec<ChannelStat> {
    let step = plan.raster_hz.max(1);
    let mut out = Vec::new();
    let mut f = plan.band_start_hz;
    while f <= plan.band_end_hz {
        out.push(measure_channel(spectrum, f, plan));
        f = f.saturating_add(step);
    }
    out
}

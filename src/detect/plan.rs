use serde::{Deserialize, Serialize};

use crate::radio::{CaseFile, DetectMode, DetectShape, Family};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanPhase {
    Spectrum,
    Identify,
    Done,
}

#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub phase: ScanPhase,
    pub current: u32,
    pub total: u32,
    pub hz: f64,
}

#[derive(Debug, Clone)]
pub struct ScanPlan {
    pub mode: DetectMode,
    pub shape: DetectShape,
    pub family: Family,
    pub band_start_hz: u64,
    pub band_end_hz: u64,
    pub raster_hz: u64,
    pub sample_rate_hz: u32,
    /// Shape thresholds (Hz / unitless).
    pub test_half_hz: f64,
    pub guard_hz: f64,
    pub train_hz: f64,
    pub snr_threshold_db: f32,
    pub min_occupancy: f32,
    pub min_beq_hz: f32,
    pub max_beq_hz: f32,
    pub min_sep_hz: u64,
    pub needle: bool,
}

impl ScanPlan {
    pub fn from_case(case: &CaseFile) -> Self {
        let mode = case.detect.mode;
        let shape = case.detect.shape;
        let (band_start, band_end) = match (case.detect.band_start_hz, case.detect.band_end_hz) {
            (Some(a), Some(b)) => (a, b),
            _ => match case.family {
                Family::BroadcastFm => (87_500_000, 108_000_000),
                Family::Airband => (118_000_000, 136_975_000),
                Family::Marine => (156_000_000, 162_550_000),
                Family::WeatherSat => (137_000_000, 138_000_000),
                Family::AmateurFm => (144_000_000, 146_000_000),
                Family::AmateurSsb => (28_300_000, 28_500_000),
                Family::AmateurCw => (28_000_000, 28_070_000),
                Family::Digital => (
                    case.tune.frequency_hz.saturating_sub(1_000_000),
                    case.tune.frequency_hz.saturating_add(1_000_000),
                ),
                Family::Lab => (
                    case.tune
                        .frequency_hz
                        .saturating_sub(case.window.sample_rate_hz as u64 / 2),
                    case.tune
                        .frequency_hz
                        .saturating_add(case.window.sample_rate_hz as u64 / 2),
                ),
            },
        };
        let raster = case
            .detect
            .raster_hz
            .unwrap_or_else(|| case.family.default_raster_hz())
            .max(if matches!(shape, DetectShape::CwNeedle) {
                100
            } else {
                0
            });

        let (
            test_half_hz,
            guard_hz,
            train_hz,
            snr_threshold_db,
            min_occupancy,
            min_beq_hz,
            max_beq_hz,
            min_sep_hz,
            needle,
        ) = match shape {
            DetectShape::WbfmHaystack => (
                90_000.0, 120_000.0, 800_000.0, 4.5, 0.30, 55_000.0, 250_000.0, 250_000, false,
            ),
            DetectShape::Nbfm => (
                6_250.0, 15_000.0, 100_000.0, 4.0, 0.25, 6_000.0, 20_000.0, 25_000, false,
            ),
            DetectShape::Apt => (
                20_000.0, 30_000.0, 150_000.0, 3.5, 0.20, 20_000.0, 50_000.0, 50_000, false,
            ),
            DetectShape::Am => (
                5_000.0, 12_000.0, 80_000.0, 4.0, 0.20, 4_000.0, 15_000.0, 25_000, false,
            ),
            DetectShape::CwNeedle => (
                400.0, 1_000.0, 20_000.0, 6.0, 0.0, 100.0, 800.0, 200, true,
            ),
            DetectShape::Energy => (
                50_000.0, 80_000.0, 400_000.0, 5.0, 0.05, 1_000.0, 2_000_000.0, 100_000, false,
            ),
        };

        Self {
            mode,
            shape,
            family: case.family,
            band_start_hz: band_start,
            band_end_hz: band_end.max(band_start + 1),
            raster_hz: if raster == 0 { 25_000 } else { raster },
            sample_rate_hz: if case.window.sample_rate_hz >= 2_000_000 {
                2_048_000
            } else {
                1_024_000
            },
            test_half_hz,
            guard_hz,
            train_hz,
            snr_threshold_db,
            min_occupancy,
            min_beq_hz,
            max_beq_hz,
            min_sep_hz,
            needle,
        }
    }
}

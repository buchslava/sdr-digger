use crate::i18n::{t, Lang, Msg};

use super::case::Recipe;
use super::params::RadioMode;

/// RTL2832 valid bands: 225_001–300_000 and 900_001–3_200_000 Hz.
pub const RTL_SAMPLE_RATES: &[u32] = &[
    250_000, 256_000, 1_024_000, 1_536_000, 1_792_000, 1_920_000, 2_048_000, 2_400_000, 2_560_000,
];

pub const RTL_PREFERRED_RATES: &[u32] = &[
    256_000, 1_024_000, 1_536_000, 1_792_000, 1_920_000, 2_048_000, 2_160_000, 2_560_000,
];

const RTL_MIN: u32 = 225_001;
const RTL_MAX: u32 = 3_200_000;

pub fn is_rtlsdr_valid_sample_rate(rate: u32) -> bool {
    if !(RTL_MIN..=RTL_MAX).contains(&rate) {
        return false;
    }
    !(300_000 < rate && rate <= 900_000)
}

pub fn nearest_preferred_rate(requested: u32) -> u32 {
    RTL_PREFERRED_RATES
        .iter()
        .min_by_key(|&&rate| rate.abs_diff(requested))
        .copied()
        .unwrap_or(1_024_000)
}

/// Apply crystal PPM: `device_hz = freq * (1 + ppm / 1e6)`.
pub fn apply_ppm(freq_hz: u64, ppm: f64) -> u64 {
    let corrected = (freq_hz as f64) * (1.0 + ppm / 1_000_000.0);
    corrected.round().clamp(0.0, u64::MAX as f64) as u64
}

pub fn snap_to_raster(freq_hz: u64, raster_hz: u64) -> u64 {
    if raster_hz == 0 {
        return freq_hz;
    }
    let r = raster_hz as f64;
    ((freq_hz as f64 / r).round() * r) as u64
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintIssue {
    pub msg: Msg,
}

/// Validate a recipe. Returns human-facing issues (via Msg) for the coach.
pub fn check_recipe(recipe: &Recipe) -> Vec<ConstraintIssue> {
    let mut issues = Vec::new();

    if !is_rtlsdr_valid_sample_rate(recipe.window.sample_rate_hz) {
        issues.push(ConstraintIssue {
            msg: Msg::ConstraintRtlRate,
        });
    }

    let usable = (recipe.window.sample_rate_hz as f64) * 0.80 / 2.0;
    if recipe.window.channel_bw_hz as f64 > usable && recipe.voice.mode != RadioMode::Off {
        issues.push(ConstraintIssue {
            msg: Msg::ConstraintNyquist,
        });
    }

    if recipe.voice.mode == RadioMode::Wbfm && recipe.window.channel_bw_hz < 150_000 {
        issues.push(ConstraintIssue {
            msg: Msg::ConstraintCarson,
        });
    }

    if recipe.voice.mode == RadioMode::Wbfm && recipe.window.sample_rate_hz < 900_000 {
        issues.push(ConstraintIssue {
            msg: Msg::ConstraintCarson,
        });
    }

    issues
}

/// Format constraint messages for the coach panel.
pub fn format_issues(lang: Lang, issues: &[ConstraintIssue]) -> Vec<String> {
    issues
        .iter()
        .map(|i| t(lang, i.msg).to_string())
        .collect()
}

/// R820T-style gain steps in tenths of a dB (from sdrrat).
pub fn rtl_gain_tenths() -> &'static [i32] {
    &[
        0, 9, 14, 27, 37, 77, 87, 125, 144, 157, 166, 197, 207, 229, 254, 280, 297, 328, 338, 364,
        372, 386, 402, 421, 434, 439, 445, 480, 496,
    ]
}

pub fn nearest_gain_tenths(gain_db: f64) -> i32 {
    let want = (gain_db * 10.0).round() as i32;
    *rtl_gain_tenths()
        .iter()
        .min_by_key(|&&g| (g - want).unsigned_abs())
        .unwrap_or(&364)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtl_dead_band_rejected() {
        assert!(!is_rtlsdr_valid_sample_rate(768_000));
        assert!(is_rtlsdr_valid_sample_rate(256_000));
        assert!(is_rtlsdr_valid_sample_rate(1_024_000));
    }

    #[test]
    fn ppm_moves_frequency() {
        let f = apply_ppm(100_000_000, 50.0);
        assert_eq!(f, 100_005_000);
    }

    #[test]
    fn raster_snap() {
        assert_eq!(snap_to_raster(101_537_000, 100_000), 101_500_000);
    }
}

use super::cfar::measure_raster;
use super::cluster::cluster_hits;
use super::plan::ScanPlan;
use super::spectrum::BandSpectrum;
use super::Hit;
use crate::radio::Family;

pub fn detect(spectrum: &BandSpectrum, plan: &ScanPlan, family: Family) -> Vec<Hit> {
    let stats = measure_raster(spectrum, plan);
    cluster_hits(&stats, plan, family)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::spectrum::{paint_haystack, BandSpectrum};
    use crate::radio::{DetectMode, DetectShape};

    fn fm_plan() -> ScanPlan {
        ScanPlan {
            mode: DetectMode::Hop,
            shape: DetectShape::WbfmHaystack,
            family: Family::BroadcastFm,
            band_start_hz: 87_500_000,
            band_end_hz: 108_000_000,
            raster_hz: 100_000,
            sample_rate_hz: 2_048_000,
            test_half_hz: 90_000.0,
            guard_hz: 120_000.0,
            train_hz: 800_000.0,
            snr_threshold_db: 4.5,
            min_occupancy: 0.30,
            min_beq_hz: 55_000.0,
            max_beq_hz: 250_000.0,
            min_sep_hz: 250_000,
            needle: false,
        }
    }

    #[test]
    fn two_haystacks() {
        let plan = fm_plan();
        let mut spec = BandSpectrum::empty_floor(87_500_000.0, 108_000_000.0, -80.0);
        paint_haystack(&mut spec, 88_000_000.0, -55.0, -80.0, 60_000.0, 100_000.0);
        paint_haystack(&mut spec, 105_200_000.0, -52.0, -80.0, 60_000.0, 100_000.0);
        let hits = detect(&spec, &plan, Family::BroadcastFm);
        let freqs: Vec<u64> = hits.iter().map(|h| h.frequency_hz).collect();
        assert!(freqs.contains(&88_000_000), "{freqs:?}");
        assert!(freqs.contains(&105_200_000), "{freqs:?}");
    }

    #[test]
    fn noise_only() {
        let plan = fm_plan();
        let spec = BandSpectrum::empty_floor(87_500_000.0, 108_000_000.0, -80.0);
        assert!(detect(&spec, &plan, Family::BroadcastFm).is_empty());
    }
}

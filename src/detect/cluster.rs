use super::cfar::ChannelStat;
use super::plan::ScanPlan;
use super::spectrum::cmp_f32;
use super::Hit;
use crate::radio::Family;

const VALLEY_DB: f32 = 6.0;
const MAX_CHANNELS: usize = 50;

pub fn cluster_hits(stats: &[ChannelStat], plan: &ScanPlan, family: Family) -> Vec<Hit> {
    let eligible: Vec<bool> = stats.iter().map(|s| s.eligible(plan)).collect();
    let peaks = best_per_blob(stats, &eligible);
    let merged = merge_spaced(peaks, plan.min_sep_hz, MAX_CHANNELS);
    merged
        .into_iter()
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

fn best_per_blob(stats: &[ChannelStat], eligible: &[bool]) -> Vec<ChannelStat> {
    let mut peaks = Vec::new();
    let mut i = 0;
    while i < stats.len() {
        if !eligible[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < stats.len() && eligible[i] {
            i += 1;
        }
        for (lo, hi) in split_valleys(stats, start, i) {
            if let Some(best) = argmax_quality(stats, lo, hi) {
                peaks.push(stats[best]);
            }
        }
    }
    peaks
}

fn split_valleys(stats: &[ChannelStat], start: usize, end: usize) -> Vec<(usize, usize)> {
    if end <= start + 2 {
        return vec![(start, end)];
    }
    let mut maxima = Vec::new();
    for i in start..end {
        let left = if i == start {
            f32::NEG_INFINITY
        } else {
            stats[i - 1].snr_db
        };
        let right = if i + 1 == end {
            f32::NEG_INFINITY
        } else {
            stats[i + 1].snr_db
        };
        if stats[i].snr_db >= left && stats[i].snr_db >= right {
            maxima.push(i);
        }
    }
    if maxima.len() <= 1 {
        return vec![(start, end)];
    }
    let mut segments = Vec::new();
    let mut seg_start = start;
    for pair in maxima.windows(2) {
        let left_peak = pair[0];
        let right_peak = pair[1];
        let Some((valley, valley_snr)) = (left_peak..=right_peak)
            .map(|i| (i, stats[i].snr_db))
            .min_by(|a, b| cmp_f32(a.1, b.1))
        else {
            continue;
        };
        let weaker = stats[left_peak].snr_db.min(stats[right_peak].snr_db);
        if weaker - valley_snr >= VALLEY_DB && valley > seg_start && valley + 1 < end {
            segments.push((seg_start, valley));
            seg_start = valley + 1;
        }
    }
    if seg_start < end {
        segments.push((seg_start, end));
    }
    if segments.is_empty() {
        vec![(start, end)]
    } else {
        segments
    }
}

fn argmax_quality(stats: &[ChannelStat], start: usize, end: usize) -> Option<usize> {
    (start..end).max_by(|&a, &b| {
        cmp_f32(stats[a].snr_db, stats[b].snr_db)
            .then_with(|| cmp_f32(stats[a].occupancy, stats[b].occupancy))
    })
}

fn merge_spaced(mut peaks: Vec<ChannelStat>, min_sep_hz: u64, max_peaks: usize) -> Vec<ChannelStat> {
    peaks.sort_by(|a, b| cmp_f32(b.snr_db, a.snr_db));
    let mut selected = Vec::with_capacity(peaks.len().min(max_peaks));
    for peak in peaks {
        if selected
            .iter()
            .any(|s: &ChannelStat| s.frequency_hz.abs_diff(peak.frequency_hz) < min_sep_hz)
        {
            continue;
        }
        selected.push(peak);
        if selected.len() >= max_peaks {
            break;
        }
    }
    selected.sort_by_key(|s| s.frequency_hz);
    selected
}

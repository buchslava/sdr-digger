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

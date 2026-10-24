mod case;
mod catalog;
mod coach;
mod constraints;
mod family;
mod params;
mod persist;

pub use case::{Bookmark, CaseFile, DetectIdentify, DetectMode, DetectShape};
pub use catalog::{ensure_user_dir, list_cases, load_case, save_case};
pub use coach::{build_coach_lines, CoachContext};
pub use constraints::{
    apply_ppm, check_recipe, nearest_gain_tenths, rtl_gain_tenths, snap_to_raster,
    RTL_SAMPLE_RATES,
};
pub use family::{band_label_msg, Family};
pub use params::{GainMode, RadioMode};
pub use persist::{configure_linux_audio_env, load_settings, save_settings, AppSettings};

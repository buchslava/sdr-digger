use serde::{Deserialize, Serialize};

use crate::i18n::Msg;

use super::params::RadioMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    BroadcastFm,
    Airband,
    Marine,
    WeatherSat,
    AmateurFm,
    AmateurSsb,
    AmateurCw,
    Digital,
    Lab,
}

impl Family {
    pub const ALL: &'static [Family] = &[
        Family::BroadcastFm,
        Family::Airband,
        Family::Marine,
        Family::WeatherSat,
        Family::AmateurFm,
        Family::AmateurSsb,
        Family::AmateurCw,
        Family::Digital,
        Family::Lab,
    ];

    pub fn msg(self) -> Msg {
        match self {
            Family::BroadcastFm => Msg::FamilyBroadcastFm,
            Family::Airband => Msg::FamilyAirband,
            Family::Marine => Msg::FamilyMarine,
            Family::WeatherSat => Msg::FamilyWeatherSat,
            Family::AmateurFm => Msg::FamilyAmateurFm,
            Family::AmateurSsb => Msg::FamilyAmateurSsb,
            Family::AmateurCw => Msg::FamilyAmateurCw,
            Family::Digital => Msg::FamilyDigital,
            Family::Lab => Msg::FamilyLab,
        }
    }

    /// Fine-tune step after a detector hit (Hz).
    pub fn fine_tune_hz(self) -> u64 {
        match self {
            Family::BroadcastFm => 50_000,
            Family::Airband => 8_330,
            Family::Marine | Family::AmateurFm => 1_000,
            Family::WeatherSat => 100,
            Family::AmateurSsb => 50,
            Family::AmateurCw => 10,
            Family::Digital => 25_000,
            Family::Lab => 1_000,
        }
    }

    pub fn default_mode(self) -> RadioMode {
        match self {
            Family::BroadcastFm => RadioMode::Wbfm,
            Family::Airband => RadioMode::Am,
            Family::Marine | Family::WeatherSat | Family::AmateurFm => RadioMode::Nbfm,
            Family::AmateurSsb => RadioMode::Usb,
            Family::AmateurCw => RadioMode::Cw,
            Family::Digital => RadioMode::Off,
            Family::Lab => RadioMode::Off,
        }
    }

    pub fn default_raster_hz(self) -> u64 {
        match self {
            Family::BroadcastFm => 100_000,
            Family::Airband => 25_000,
            Family::Marine | Family::AmateurFm => 12_500,
            Family::AmateurSsb | Family::AmateurCw => 0,
            Family::WeatherSat | Family::Digital | Family::Lab => 0,
        }
    }
}

/// Band label message for the current frequency / family.
pub fn band_label_msg(family: Family, freq_hz: u64) -> Msg {
    match family {
        Family::BroadcastFm => Msg::BandVhfIi,
        Family::Airband => Msg::BandAirband,
        Family::Marine => Msg::BandMarine,
        Family::WeatherSat => Msg::BandNoaaApt,
        Family::AmateurFm => {
            if (430_000_000..=440_000_000).contains(&freq_hz) {
                Msg::BandHam70cm
            } else {
                Msg::BandHam2m
            }
        }
        Family::AmateurSsb | Family::AmateurCw => {
            if freq_hz < 30_000_000 {
                Msg::BandHf
            } else if (144_000_000..=148_000_000).contains(&freq_hz) {
                Msg::BandHam2m
            } else {
                Msg::BandCustom
            }
        }
        Family::Digital | Family::Lab => Msg::BandCustom,
    }
}

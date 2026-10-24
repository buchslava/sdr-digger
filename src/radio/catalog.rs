use std::fs;
use std::path::PathBuf;

use super::case::CaseFile;
use super::persist::{config_dir, CASES_SUBDIR};

/// Bundled case slug + TOML source.
pub struct BundledCase {
    pub slug: &'static str,
    pub toml: &'static str,
}

macro_rules! bundled {
    ($($slug:literal),* $(,)?) => {
        pub const BUNDLED: &[BundledCase] = &[
            $(
                BundledCase {
                    slug: $slug,
                    toml: include_str!(concat!("../../cases/", $slug, ".toml")),
                },
            )*
        ];
    };
}

bundled!(
    "fm-broadcast-r1",
    "airband-am",
    "marine-ch16",
    "cb-27",
    "pmr446",
    "ism-433",
    "noaa-weather",
    "noaa-apt",
    "meteor-lrpt",
    "iss-voice",
    "so-50",
    "ham-2m-fm",
    "ham-2m-ssb",
    "ham-2m-cw",
    "ham-70cm-fm",
    "ham-10m-ssb",
    "ham-10m-cw",
    "uvb-76",
    "adsb-1090",
    "ais-a",
    "spectrum-survey",
    "lab-custom",
);

pub fn user_cases_dir() -> PathBuf {
    config_dir().join(CASES_SUBDIR)
}

/// Ensure `~/.sdr-digger/cases/` exists and seed missing bundled files.
pub fn ensure_user_dir() -> Result<(), String> {
    let dir = user_cases_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for b in BUNDLED {
        let path = dir.join(format!("{}.toml", b.slug));
        if !path.exists() {
            fs::write(&path, b.toml).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn parse_case_toml(raw: &str) -> Result<CaseFile, String> {
    toml::from_str(raw).map_err(|e| e.to_string())
}

pub fn load_case(slug: &str) -> Result<CaseFile, String> {
    let path = user_cases_dir().join(format!("{slug}.toml"));
    if path.exists() {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        return parse_case_toml(&raw);
    }
    for b in BUNDLED {
        if b.slug == slug {
            return parse_case_toml(b.toml);
        }
    }
    Err(format!("case not found: {slug}"))
}

pub fn save_case(case: &CaseFile) -> Result<PathBuf, String> {
    let dir = user_cases_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.toml", case.slug));
    let raw = toml::to_string_pretty(case).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| e.to_string())?;
    Ok(path)
}

/// List cases from user dir (seeded), falling back to bundled parse.
pub fn list_cases() -> Vec<CaseFile> {
    let _ = ensure_user_dir();
    let mut out = Vec::new();
    let dir = user_cases_dir();
    if let Ok(rd) = fs::read_dir(&dir) {
        let mut paths: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("toml"))
            .collect();
        paths.sort();
        for p in paths {
            if let Ok(raw) = fs::read_to_string(&p) {
                if let Ok(c) = parse_case_toml(&raw) {
                    out.push(c);
                }
            }
        }
    }
    if out.is_empty() {
        for b in BUNDLED {
            if let Ok(c) = parse_case_toml(b.toml) {
                out.push(c);
            }
        }
    }
    out
}

pub fn case_path(slug: &str) -> PathBuf {
    user_cases_dir().join(format!("{slug}.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_bundled_parse() {
        for b in BUNDLED {
            let case = parse_case_toml(b.toml).unwrap_or_else(|e| panic!("{}: {e}", b.slug));
            let uk = case
                .coach
                .uk
                .as_ref()
                .unwrap_or_else(|| panic!("{} missing [coach.uk]", b.slug));
            assert!(
                !uk.summary.is_empty(),
                "{} Ukrainian coach summary is empty",
                b.slug
            );
        }
    }
}

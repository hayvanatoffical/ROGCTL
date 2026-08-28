//! Valorant's own frame rate limits, which live outside the driver entirely.
//!
//! Everything else rogctl manages is a driver or firmware lever that applies to
//! every application. Valorant is the exception worth special-casing: it carries
//! four independent frame caps in its own settings file, any one of which can
//! pin the client below the panel, and none of which are visible from the driver
//! profile. A machine can therefore be configured perfectly at the driver level
//! and still be held at 60 by a line in an ini file.
//!
//! This module reads that file, and - only when asked - rewrites the four caps
//! so none of them can bind below the panel's refresh rate.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

/// The settings this module has an opinion about, and nothing else. Crosshair,
/// audio and sensitivity live in the same file and are never touched.
const LIMIT_ALWAYS: &str = "EAresBoolSettingName::LimitFramerateAlways";
const MAX_ALWAYS: &str = "EAresFloatSettingName::MaxFramerateAlways";
const LIMIT_MENU: &str = "EAresBoolSettingName::LimitFramerateInMenu";
const MAX_MENU: &str = "EAresFloatSettingName::MaxFramerateInMenu";
const LIMIT_BACKGROUND: &str = "EAresBoolSettingName::LimitFramerateInBackground";
const LIMIT_BATTERY: &str = "EAresBoolSettingName::LimitFramerateOnBattery";
const MAX_BATTERY: &str = "EAresFloatSettingName::MaxFramerateOnBattery";

/// One account's settings file.
pub struct Settings {
    pub path: PathBuf,
    /// The account folder name, which is the only thing distinguishing one
    /// account's settings from another's.
    pub account: String,
    lines: Vec<String>,
}

/// Processes that mean the file must not be touched.
///
/// Valorant rewrites this file from memory when it exits, so an edit made while
/// it is running is not merely ineffective - it is silently discarded, which
/// looks exactly like the fix not working.
const BLOCKERS: &[&str] = &[
    "VALORANT-Win64-Shipping.exe",
    "VALORANT.exe",
    "RiotClientServices.exe",
];

pub fn blocking_processes() -> Vec<String> {
    let running = crate::process::running_names();
    BLOCKERS
        .iter()
        .filter(|b| {
            running
                .iter()
                .any(|r| r.eq_ignore_ascii_case(b) || r.to_lowercase().starts_with(&b.to_lowercase()[..b.len().min(24)]))
        })
        .map(|s| s.to_string())
        .collect()
}

fn config_root() -> Result<PathBuf> {
    let local = std::env::var("LOCALAPPDATA")
        .map_err(|_| anyhow!("LOCALAPPDATA okunamadi"))?;
    let root = Path::new(&local).join("VALORANT").join("Saved").join("Config");
    if !root.is_dir() {
        return Err(anyhow!("Valorant ayar klasoru bulunamadi: {}", root.display()));
    }
    Ok(root)
}

impl Settings {
    /// The account that played most recently.
    ///
    /// A machine accumulates one folder per account that has ever signed in, and
    /// only one of them is the one being played. The file the game wrote last is
    /// that one; guessing any other way would edit settings nobody is using.
    pub fn newest() -> Result<Self> {
        Self::all()?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("hicbir hesapta RiotUserSettings.ini bulunamadi"))
    }

    /// Every account on the machine, most recently written first.
    ///
    /// The caps are per-account, so fixing the account being played leaves the
    /// same 60 waiting in every other one. Signing into another account is not
    /// an unusual thing to do, and when the frame rate collapses again there is
    /// nothing on screen connecting it to a file nobody remembers editing.
    pub fn all() -> Result<Vec<Self>> {
        let root = config_root()?;
        let mut found: Vec<(std::time::SystemTime, PathBuf, String)> = Vec::new();

        for entry in std::fs::read_dir(&root)? {
            let Ok(entry) = entry else { continue };
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let account = entry.file_name().to_string_lossy().to_string();
            if account.eq_ignore_ascii_case("CrashReportClient") {
                continue;
            }
            let ini = dir.join("Windows").join("RiotUserSettings.ini");
            let Ok(meta) = std::fs::metadata(&ini) else { continue };
            let Ok(modified) = meta.modified() else { continue };
            found.push((modified, ini, account));
        }

        found.sort_by(|a, b| b.0.cmp(&a.0));
        found
            .into_iter()
            .map(|(_, path, account)| Self::load(path, account))
            .collect()
    }

    fn load(path: PathBuf, account: String) -> Result<Self> {
        let text = std::fs::read_to_string(&path)?;
        let lines = text.lines().map(|s| s.to_string()).collect();
        Ok(Self { path, account, lines })
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.lines
            .iter()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix('='))
    }

    /// The caps as they stand, in the order they can bite.
    pub fn caps(&self) -> Vec<(&'static str, &'static str, Option<String>)> {
        let g = |k: &str| self.get(k).map(|s| s.to_string());
        vec![
            ("Her zaman - acik mi", LIMIT_ALWAYS, g(LIMIT_ALWAYS)),
            ("Her zaman - deger ", MAX_ALWAYS, g(MAX_ALWAYS)),
            ("Menude   - acik mi", LIMIT_MENU, g(LIMIT_MENU)),
            ("Menude   - deger  ", MAX_MENU, g(MAX_MENU)),
            ("Arka plan - acik mi", LIMIT_BACKGROUND, g(LIMIT_BACKGROUND)),
            ("Pilde    - acik mi", LIMIT_BATTERY, g(LIMIT_BATTERY)),
            ("Pilde    - deger  ", MAX_BATTERY, g(MAX_BATTERY)),
        ]
    }

    /// Any cap that does not agree with `target`.
    ///
    /// Both directions count. Below target is the obvious fault - it is the 60
    /// that started all this. Above target is a fault too, just a quieter one:
    /// the target already carries whatever margin the panel needs, so a cap
    /// sitting above it is the game overriding that decision. Reporting only
    /// the low side makes `duzelt` look like it changed something it had no
    /// reason to.
    ///
    /// A missing key is reported rather than ignored: absent means the game
    /// falls back to its own default, and a default that is not written down is
    /// a cap nobody can see.
    pub fn offenders(&self, target: u32) -> Vec<String> {
        let mut out = Vec::new();
        let num = |k: &str| self.get(k).and_then(|v| v.parse::<f32>().ok());

        if let Some(v) = num(MAX_MENU) {
            if v < target as f32 {
                out.push(format!(
                    "menu siniri {v:.0} fps - satin alma ekrani gibi durumlarda devreye girer"
                ));
            } else if v > target as f32 {
                out.push(format!("menu siniri {v:.0} fps - hedefin ({target}) ustunde"));
            }
        }
        if let Some(v) = num(MAX_ALWAYS) {
            if v < target as f32 {
                out.push(format!("her zaman siniri {v:.0} fps"));
            } else if v > target as f32 {
                out.push(format!(
                    "her zaman siniri {v:.0} fps - hedefin ({target}) ustunde, \
                     VRR penceresinin disina cikiyor"
                ));
            }
        }
        if self.get(LIMIT_ALWAYS).is_none() {
            out.push(format!(
                "{LIMIT_ALWAYS} dosyada YOK - oyun kendi varsayilanini kullaniyor, \
                 yani sinirin acik mi kapali mi oldugu dosyadan okunamiyor"
            ));
        }
        out
    }

    /// Set a key, adding it if the file does not carry it.
    fn set(&mut self, key: &str, value: &str) -> Option<(String, String)> {
        let line = format!("{key}={value}");
        for l in self.lines.iter_mut() {
            if let Some(rest) = l.strip_prefix(key) {
                if let Some(old) = rest.strip_prefix('=') {
                    let old = old.to_string();
                    if old == value {
                        return None;
                    }
                    *l = line;
                    return Some((old, value.to_string()));
                }
            }
        }
        // Append inside the [Settings] block, which is the whole file.
        self.lines.push(line);
        Some(("(yoktu)".to_string(), value.to_string()))
    }

    /// Rewrite every cap so none of them binds below `target`.
    ///
    /// The battery caps are deliberately left alone: on battery a lower frame
    /// rate is the point, and this machine's battery envelope is a separate
    /// decision that rogctl already makes elsewhere.
    pub fn normalise(&mut self, target: u32) -> Vec<(String, String, String)> {
        let t = format!("{target}");
        let mut changed = Vec::new();
        let mut apply = |key: &str, value: &str, changed: &mut Vec<_>| {
            if let Some((old, new)) = self.set(key, value) {
                changed.push((key.to_string(), old, new));
            }
        };
        // The always-cap is the one that should bind, so it is switched on and
        // set to the panel. Everything else is raised to match, so that if the
        // game decides some state counts as "menu" it still cannot drop.
        apply(LIMIT_ALWAYS, "True", &mut changed);
        apply(MAX_ALWAYS, &t, &mut changed);
        apply(LIMIT_MENU, "False", &mut changed);
        apply(MAX_MENU, &t, &mut changed);
        apply(LIMIT_BACKGROUND, "False", &mut changed);
        changed
    }

    /// Switch every cap off and let the game render as fast as it can.
    ///
    /// The three booleans are what actually bind; the `Max` numbers beside them
    /// are only consulted when their boolean is on, so they are left in place
    /// rather than deleted. That way `duzelt` afterwards has something to go
    /// back to, and the file still records what the caps were.
    ///
    /// The battery cap is again left alone. Uncapped on mains is a choice;
    /// uncapped on a battery that has to last a match is not the same choice,
    /// and nothing here has been asked to make it.
    pub fn uncap(&mut self) -> Vec<(String, String, String)> {
        let mut changed = Vec::new();
        let mut apply = |key: &str, value: &str, changed: &mut Vec<_>| {
            if let Some((old, new)) = self.set(key, value) {
                changed.push((key.to_string(), old, new));
            }
        };
        apply(LIMIT_ALWAYS, "False", &mut changed);
        apply(LIMIT_MENU, "False", &mut changed);
        apply(LIMIT_BACKGROUND, "False", &mut changed);
        changed
    }

    /// True when no cap in this file can bind.
    pub fn is_uncapped(&self) -> bool {
        [LIMIT_ALWAYS, LIMIT_MENU, LIMIT_BACKGROUND]
            .iter()
            .all(|k| self.get(k).is_some_and(|v| v.eq_ignore_ascii_case("False")))
    }

    /// Write the file back, keeping a copy of what was there.
    pub fn save(&self) -> Result<PathBuf> {
        let backup = self.path.with_extension("ini.rogctl-yedek");
        std::fs::copy(&self.path, &backup)?;
        let mut text = self.lines.join("\r\n");
        text.push_str("\r\n");
        std::fs::write(&self.path, text)?;
        Ok(backup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(lines: &[&str]) -> Settings {
        Settings {
            path: PathBuf::from("test.ini"),
            account: "test".to_string(),
            lines: lines.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// The file that started the whole investigation: a menu cap of 60 on a
    /// 144Hz panel, and the switch that would have made it visible missing
    /// entirely.
    #[test]
    fn reports_the_cap_that_was_actually_found() {
        let s = settings(&[
            "EAresFloatSettingName::MaxFramerateInMenu=60",
            "EAresFloatSettingName::MaxFramerateAlways=148",
        ]);
        let bad = s.offenders(141);
        assert!(bad.iter().any(|b| b.contains("60")), "60 sinirini kacirdi: {bad:?}");
        assert!(
            bad.iter().any(|b| b.contains("LimitFramerateAlways")),
            "eksik anahtar bildirilmedi: {bad:?}"
        );
    }

    /// A cap above the target is a fault too. Reporting only the low side made
    /// `duzelt` look like it changed a file it had no reason to touch.
    #[test]
    fn a_cap_above_the_target_counts_as_wrong() {
        let s = settings(&[
            "EAresBoolSettingName::LimitFramerateAlways=True",
            "EAresFloatSettingName::MaxFramerateAlways=155",
        ]);
        assert!(s.offenders(141).iter().any(|b| b.contains("155")));
    }

    #[test]
    fn normalise_writes_every_cap_and_adds_what_is_missing() {
        let mut s = settings(&["EAresFloatSettingName::MaxFramerateInMenu=60"]);
        s.normalise(141);

        assert_eq!(s.get(LIMIT_ALWAYS), Some("True"));
        assert_eq!(s.get(MAX_ALWAYS), Some("141"));
        assert_eq!(s.get(MAX_MENU), Some("141"));
        assert_eq!(s.get(LIMIT_MENU), Some("False"));
        assert!(s.offenders(141).is_empty(), "duzeltilen dosya hala sikayetli");
    }

    /// Settings the module has no opinion about must survive untouched -
    /// crosshair and sensitivity live in the same file.
    #[test]
    fn unrelated_settings_are_left_alone() {
        let mut s = settings(&[
            "EAresIntSettingName::MouseSensitivity=42",
            "EAresFloatSettingName::MaxFramerateAlways=60",
        ]);
        s.normalise(141);
        assert_eq!(s.get("EAresIntSettingName::MouseSensitivity"), Some("42"));
    }

    /// The battery caps are deliberately outside what either mode rewrites.
    #[test]
    fn battery_caps_are_never_rewritten() {
        let original = [
            "EAresBoolSettingName::LimitFramerateOnBattery=True",
            "EAresFloatSettingName::MaxFramerateOnBattery=60",
        ];
        for mut s in [settings(&original), settings(&original)] {
            s.normalise(141);
            assert_eq!(s.get(LIMIT_BATTERY), Some("True"));
            assert_eq!(s.get(MAX_BATTERY), Some("60"));
        }
        let mut s = settings(&original);
        s.uncap();
        assert_eq!(s.get(LIMIT_BATTERY), Some("True"));
    }

    #[test]
    fn uncap_switches_off_without_discarding_the_numbers() {
        let mut s = settings(&[
            "EAresBoolSettingName::LimitFramerateAlways=True",
            "EAresFloatSettingName::MaxFramerateAlways=141",
        ]);
        s.uncap();
        assert!(s.is_uncapped());
        // The number stays, so `duzelt` afterwards has something to read and
        // the file still records what the cap used to be.
        assert_eq!(s.get(MAX_ALWAYS), Some("141"));
    }

    /// Round trip: uncapping and re-capping must land back on a clean file
    /// rather than accumulating duplicate keys.
    #[test]
    fn uncap_then_cap_leaves_one_line_per_key() {
        let mut s = settings(&["EAresFloatSettingName::MaxFramerateInMenu=60"]);
        s.uncap();
        s.normalise(141);
        for key in [LIMIT_ALWAYS, MAX_ALWAYS, LIMIT_MENU, MAX_MENU] {
            let n = s.lines.iter().filter(|l| l.starts_with(key)).count();
            assert_eq!(n, 1, "{key} {n} kez var");
        }
        assert!(!s.is_uncapped());
    }

    /// An unchanged file must report no changes, so `duzelt` can say "nothing
    /// to do" instead of taking a backup and rewriting identical content.
    #[test]
    fn a_correct_file_reports_no_changes() {
        let mut s = settings(&[
            "EAresBoolSettingName::LimitFramerateAlways=True",
            "EAresFloatSettingName::MaxFramerateAlways=141",
            "EAresBoolSettingName::LimitFramerateInMenu=False",
            "EAresFloatSettingName::MaxFramerateInMenu=141",
            "EAresBoolSettingName::LimitFramerateInBackground=False",
        ]);
        assert!(s.normalise(141).is_empty());
    }
}

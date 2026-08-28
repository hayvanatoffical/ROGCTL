//! User-editable configuration.
//!
//! Everything the policy engine used to hardcode lives here instead: the
//! envelopes, the classifier's patience, the battery behaviour and the
//! per-game overrides. If the file is missing it is written out with the
//! tuned defaults, so the first run documents its own format.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::policy::{Envelope, Mode};

/// One mode's thermal envelope, as it appears in the file.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeCfg {
    pub gpu_clock_ceiling_mhz: u32,
    pub gpu_clock_floor_mhz: u32,
    pub cpu_temp_target: u32,
    pub gpu_temp_target: u32,
    pub fan_idle_pct: u8,
    pub fan_max_pct: u8,
    pub fan_knee_c: u8,
    #[serde(default = "yes")]
    pub demand_scaling: bool,
}

impl From<EnvelopeCfg> for Envelope {
    fn from(c: EnvelopeCfg) -> Self {
        Envelope {
            gpu_clock_ceiling_mhz: c.gpu_clock_ceiling_mhz,
            gpu_clock_floor_mhz: c.gpu_clock_floor_mhz,
            cpu_temp_target: c.cpu_temp_target,
            gpu_temp_target: c.gpu_temp_target,
            fan_idle_pct: c.fan_idle_pct,
            fan_max_pct: c.fan_max_pct,
            fan_knee_c: c.fan_knee_c,
            demand_scaling: c.demand_scaling,
        }
    }
}

/// Fields a game profile may override on top of its base mode. Every one is
/// optional so a profile can change a single number without restating the rest.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct EnvelopeOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_clock_ceiling_mhz: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_clock_floor_mhz: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_temp_target: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_temp_target: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fan_idle_pct: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fan_max_pct: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fan_knee_c: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub demand_scaling: Option<bool>,
}

impl EnvelopeOverride {
    pub fn apply(&self, mut e: Envelope) -> Envelope {
        if let Some(v) = self.gpu_clock_ceiling_mhz {
            e.gpu_clock_ceiling_mhz = v;
        }
        if let Some(v) = self.gpu_clock_floor_mhz {
            e.gpu_clock_floor_mhz = v;
        }
        if let Some(v) = self.cpu_temp_target {
            e.cpu_temp_target = v;
        }
        if let Some(v) = self.gpu_temp_target {
            e.gpu_temp_target = v;
        }
        if let Some(v) = self.fan_idle_pct {
            e.fan_idle_pct = v;
        }
        if let Some(v) = self.fan_max_pct {
            e.fan_max_pct = v;
        }
        if let Some(v) = self.fan_knee_c {
            e.fan_knee_c = v;
        }
        if let Some(v) = self.demand_scaling {
            e.demand_scaling = v;
        }
        e
    }
}

/// Pin a named process to a mode, optionally reshaping that mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    /// Executable name, matched case-insensitively.
    pub process: String,
    /// Base mode key. Omit to keep whatever the classifier decided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default)]
    pub overrides: EnvelopeOverride,
}

/// Applied on top of the active envelope while running on battery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryCfg {
    /// Multiplier on the fan ceiling: quieter, at the cost of headroom.
    pub fan_max_scale: f32,
    /// Multiplier on the GPU clock ceiling: the single biggest lever on
    /// battery life.
    pub clock_ceiling_scale: f32,
    /// Never escalate past this mode while unplugged. Omit for no cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_mode: Option<String>,
}

/// Physical memory reclaim behaviour.
///
/// The defaults are deliberately conservative: only the throwaway end of the
/// standby list, only at the moments it buys something. Everything that costs
/// the user real cache is off until asked for.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryCfg {
    pub enabled: bool,
    /// Reclaim when the workload crosses into or out of a heavy mode - a game
    /// starting is the one moment a purge measurably prevents stutter, and a
    /// game exiting is when the biggest residue is left behind.
    pub on_mode_change: bool,
    /// Act when genuinely free memory (not "available") falls below this.
    pub min_free_mb: u64,
    /// Below this, the machine is genuinely starving and the full standby list
    /// is fair game regardless of `aggressive`.
    ///
    /// A measured session sat between 2 MB and 250 MB free for over two hours
    /// while the low-priority purge returned nothing every time - the throwaway
    /// end of the list was already empty, and everything left was ordinary
    /// cache the safe pass will not touch. Re-reading cache from disk is a far
    /// smaller price than running that close to empty.
    #[serde(default = "default_critical_free_mb")]
    pub critical_free_mb: u64,
    /// Never act unless there is at least this much standby to take. Below it,
    /// a purge would only be discarding cache that is doing its job.
    pub min_standby_mb: u64,
    /// Seconds between pressure-triggered passes. Mode changes ignore this.
    pub cooldown_s: u64,
    /// Allow pressure passes to empty the whole standby list and the file
    /// cache, not just the low-priority end. Frees far more, but the discarded
    /// pages get re-read from disk.
    pub aggressive: bool,
}

/// NVIDIA driver profile settings.
///
/// Only integer settings whose driver-supplied name states the unit are driven
/// here; see `nvapi::MANAGED` for why the enum-valued ones are left alone.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NvidiaCfg {
    pub enabled: bool,
    /// Global frame cap. 0 follows the panel's refresh rate, which is the point
    /// of the setting: frames past the refresh rate are never displayed, so
    /// they are pure heat.
    pub max_fps: u32,
    /// Frames per second the cap is set *above* the panel's refresh rate -
    /// **only when VRR (G-SYNC) is off**.
    ///
    /// Without VRR the driver's limiter undershoots its target by a few frames,
    /// so aiming above the refresh rate lands the delivered rate at refresh
    /// rather than under it, and the surplus frames cost nothing because they
    /// are never displayed.
    ///
    /// With VRR on this is ignored and the cap goes *below* refresh instead.
    /// There is no undershoot to compensate for above refresh - there is the
    /// edge of the variable refresh window. Crossing it drops VRR out of the
    /// picture entirely: with VSync on the driver paces to the panel, with
    /// VSync off you get tearing. Only used when `max_fps` is 0.
    #[serde(default = "default_fps_headroom")]
    pub fps_headroom: u32,
    /// Whether the VRR state is allowed to flip the cap below the refresh rate.
    ///
    /// True is the safe default and what a G-SYNC panel wants. False forces the
    /// above-refresh rule unconditionally, which is a legitimate choice: with
    /// VSync off, exceeding the variable refresh window costs tearing, not frame
    /// rate, and some people would rather have the frames.
    #[serde(default = "default_respect_vrr")]
    pub respect_vrr: bool,
    /// How far below the refresh rate the cap sits while VRR is on.
    #[serde(default = "default_vrr_margin")]
    pub vrr_margin: u32,
    /// Refresh rate to build the cap from, instead of reading the panel.
    ///
    /// 0 reads the panel live, which is what makes the cap follow a mode change.
    /// Setting it to the panel's real rate (144 here, 240 if the mode is
    /// changed) pins the cap instead: nothing that transiently reports a low
    /// refresh rate can move it, at the cost of having to edit this line if the
    /// panel's mode is ever changed for real.
    #[serde(default)]
    pub refresh_hz: u32,
    /// Cap for applications the driver considers idle - a minimised or
    /// alt-tabbed game stops rendering hundreds of unseen frames.
    pub idle_max_fps: u32,
    /// How long an application must be idle before that cap applies.
    pub idle_timeout_s: u32,
}

/// Bumped whenever the tuned defaults change in a way an existing file would
/// silently keep overriding.
///
/// A config file that already exists is never rewritten, which is right for
/// preserving edits and wrong for shipping corrections: a measured fix to the
/// default envelopes reaches nobody who already has a file. The installer
/// compares this number, backs the old file up and regenerates.
pub const CONFIG_VERSION: u32 = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub surum: u32,
    pub tick_ms: u64,
    /// Ticks a heavier workload must persist before the mode escalates.
    pub escalate_ticks: u32,
    /// Ticks a lighter workload must persist before the mode relaxes. Held
    /// high so a loading screen does not drop you out of game mode.
    pub relax_ticks: u32,
    /// How often to rewrite the fan curves, in ticks, in case something else
    /// overwrote them.
    pub curve_refresh_ticks: u64,
    /// Stop the ASUS thermal services while running. Needs administrator.
    pub suspend_asus_services: bool,
    pub modes: BTreeMap<String, EnvelopeCfg>,
    pub battery: BatteryCfg,
    #[serde(default = "default_memory")]
    pub memory: MemoryCfg,
    #[serde(default = "default_nvidia")]
    pub nvidia: NvidiaCfg,
    pub games: Vec<GameProfile>,
}

impl Config {
    pub fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("rogctl.yaml")))
            .unwrap_or_else(|| PathBuf::from("rogctl.yaml"))
    }

    /// Load the config, writing the defaults out first if there is no file.
    /// A malformed file is reported and the defaults are used, because a
    /// daemon that refuses to start leaves the machine on ASUS's curve.
    pub fn load_or_create(path: &Path) -> (Self, Option<String>) {
        if !path.exists() {
            let cfg = Self::default();
            let note = match cfg.save(path) {
                Ok(()) => format!("varsayilan config yazildi: {}", path.display()),
                Err(e) => format!("config yazilamadi ({e}) - varsayilanlarla devam"),
            };
            return (cfg, Some(note));
        }

        match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|s| {
            serde_yaml::from_str::<Config>(&s).map_err(|e| e.to_string())
        }) {
            Ok(cfg) => (cfg, None),
            Err(e) => (
                Self::default(),
                Some(format!("config okunamadi ({e}) - VARSAYILANLAR kullaniliyor")),
            ),
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let body = serde_yaml::to_string(self)?;
        std::fs::write(path, format!("{}\n{body}", HEADER))?;
        Ok(())
    }

    /// Envelope for a mode, falling back to the built-in default if the file
    /// is missing that key.
    pub fn envelope(&self, mode: Mode) -> Envelope {
        self.modes
            .get(mode.key())
            .copied()
            .map(Envelope::from)
            .unwrap_or_else(|| Envelope::from(default_envelope(mode)))
    }

    /// The profile matching any currently running process, if any.
    pub fn match_game<'a>(&'a self, running: &[String]) -> Option<&'a GameProfile> {
        self.games.iter().find(|g| {
            running
                .iter()
                .any(|p| p.eq_ignore_ascii_case(&g.process))
        })
    }
}

const HEADER: &str = "\
# rogctl yapilandirmasi
#
# Duzenledikten sonra daemon'i yeniden baslat:
#   Stop-ScheduledTask rogctl ; Start-ScheduledTask rogctl
#
# Sicakliklar Celsius, fan degerleri yuzde, saatler MHz.
# Egrilerin nasil ciktigini gormek icin:  rogctl plan
#
# Not: Bu makinede CPU guc limiti yazilamiyor (ASUS firmware kilidi), bu yuzden
# cpu_temp_target yalnizca CPU fan egrisinin sekillendigi hedeftir.
#
# memory bolumu: Windows'un standby listesini bosaltir. Varsayilan olarak
# YALNIZCA dusuk oncelikli (bir kez okunmus, tekrar istenmeyecek) kismi alir;
# ise yarayan dosya onbellegi korunur. 'aggressive: true' tumunu alir - cok daha
# fazla RAM bosaltir ama atilan sayfalar diskten yeniden okunur.
# Anlik durum ve elle temizlik:  rogctl mem  /  rogctl mem temizle
#
# nvidia bolumu: NVIDIA surucu profiline yazilir ve KALICIDIR (rogctl kapansa
# da durur). max_fps: 0 = ekranin tazeleme hizi. Ekranda gorunmeyen kare uretmek
# saf isidir, en buyuk isi kolu budur.
# Uygula / geri al:  rogctl nv uygula  /  rogctl nv sifirla
# Surucunun bildirdigi tum ayarlar:  rogctl nv
#
# Sogutucu standi: 700/1400 devirli standlarin yazilim arayuzu yoktur, kademe
# dugmesi mekaniktir. Hangi kademenin ise yaradigini olcmek icin ayni yuk
# altinda:  rogctl cool kaydet  ->  kademeyi degistir  ->  rogctl cool";

/// serde default: eski config dosyalarinda alan yoksa talep kismasi aciktir.
fn yes() -> bool {
    true
}

/// serde default, so a config file written before this section existed still
/// loads and simply gains the feature.
fn default_memory() -> MemoryCfg {
    MemoryCfg {
        enabled: true,
        on_mode_change: true,
        // Below ~2 GB genuinely free, a large allocation starts repurposing
        // standby pages at fault time, which is where the stutter comes from.
        min_free_mb: 2048,
        critical_free_mb: default_critical_free_mb(),
        min_standby_mb: 2048,
        cooldown_s: 180,
        aggressive: false,
    }
}

fn default_critical_free_mb() -> u64 {
    768
}

fn default_fps_headroom() -> u32 {
    7
}

fn default_respect_vrr() -> bool {
    true
}

/// Kept as the driver module's figure rather than a second copy of it: two
/// constants that must agree are one constant with an extra way to be wrong.
fn default_vrr_margin() -> u32 {
    crate::nvapi::VRR_MARGIN_FPS
}

fn default_nvidia() -> NvidiaCfg {
    NvidiaCfg {
        enabled: true,
        max_fps: 0,
        fps_headroom: default_fps_headroom(),
        respect_vrr: default_respect_vrr(),
        vrr_margin: default_vrr_margin(),
        // Read the panel. The guards against a bad reading - snapping to real
        // rates, and requiring two agreeing readings before lowering the cap -
        // are in place, so the live value is the better default. Pin it here if
        // the panel's rate is fixed and the cap must never move.
        refresh_hz: 0,
        // Off by default.
        //
        // This was set to 30 to stop an alt-tabbed game rendering unseen
        // frames, and the saving is real - but the driver decides what "idle"
        // means, and it does not only mean minimised. A game left running while
        // the player reads a menu, watches a cutscene or simply stops moving
        // can trip the heuristic and get clamped to 30fps mid-session, which
        // reads as a sudden stutter with no thermal cause. The heat it saves is
        // heat the machine was not making anyway (an unfocused game is not the
        // load that pushes this chassis to 95C), so it is a bad trade on a
        // machine bought to play games on.
        idle_max_fps: 0,
        idle_timeout_s: 10,
    }
}

fn default_envelope(mode: Mode) -> EnvelopeCfg {
    match mode {
        Mode::Idle => EnvelopeCfg {
            gpu_clock_ceiling_mhz: 900,
            gpu_clock_floor_mhz: 400,
            cpu_temp_target: 70,
            gpu_temp_target: 60,
            fan_idle_pct: 0,
            fan_max_pct: 30,
            fan_knee_c: 55,
            demand_scaling: true,
        },
        Mode::Media => EnvelopeCfg {
            gpu_clock_ceiling_mhz: 1100,
            gpu_clock_floor_mhz: 600,
            cpu_temp_target: 75,
            gpu_temp_target: 65,
            fan_idle_pct: 0,
            fan_max_pct: 35,
            fan_knee_c: 58,
            demand_scaling: true,
        },
        Mode::Office => EnvelopeCfg {
            gpu_clock_ceiling_mhz: 1400,
            gpu_clock_floor_mhz: 700,
            cpu_temp_target: 85,
            gpu_temp_target: 70,
            fan_idle_pct: 8,
            fan_max_pct: 55,
            fan_knee_c: 60,
            demand_scaling: true,
        },
        // Demand scaling is off in all three game tiers, and that is a
        // correction rather than a preference.
        //
        // Across a measured four and a half hour session the clock ceiling sat
        // below maximum in 95% of AAA-mode samples, and in a third of them the
        // GPU was below its temperature target while being held near 1300MHz at
        // 59% load. Those frames were given away for no thermal reason at all:
        // utilisation is low in a game because something else is the
        // bottleneck, and taking clock away from the GPU cannot fix that - it
        // only lengthens the frame. Under load the thermal trim is the correct
        // and sufficient controller, so demand scaling now stays where it earns
        // its keep, in the idle and desktop envelopes.
        Mode::LightGame => EnvelopeCfg {
            gpu_clock_ceiling_mhz: 1600,
            gpu_clock_floor_mhz: 1100,
            cpu_temp_target: 85,
            gpu_temp_target: 75,
            fan_idle_pct: 15,
            fan_max_pct: 75,
            fan_knee_c: 52,
            demand_scaling: false,
        },
        Mode::AaaGame => EnvelopeCfg {
            gpu_clock_ceiling_mhz: 1785,
            gpu_clock_floor_mhz: 1300,
            cpu_temp_target: 87,
            gpu_temp_target: 78,
            fan_idle_pct: 20,
            // Raised from 95. The same session held the CPU at 87C on average
            // and 95C at peak with the fan already pinned at its allowance, so
            // the remaining headroom is worth having.
            fan_max_pct: 100,
            fan_knee_c: 48,
            demand_scaling: false,
        },
        Mode::Render => EnvelopeCfg {
            gpu_clock_ceiling_mhz: 1785,
            gpu_clock_floor_mhz: 1300,
            cpu_temp_target: 92,
            gpu_temp_target: 84,
            fan_idle_pct: 25,
            fan_max_pct: 100,
            fan_knee_c: 48,
            demand_scaling: false,
        },
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut modes = BTreeMap::new();
        for m in Mode::ALL {
            modes.insert(m.key().to_string(), default_envelope(m));
        }

        Self {
            surum: CONFIG_VERSION,
            tick_ms: 1000,
            escalate_ticks: 2,
            // Raised from 12. A game's quiet stretches - cutscenes, menus,
            // inventory, loading - routinely run far longer than twelve
            // seconds, and every premature relax rewrote the fan curves and
            // reset the governor mid-session.
            relax_ticks: 45,
            curve_refresh_ticks: 15,
            suspend_asus_services: true,
            modes,
            battery: BatteryCfg {
                fan_max_scale: 0.7,
                clock_ceiling_scale: 0.6,
                cap_mode: Some("hafif_oyun".to_string()),
            },
            memory: default_memory(),
            nvidia: default_nvidia(),
            games: vec![
                // Competitive shooter: frames matter more than silence, but it
                // asks little of the GPU, so the ceiling stays high and the
                // demand cap is left to do the trimming.
                // Measured while running: the GPU sits around 30% busy while
                // the CPU climbs past 90C. So the fans are given full authority
                // for the CPU's sake, and demand scaling is switched off - at
                // that utilisation it would have pinned the card near 980MHz
                // purely because the game was not keeping it busy, which in a
                // competitive shooter is frames given away for nothing.
                GameProfile {
                    process: "VALORANT-Win64-Shipping.exe".to_string(),
                    mode: Some("hafif_oyun".to_string()),
                    overrides: EnvelopeOverride {
                        gpu_clock_ceiling_mhz: Some(1785),
                        gpu_clock_floor_mhz: Some(1200),
                        gpu_temp_target: Some(72),
                        fan_max_pct: Some(95),
                        fan_knee_c: Some(45),
                        demand_scaling: Some(false),
                        ..Default::default()
                    },
                },
                // Single-player AAA title, profiled from a measured session:
                // GPU load averaged 75% and the card never exceeded 80C, while
                // the CPU averaged 87C with its fan already saturated. So the
                // GPU is given its full range and the cooling is aimed at the
                // CPU, which is the part that was actually running out of room.
                GameProfile {
                    process: "APlagueTaleInnocence_x64.exe".to_string(),
                    mode: Some("aaa_oyun".to_string()),
                    overrides: EnvelopeOverride {
                        gpu_clock_ceiling_mhz: Some(1785),
                        gpu_clock_floor_mhz: Some(1300),
                        gpu_temp_target: Some(80),
                        fan_max_pct: Some(100),
                        fan_knee_c: Some(45),
                        demand_scaling: Some(false),
                        ..Default::default()
                    },
                },
                GameProfile {
                    process: "cs2.exe".to_string(),
                    mode: Some("hafif_oyun".to_string()),
                    overrides: EnvelopeOverride {
                        gpu_temp_target: Some(72),
                        fan_max_pct: Some(90),
                        demand_scaling: Some(false),
                        ..Default::default()
                    },
                },
            ],
        }
    }
}

//! Applying decisions to the hardware, and undoing them cleanly.
//!
//! Every value this module writes is captured first, so `restore` can put the
//! machine back exactly as it was found. Nothing here persists across a
//! reboot; the BIOS owns the defaults and we only override them while running.

use crate::acpi::Acpi;
use crate::devices;
use crate::gpu::Gpu;
use crate::policy::{Envelope, Mode};

/// What the machine looked like before we touched it.
struct Baseline {
    cpu_curve: Option<Vec<u8>>,
    gpu_curve: Option<Vec<u8>>,
    perf_mode: Option<u32>,
}

pub struct Controller {
    baseline: Baseline,
    applied: Option<(Mode, Envelope)>,
    applied_ceiling_mhz: u32,
    /// Set once a write path has been refused, so we complain exactly once
    /// instead of every second.
    gpu_write_denied: bool,
}

/// Temperatures at which cooling stops being negotiable, and the speeds owed
/// at each. These occupy the last two of the eight curve points in every mode.
const GUARD_WARM_C: u8 = 90;
const GUARD_WARM_PCT: u8 = 85;
const GUARD_HOT_C: u8 = 95;
const GUARD_HOT_PCT: u8 = 100;

/// Build an eight point fan curve from an envelope.
///
/// The first six points carry the mode's character: a convex ramp that stays
/// nearly flat while the chip is cool and steepens toward the target, so quiet
/// modes are genuinely quiet.
///
/// The last two are a fixed guard. The curve we write persists in the embedded
/// controller, so if this process dies while a silent profile is loaded, that
/// profile is what cools the machine until something replaces it. Reserving
/// the top of every curve means no mode can ever refuse to spin up at 90C,
/// however quiet it is meant to be lower down.
pub fn build_curve(env: &Envelope, target_temp: u32) -> [u8; 16] {
    let mut curve = [0u8; 16];

    let t_start = env.fan_knee_c.saturating_sub(15).max(30) as f32;
    // Held below the guard band so the eight temperatures stay ordered.
    let t_end = ((target_temp + 6).min(86) as f32).max(t_start + 4.0);

    let s_start = env.fan_idle_pct as f32;
    let s_span = (env.fan_max_pct as f32 - s_start).max(0.0);
    let knee = (env.fan_knee_c as f32).clamp(t_start + 1.0, t_end - 1.0);

    // Points 0 and 1 form a flat dead zone below the knee. Without it the ramp
    // starts immediately and a quiet mode asks for a few percent at desk
    // temperatures - and a fan given 3% does not idle gently, it either stops
    // or runs at its lowest stable speed, which on this chassis is about
    // 2400 RPM. Holding a true zero below the knee is what actually buys
    // silence.
    curve[0] = t_start.round() as u8;
    curve[8] = s_start.round() as u8;
    curve[1] = knee.round() as u8;
    curve[9] = s_start.round() as u8;

    // Points 2..5 carry the working ramp from the knee up to the target.
    for i in 2..6 {
        let f = (i - 1) as f32 / 4.0;
        curve[i] = (knee + (t_end - knee) * f).round().clamp(0.0, 100.0) as u8;
        curve[i + 8] = (s_start + s_span * f.powf(1.4)).round().clamp(0.0, 100.0) as u8;
    }

    curve[6] = GUARD_WARM_C;
    curve[6 + 8] = GUARD_WARM_PCT.max(env.fan_max_pct);
    curve[7] = GUARD_HOT_C;
    curve[7 + 8] = GUARD_HOT_PCT;

    // The BIOS expects both series to be non-decreasing.
    for i in 1..8 {
        if curve[i] < curve[i - 1] {
            curve[i] = curve[i - 1];
        }
        if curve[i + 8] < curve[i + 7] {
            curve[i + 8] = curve[i + 7];
        }
    }

    curve
}

impl Controller {
    pub fn new(acpi: &Acpi, gpu: Option<&Gpu>) -> Self {
        // Release any clock lock still in force before deciding anything.
        //
        // A lock survives the process that set it - that is the whole reason
        // `restore` exists - so a previous run that was killed rather than
        // asked to stop leaves the GPU pinned at whatever ceiling it happened
        // to be holding. Idle mode locks around 900MHz, which is a third of
        // this card, and the symptom is a machine that games badly for days
        // with nothing on screen explaining why. Starting from a released
        // lock costs one call and makes that state unreachable.
        if let Some(g) = gpu {
            let _ = g.unlock_graphics_clock();
        }

        let baseline = Baseline {
            cpu_curve: acpi.read_blob(devices::CPU_FAN_CURVE).ok().filter(|b| b.len() >= 16),
            gpu_curve: acpi.read_blob(devices::GPU_FAN_CURVE).ok().filter(|b| b.len() >= 16),
            perf_mode: acpi.read(devices::PERF_MODE).ok().map(|v| v & 0xFFFF),
        };

        Self {
            baseline,
            applied: None,
            applied_ceiling_mhz: 0,
            gpu_write_denied: false,
        }
    }

    /// Apply everything that only needs to change when the mode does: fan
    /// curves, package power budget, board power limit.
    /// Returns true when something actually had to be written. The envelope is
    /// part of the comparison, not just the mode: a game profile appearing or
    /// the machine being unplugged reshapes the curves without the mode ever
    /// changing.
    pub fn apply_mode(&mut self, acpi: &Acpi, gpu: Option<&Gpu>, mode: Mode, env: &Envelope) -> bool {
        if self.applied == Some((mode, *env)) {
            return false;
        }
        self.write_curves(acpi, env);
        let _ = gpu; // board power limit is refused by the vBIOS on this card
        self.applied = Some((mode, *env));
        true
    }

    /// Write both curves again without any change of mode.
    ///
    /// Armoury Crate reasserts its own curves periodically, and stopping its
    /// services needs administrator rights we may not have. Rewriting on a
    /// timer means the worst case is a few seconds of its profile rather than
    /// ours quietly being discarded for the rest of the session.
    pub fn refresh_curves(&self, acpi: &Acpi, env: &Envelope) {
        self.write_curves(acpi, env);
    }

    fn write_curves(&self, acpi: &Acpi, env: &Envelope) {
        let cpu_curve = build_curve(env, env.cpu_temp_target);
        let gpu_curve = build_curve(env, env.gpu_temp_target);

        if let Err(e) = acpi.write_blob(devices::CPU_FAN_CURVE, &cpu_curve) {
            eprintln!("  [!] CPU fan egrisi yazilamadi: {e}");
        }
        if let Err(e) = acpi.write_blob(devices::GPU_FAN_CURVE, &gpu_curve) {
            eprintln!("  [!] GPU fan egrisi yazilamadi: {e}");
        }
    }

    /// Per-tick GPU clock ceiling. Only written when it actually moves.
    ///
    /// Returns the one-off warning when the lock cannot be written, for the
    /// caller to put wherever its output goes. This used to print to stderr,
    /// which a detached daemon discards: the single failure that silently
    /// costs two thirds of the card's clock was the one failure nobody could
    /// see afterwards.
    #[must_use]
    pub fn apply_clock_ceiling(&mut self, gpu: Option<&Gpu>, mhz: u32) -> Option<String> {
        let g = gpu?;
        if self.gpu_write_denied || mhz == self.applied_ceiling_mhz {
            return None;
        }
        match g.lock_graphics_clock(210, mhz) {
            Ok(()) => {
                self.applied_ceiling_mhz = mhz;
                None
            }
            Err(e) => {
                self.gpu_write_denied = true;
                Some(format!(
                    "[!] GPU saat tavani yazilamadi ({e}). Kart kendi basina                      boost edecek - sicaklik yonetimi yalnizca fanlara kalir.                      Yonetici yetkisi gerekiyor."
                ))
            }
        }
    }

    /// Put everything back the way we found it.
    pub fn restore(&self, acpi: &Acpi, gpu: Option<&Gpu>) {
        if let Some(c) = &self.baseline.cpu_curve {
            let _ = acpi.write_blob(devices::CPU_FAN_CURVE, &c[..16]);
        }
        if let Some(c) = &self.baseline.gpu_curve {
            let _ = acpi.write_blob(devices::GPU_FAN_CURVE, &c[..16]);
        }
        if let Some(m) = self.baseline.perf_mode {
            let _ = acpi.write(devices::PERF_MODE, m);
        }

        // Releasing the clock ceiling is the one that matters: leaving a lock
        // behind would cap the GPU until the driver is reloaded.
        if let Some(g) = gpu {
            let _ = g.unlock_graphics_clock();
        }
    }
}

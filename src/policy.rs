//! Workload classification and the control policy that follows from it.
//!
//! Two decisions live here. First, *what is the machine doing* - inferred from
//! CPU load, GPU load and VRAM occupancy, smoothed so a loading screen or a
//! background compile does not make the profile flap. Second, *what should the
//! hardware do about it* - a per-mode envelope plus a governor that trims the
//! GPU clock ceiling to hold a temperature target.
//!
//! The governor is the part that fixes partial-load heat. A GeForce boosts to
//! its maximum clock whenever anything asks it to, and board power scales
//! roughly with V squared times f, so a GPU that is only half busy still pays
//! near-peak voltage. Capping the clock makes the driver select the lowest
//! voltage point that sustains it, which is an undervolt expressed through a
//! supported API.

use crate::telemetry::Sample;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    Media,
    Office,
    LightGame,
    AaaGame,
    Render,
}

impl Mode {
    pub const ALL: [Mode; 6] = [
        Mode::Idle,
        Mode::Media,
        Mode::Office,
        Mode::LightGame,
        Mode::AaaGame,
        Mode::Render,
    ];

    /// Stable identifier used as the config file's map key. Kept separate from
    /// `label`, which is display text and free to change.
    pub fn key(self) -> &'static str {
        match self {
            Mode::Idle => "bosta",
            Mode::Media => "film",
            Mode::Office => "ofis",
            Mode::LightGame => "hafif_oyun",
            Mode::AaaGame => "aaa_oyun",
            Mode::Render => "render",
        }
    }

    pub fn from_key(s: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.key() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Idle => "bosta",
            Mode::Media => "film",
            Mode::Office => "ofis",
            Mode::LightGame => "hafif oyun",
            Mode::AaaGame => "AAA oyun",
            Mode::Render => "render",
        }
    }

    /// Ranking used to decide whether a mode change is an escalation. Going up
    /// should be quick; coming down should be reluctant. Also what the battery
    /// cap compares against.
    pub fn intensity(self) -> u8 {
        match self {
            Mode::Idle => 0,
            Mode::Media => 1,
            Mode::Office => 2,
            Mode::LightGame => 3,
            Mode::AaaGame => 4,
            Mode::Render => 5,
        }
    }
}

/// The thermal envelope for one mode.
///
/// Deliberately limited to what this machine actually honours. CPU package
/// power was measured to be untouchable here - ACPI PPT writes, the ASUS
/// performance mode and the Windows processor maximum state were each set to
/// aggressive values under sustained load and none of them moved the package
/// temperature off 95C. The i9-12900H targets Tjmax by design and ASUS owns
/// that behaviour in firmware, so a `cpu_ppt_w` field here would be a lie.
/// The dGPU board power limit is likewise refused by the vBIOS (NVML reports
/// NOT_SUPPORTED), which leaves the clock ceiling as the GPU power lever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Envelope {
    /// Upper bound the governor may raise the clock ceiling to.
    pub gpu_clock_ceiling_mhz: u32,
    /// Lower bound the governor may cut to. In the game and render modes this
    /// is held high on purpose: once the GPU is the bottleneck, every megahertz
    /// removed is frames removed, so the fans are made to do the work instead.
    pub gpu_clock_floor_mhz: u32,
    pub cpu_temp_target: u32,
    pub gpu_temp_target: u32,
    pub fan_idle_pct: u8,
    pub fan_max_pct: u8,
    /// Temperature at which the fan curve starts to climb.
    pub fan_knee_c: u8,
    /// Whether the governor may trim the clock just because utilisation is
    /// low. Worth turning off for competitive shooters: they leave the GPU
    /// far from busy while still wanting every frame, and a clock chosen from
    /// utilisation alone would cap them for no thermal reason.
    pub demand_scaling: bool,
}

/// Pick the mode that best explains this instant, before smoothing.
///
/// `cpu` and `gpu` arrive already smoothed; VRAM and the decoder counter are
/// read raw because they do not bounce the way utilisation does.
fn classify_raw(cpu: f32, gpu: f32, vram: f32, decoder: u32, current: Mode) -> Mode {
    // Hysteresis on the thresholds themselves, not just on how many ticks a
    // change must persist.
    //
    // A four and a half hour session was measured flipping mode 120 times, 118
    // of them between the two game tiers, because a real game's mean GPU load
    // sat almost exactly on the gate that separates them - the worst possible
    // place for a threshold. Tick counting cannot fix that: the load genuinely
    // stays on the far side long enough to qualify, over and over. Making a
    // gate easier to stay above than to cross is what actually settles it, and
    // it also stops a cutscene from dropping a running game into the idle
    // envelope, which had been happening every fifteen seconds.
    // The floor is the correction to that fix. Hysteresis lowers a gate by
    // 0.15, and for the light-game gate that lands it at 0.15 - below what an
    // idle desktop actually reads on this machine. Measured while nothing but a
    // browser was open: 16% GPU, 20% VRAM. Both clear the lowered gate, so once
    // anything nudged the machine into the light-game envelope it could never
    // come back out, and an afternoon of browsing ran on a 85C target with the
    // fan knee at 52C. Tick counting cannot rescue it either: the raw
    // classification never disagrees, so the relax counter never starts.
    //
    // So a gate may be made easier to hold, but never easier than the noise
    // floor of an idle desktop. Only the light-game floor binds; the two above
    // it sit far above anything a desktop produces and keep their full 0.15.
    let sticky = |m: Mode, gate: f32, floor: f32| {
        if current.intensity() >= m.intensity() {
            (gate - 0.15).max(floor)
        } else {
            gate
        }
    };

    // Sustained CPU saturation, or a GPU compute job that is also feeding the
    // card hard from the CPU side.
    if cpu > sticky(Mode::Render, 0.85, 0.70) || (gpu > 0.90 && vram > 0.30 && cpu > 0.50) {
        return Mode::Render;
    }
    // Load, not footprint, is what makes a title demanding. An earlier version
    // gated this behind VRAM occupancy above 45% and misread a real game that
    // sat at 85% GPU on 2.7GB of 8GB - plenty of engines simply do not fill
    // the card. VRAM now only corroborates the lighter tier.
    // A demanding title's load swings hard between scenes - the one measured
    // here ranged from 60% to 100% second to second - so a high bar alone
    // drops it into the lighter envelope every time the camera turns. A
    // resident working set is the steadier half of the evidence, so moderate
    // load plus real VRAM occupancy counts as the same thing.
    if gpu > sticky(Mode::AaaGame, 0.75, 0.60) || (gpu > sticky(Mode::AaaGame, 0.55, 0.40) && vram > 0.28) {
        return Mode::AaaGame;
    }
    if gpu > sticky(Mode::LightGame, 0.30, 0.25) && vram > 0.15 {
        return Mode::LightGame;
    }
    // Video playback is read straight off the decode engine rather than
    // guessed at from VRAM. An idle desktop with a browser open holds about
    // the same working set as a film does, so occupancy alone cannot tell them
    // apart - it was doing exactly that until NVDEC replaced it.
    //
    // Caveat: with hybrid graphics a browser may decode on the Intel iGPU, in
    // which case NVDEC stays flat and playback lands in Idle instead. That is
    // a benign miss, since Idle is the quieter envelope of the two.
    if decoder > 0 && gpu < 0.40 && cpu < 0.35 {
        return Mode::Media;
    }
    if cpu > 0.15 {
        return Mode::Office;
    }
    Mode::Idle
}

/// Smooths raw classifications into a stable mode.
pub struct Classifier {
    current: Mode,
    candidate: Mode,
    ticks: u32,
    escalate_ticks: u32,
    relax_ticks: u32,
    ema_cpu: f32,
    ema_gpu: f32,
    seeded: bool,
}

/// Weight given to the newest sample. In-game GPU utilisation swings between
/// 60% and 100% from frame to frame; without this the classifier would chase
/// every dip.
const EMA_ALPHA: f32 = 0.3;

impl Classifier {
    pub fn new(escalate_ticks: u32, relax_ticks: u32) -> Self {
        Self {
            current: Mode::Idle,
            candidate: Mode::Idle,
            ticks: 0,
            escalate_ticks,
            relax_ticks,
            ema_cpu: 0.0,
            ema_gpu: 0.0,
            seeded: false,
        }
    }

    /// Smoothed utilisation, exposed so the status line can show what the
    /// classifier is actually reasoning about.
    pub fn smoothed(&self) -> (f32, f32) {
        (self.ema_cpu, self.ema_gpu)
    }

    pub fn current(&self) -> Mode {
        self.current
    }

    /// Feed one sample. Returns `Some(mode)` only when the mode actually
    /// changes, so callers can log transitions rather than every tick.
    pub fn update(&mut self, s: &Sample) -> Option<Mode> {
        let cpu_now = s.cpu_util;
        let gpu_now = s.gpu.util_gpu as f32 / 100.0;

        if self.seeded {
            self.ema_cpu += EMA_ALPHA * (cpu_now - self.ema_cpu);
            self.ema_gpu += EMA_ALPHA * (gpu_now - self.ema_gpu);
        } else {
            self.ema_cpu = cpu_now;
            self.ema_gpu = gpu_now;
            self.seeded = true;
        }

        let raw = classify_raw(
            self.ema_cpu,
            self.ema_gpu,
            s.gpu.vram_frac,
            s.gpu.util_decoder,
            self.current,
        );

        if raw == self.current {
            self.candidate = raw;
            self.ticks = 0;
            return None;
        }

        if raw == self.candidate {
            self.ticks += 1;
        } else {
            self.candidate = raw;
            self.ticks = 1;
        }

        let needed = if raw.intensity() > self.current.intensity() {
            self.escalate_ticks
        } else {
            self.relax_ticks
        };

        if self.ticks >= needed {
            self.current = raw;
            self.ticks = 0;
            return Some(raw);
        }
        None
    }
}

/// Holds a GPU temperature target by trimming the clock ceiling.
pub struct Governor {
    ceiling_mhz: u32,
    /// Ticks to wait between adjustments so the thermal response has time to
    /// show up before we react again.
    cooldown: u32,
    since_change: u32,
}

const STEP_DOWN_MHZ: u32 = 60;
const STEP_UP_MHZ: u32 = 45;

/// How far above a mode's comfort target the temperature must go before we
/// will trim clocks on a GPU that is already fully loaded.
const SATURATED_SAFETY_MARGIN_C: u32 = 8;

/// Utilisation above which the GPU is the thing limiting frame rate.
const SATURATED_UTIL: f32 = 0.90;

impl Governor {
    pub fn new() -> Self {
        Self {
            ceiling_mhz: 1785,
            cooldown: 3,
            since_change: 0,
        }
    }

    /// Work out the clock ceiling for this tick.
    ///
    /// Two forces act on it. A demand cap keeps the clock proportional to how
    /// busy the GPU actually is, which is what stops a half-loaded card from
    /// boosting to maximum voltage and sitting at eighty degrees. On top of
    /// that, a thermal term pulls the ceiling down whenever the measured
    /// temperature exceeds the envelope's target.
    pub fn update(&mut self, s: &Sample, env: &Envelope) -> u32 {
        let util = s.gpu.util_gpu as f32 / 100.0;
        let temp = s.gpu.temp_c;
        let saturated = util > SATURATED_UTIL;

        // The mode's floor exists to protect frame rate, and that only matters
        // while the GPU is the bottleneck. Below saturation the card is boosting
        // harder than the work requires - the case where a game draws modest
        // utilisation yet still runs hot - so the governor is allowed to reach
        // considerably further down for a real voltage drop at no cost.
        let floor = if saturated {
            env.gpu_clock_floor_mhz
        } else {
            (env.gpu_clock_floor_mhz * 3 / 4).max(400)
        };

        // How much clock the current demand can justify. This is the half that
        // addresses partial-load heat: a GPU at 40% has no business holding
        // maximum boost voltage.
        let demand_cap = if !env.demand_scaling {
            env.gpu_clock_ceiling_mhz
        } else if util < 0.35 {
            (env.gpu_clock_ceiling_mhz as f32 * 0.55) as u32
        } else if util < 0.60 {
            (env.gpu_clock_ceiling_mhz as f32 * 0.72) as u32
        } else if util < 0.85 {
            (env.gpu_clock_ceiling_mhz as f32 * 0.88) as u32
        } else {
            env.gpu_clock_ceiling_mhz
        };
        let demand_cap = demand_cap.max(floor);

        self.since_change += 1;
        if self.since_change < self.cooldown {
            return self.ceiling_mhz.clamp(floor, env.gpu_clock_ceiling_mhz).min(demand_cap);
        }

        // A saturated GPU is the frame rate limiter, so cutting its clock costs
        // performance directly. Under those conditions the comfort target is
        // handed to the fans and the governor only steps in for real thermal
        // safety. An earlier revision chased the comfort target regardless and
        // wound a fully loaded card from 1785MHz down to its floor.
        let act_temp = if saturated {
            env.gpu_temp_target + SATURATED_SAFETY_MARGIN_C
        } else {
            env.gpu_temp_target
        };

        if temp > act_temp {
            // Overshoot scales the response: a couple of degrees is a nudge,
            // ten degrees is a shove.
            let over = temp - act_temp;
            self.ceiling_mhz = self
                .ceiling_mhz
                .saturating_sub(STEP_DOWN_MHZ * over.clamp(1, 4));
            self.since_change = 0;
        } else if temp + 5 < act_temp && self.ceiling_mhz < demand_cap {
            self.ceiling_mhz += STEP_UP_MHZ;
            self.since_change = 0;
        }

        self.ceiling_mhz = self
            .ceiling_mhz
            .clamp(floor, env.gpu_clock_ceiling_mhz)
            .min(demand_cap);

        self.ceiling_mhz
    }

    /// Let the ceiling float back up when the envelope changes under us.
    pub fn reset_to(&mut self, mhz: u32) {
        self.ceiling_mhz = mhz;
        self.since_change = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The measured idle desktop: a browser open, nothing else. These are the
    /// exact numbers the daemon reported while the machine was sitting in the
    /// light-game envelope with an 85C target, which is what made an ordinary
    /// afternoon run at 90C.
    const BOS_MASAUSTU: (f32, f32, f32) = (0.06, 0.16, 0.20);

    #[test]
    fn idle_desktop_leaves_light_game() {
        let (cpu, gpu, vram) = BOS_MASAUSTU;
        assert_ne!(
            classify_raw(cpu, gpu, vram, 0, Mode::LightGame),
            Mode::LightGame,
            "browsing must be able to fall out of the game envelope"
        );
    }

    #[test]
    fn real_light_game_still_holds_through_a_lull() {
        // A game that drops to 27% GPU for a few seconds - below the entry gate
        // of 0.30, above the floor - must not be dropped to the desktop envelope.
        assert_eq!(
            classify_raw(0.30, 0.27, 0.35, 0, Mode::LightGame),
            Mode::LightGame
        );
    }

    #[test]
    fn light_game_entry_gate_unchanged() {
        assert_eq!(classify_raw(0.20, 0.28, 0.20, 0, Mode::Office), Mode::Office);
        assert_eq!(
            classify_raw(0.20, 0.35, 0.20, 0, Mode::Office),
            Mode::LightGame
        );
    }

    #[test]
    fn aaa_hysteresis_keeps_its_full_width() {
        // 0.62 is below the 0.75 entry gate but above the sticky 0.60 - the case
        // the sticky gates were introduced for. The floor must not narrow it.
        assert_eq!(
            classify_raw(0.30, 0.62, 0.20, 0, Mode::AaaGame),
            Mode::AaaGame
        );
    }
}

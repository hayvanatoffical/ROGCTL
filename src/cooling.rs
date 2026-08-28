//! Measuring how well the machine is actually shedding heat.
//!
//! A laptop cooling stand has no software interface - the ones with a 700/1400
//! RPM range are driven by a physical dial, and nothing on the host can move
//! it. So the useful thing software can do is not to control the stand but to
//! answer the questions the dial poses: is it helping at all, and is the loud
//! setting worth the noise over the quiet one?
//!
//! That is a measurement problem, and it needs no knowledge of the stand. Run a
//! window with the stand off, run one with it on, and compare. Because nothing
//! here assumes the stand exists, removing it cannot break anything: the
//! numbers simply move back.
//!
//! The same measurement doubles as a health check. Vents clogging with dust or
//! a laptop sitting on a duvet show up as exactly the same signal, in the same
//! direction, as unplugging the stand.

use std::path::Path;

use crate::telemetry::Sample;

/// Assumed room temperature, in Celsius.
///
/// A laptop has no ambient sensor, so the absolute thermal resistance derived
/// below is only as good as this guess. That does not matter for the job: the
/// same constant sits in both runs of a comparison, so the ratio between them
/// stays honest even when the constant is wrong.
const ASSUMED_AMBIENT_C: f32 = 25.0;

/// A GPU drawing less than this tells us nothing about cooling - the die is
/// barely making heat, so its temperature is dominated by the chassis rather
/// than by how well air is moving through it.
const MIN_MEANINGFUL_W: f32 = 15.0;

#[derive(Debug, Clone, Copy, Default)]
pub struct Reading {
    pub secs: u64,
    pub cpu_temp: f32,
    pub gpu_temp: f32,
    pub gpu_power: f32,
    pub gpu_util: f32,
    pub cpu_util: f32,
    pub cpu_fan: f32,
    pub gpu_fan: f32,
    pub samples: u32,
}

/// Running mean of everything a comparison needs.
#[derive(Default)]
pub struct Accumulator {
    r: Reading,
}

impl Accumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, s: &Sample) {
        let n = self.r.samples as f32;
        let m = n + 1.0;
        let avg = |old: f32, new: f32| (old * n + new) / m;

        self.r.cpu_temp = avg(self.r.cpu_temp, s.cpu_temp_c as f32);
        self.r.gpu_temp = avg(self.r.gpu_temp, s.gpu.temp_c as f32);
        self.r.gpu_power = avg(self.r.gpu_power, s.gpu.power_w);
        self.r.gpu_util = avg(self.r.gpu_util, s.gpu.util_gpu as f32);
        self.r.cpu_util = avg(self.r.cpu_util, s.cpu_util * 100.0);
        self.r.cpu_fan = avg(self.r.cpu_fan, s.cpu_fan_rpm as f32);
        self.r.gpu_fan = avg(self.r.gpu_fan, s.gpu_fan_rpm as f32);
        self.r.samples += 1;
    }

    pub fn finish(mut self, secs: u64) -> Reading {
        self.r.secs = secs;
        self.r
    }
}

impl Reading {
    /// Thermal resistance in degrees per watt: how many degrees above the room
    /// each watt of GPU heat costs. Lower is better cooling.
    pub fn c_per_w(&self) -> Option<f32> {
        if self.gpu_power < MIN_MEANINGFUL_W {
            return None;
        }
        Some((self.gpu_temp - ASSUMED_AMBIENT_C) / self.gpu_power)
    }

    /// Whether two runs were taken under similar enough load to compare.
    ///
    /// Comparing a run taken in a menu against one taken mid-firefight would
    /// show a difference that has nothing to do with cooling, so the honest
    /// thing is to say so rather than print a number that looks like an answer.
    pub fn comparable_to(&self, other: &Reading) -> Result<(), String> {
        // Deliberately tight. Thermal resistance divides temperature rise by
        // power, so it looks like it normalises load away - but it does not:
        // the fan curve reacts to temperature, so a heavier run also spins the
        // fans harder and returns a flattering number that has nothing to do
        // with how the machine is being cooled.
        let dp = (self.gpu_power - other.gpu_power).abs();
        let allow = (self.gpu_power.max(other.gpu_power) * 0.10).max(5.0);
        if dp > allow {
            return Err(format!(
                "GPU gucu cok farkli ({:.0}W vs {:.0}W) - ayni yuk altinda tekrarla",
                other.gpu_power, self.gpu_power
            ));
        }
        if (self.gpu_util - other.gpu_util).abs() > 10.0 {
            return Err(format!(
                "GPU yuku cok farkli (%{:.0} vs %{:.0}) - ayni yuk altinda tekrarla",
                other.gpu_util, self.gpu_util
            ));
        }
        Ok(())
    }

    /// Read the comparison out loud, separating what the cooling did from what
    /// the fans did.
    ///
    /// Both runs sit at thermal equilibrium, so a machine that is cooled better
    /// settles at a lower temperature *and* a lower fan speed - the curve backs
    /// off once the temperature drops. That makes a fall in both the signature
    /// of real extra cooling. A better temperature bought with a faster fan is
    /// a different thing entirely, and saying so is the whole point of
    /// measuring instead of guessing.
    pub fn verdict(&self, base: &Reading) -> Vec<String> {
        let mut out = Vec::new();
        let (Some(a), Some(b)) = (base.c_per_w(), self.c_per_w()) else {
            out.push("Termal direnc olculemedi - GPU yeterince guc cekmiyor.".into());
            return out;
        };

        let pct = (b - a) / a * 100.0;
        let fan_delta = (self.gpu_fan - base.gpu_fan) + (self.cpu_fan - base.cpu_fan);
        let fan_moved = fan_delta.abs() > 300.0;

        match (pct < -4.0, pct > 4.0) {
            (true, _) if fan_delta > 300.0 => out.push(format!(
                "Sicaklik/watt {:.1}% iyilesti AMA fanlar {:.0} RPM daha hizli donuyor - \
                 kazanc sogutmadan degil fandan geliyor olabilir.",
                -pct, fan_delta
            )),
            (true, _) => {
                out.push(format!("Sogutma REFERANSTAN IYI: sicaklik/watt {:.1}% dustu.", -pct));
                if fan_delta < -300.0 {
                    out.push(format!(
                        "Ustelik fanlar {:.0} RPM daha YAVAS donuyor - hem daha serin hem daha sessiz. \
                         Bu, gercek ek sogutmanin imzasidir.",
                        -fan_delta
                    ));
                }
            }
            (_, true) if fan_delta < -300.0 => out.push(format!(
                "Sicaklik/watt {:.1}% kotulesti ama fanlar {:.0} RPM daha yavas - \
                 karsilastirma esit kosulda degil.",
                pct, -fan_delta
            )),
            (_, true) => {
                out.push(format!("Sogutma REFERANSTAN KOTU: sicaklik/watt {:.1}% artti.", pct));
                out.push("Stand cikmis, hava girisi kapali (yatak/kucak) veya fanlar tozlanmis olabilir.".into());
            }
            _ => {
                out.push("Fark olcum gurultusu icinde - anlamli bir degisiklik yok.".into());
                if fan_moved {
                    out.push(format!("(Fanlar {fan_delta:+.0} RPM oynadi, yine de sonuc esit cikti.)"));
                }
            }
        }
        out
    }

    pub fn save(&self, path: &Path, label: &str) -> std::io::Result<()> {
        let body = format!(
            "etiket={label}\nsure_s={}\nornek={}\ncpu_c={:.1}\ngpu_c={:.1}\ngpu_w={:.1}\n\
             gpu_util={:.1}\ncpu_util={:.1}\ncpu_fan={:.0}\ngpu_fan={:.0}\n",
            self.secs, self.samples, self.cpu_temp, self.gpu_temp, self.gpu_power,
            self.gpu_util, self.cpu_util, self.cpu_fan, self.gpu_fan
        );
        std::fs::write(path, body)
    }

    pub fn load(path: &Path) -> Option<(String, Reading)> {
        let body = std::fs::read_to_string(path).ok()?;
        let map: std::collections::BTreeMap<&str, &str> =
            body.lines().filter_map(|l| l.split_once('=')).collect();
        let f = |k: &str| map.get(k).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
        Some((
            map.get("etiket").copied().unwrap_or("-").to_string(),
            Reading {
                secs: f("sure_s") as u64,
                cpu_temp: f("cpu_c"),
                gpu_temp: f("gpu_c"),
                gpu_power: f("gpu_w"),
                gpu_util: f("gpu_util"),
                cpu_util: f("cpu_util"),
                cpu_fan: f("cpu_fan"),
                gpu_fan: f("gpu_fan"),
                samples: f("ornek") as u32,
            },
        ))
    }
}

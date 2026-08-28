//! Suspending the ASUS services that fight us for the fan curves.
//!
//! Armoury Crate periodically rewrites the curve the embedded controller runs,
//! which was measured here as fans returning to roughly 2300 RPM at idle a few
//! seconds after our own curve had brought them to a stop. Holding the curve
//! therefore means holding these services down for as long as we are driving.
//!
//! Only the thermal ones are touched. Aura lighting, the keyboard hotkey
//! service and the rest of the ASUS stack are left alone, and everything we
//! stop is started again on the way out.

use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

/// Keep the console hidden; the daemon has already detached from one.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The services observed reasserting ASUS's own fan curve. Deliberately short:
/// `ArmouryCrateControlInterface` and the Aura services are left running so
/// lighting and hotkeys keep working.
pub const THERMAL_SERVICES: &[&str] = &["ASUSOptimization", "ArmouryCrateService"];

fn sc(args: &[&str]) -> Option<String> {
    let out = Command::new("sc.exe")
        .args(args)
        .stdin(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn is_running(name: &str) -> bool {
    sc(&["query", name])
        .map(|s| s.contains("RUNNING"))
        .unwrap_or(false)
}

/// Outcome of a suspend attempt.
///
/// "We could not stop it" and "it was already down" look identical in a plain
/// list of names but mean opposite things, and only the first is a problem
/// worth warning about. Keeping them apart also keeps restart honest: we put
/// back exactly what we took down and nothing else.
#[derive(Debug, Default)]
pub struct Suspension {
    pub stopped_by_us: Vec<String>,
    pub already_down: Vec<String>,
    pub failed: Vec<String>,
}

impl Suspension {
    /// Every thermal service currently down, whoever stopped it.
    pub fn down(&self) -> Vec<String> {
        let mut v = self.stopped_by_us.clone();
        v.extend(self.already_down.iter().cloned());
        v
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.stopped_by_us.is_empty() {
            parts.push(format!("askiya alindi: {}", self.stopped_by_us.join(", ")));
        }
        if !self.already_down.is_empty() {
            parts.push(format!("zaten kapali: {}", self.already_down.join(", ")));
        }
        if !self.failed.is_empty() {
            parts.push(format!("DURDURULAMADI: {}", self.failed.join(", ")));
        }
        if parts.is_empty() {
            "ASUS termal servisi bulunamadi".to_string()
        } else {
            parts.join(" | ")
        }
    }
}

/// Stop the thermal services, recording precisely what happened to each.
pub fn suspend() -> Suspension {
    let mut out = Suspension::default();

    for name in THERMAL_SERVICES {
        if !is_running(name) {
            out.already_down.push((*name).to_string());
            continue;
        }
        sc(&["stop", name]);
        // sc returns as soon as the request is accepted, so confirm rather
        // than assume; a service that refused to stop must not be recorded as
        // ours to restart.
        let mut confirmed = false;
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(300));
            if !is_running(name) {
                confirmed = true;
                break;
            }
        }
        if confirmed {
            out.stopped_by_us.push((*name).to_string());
        } else {
            out.failed.push((*name).to_string());
        }
    }

    out
}

/// Start the services we stopped.
pub fn resume(names: &[String]) {
    for name in names {
        sc(&["start", name]);
    }
}

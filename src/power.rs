//! Mains or battery.
//!
//! Unplugging changes what the machine should be optimising for: the same
//! workload wants a quieter, cooler, longer-lasting envelope rather than the
//! fastest one.

use std::mem;

use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Ac,
    Battery,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct PowerState {
    pub source: Source,
    /// Remaining charge, 0-100, or `None` when the firmware will not say.
    pub battery_pct: Option<u8>,
}

pub fn read() -> PowerState {
    let mut st: SYSTEM_POWER_STATUS = unsafe { mem::zeroed() };
    if unsafe { GetSystemPowerStatus(&mut st) } == 0 {
        return PowerState {
            source: Source::Unknown,
            battery_pct: None,
        };
    }

    let source = match st.ACLineStatus {
        0 => Source::Battery,
        1 => Source::Ac,
        _ => Source::Unknown,
    };

    // 255 is the documented "unknown" sentinel.
    let battery_pct = if st.BatteryLifePercent <= 100 {
        Some(st.BatteryLifePercent)
    } else {
        None
    };

    PowerState { source, battery_pct }
}

impl PowerState {
    pub fn on_battery(&self) -> bool {
        self.source == Source::Battery
    }

    pub fn label(&self) -> &'static str {
        match self.source {
            Source::Ac => "prizde",
            Source::Battery => "batarya",
            Source::Unknown => "bilinmiyor",
        }
    }
}

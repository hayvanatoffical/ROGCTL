//! Unified sampling of everything the policy engine reasons about.
//!
//! Two sources feed in: the ASUS embedded controller over ACPI (CPU package
//! temperature, fan tachometers) and NVML (the whole GPU picture). CPU
//! utilisation comes from `GetSystemTimes`, which needs no driver — important
//! here, because Memory Integrity is enabled and the usual MSR route through
//! WinRing0 will not load.

use crate::acpi::Acpi;
use crate::devices;
use crate::gpu::{Gpu, GpuSample};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FileTime {
    low: u32,
    high: u32,
}

// windows-sys 0.59 does not bind this one, and the signature is small enough
// that declaring it directly beats pulling in another crate.
#[link(name = "kernel32")]
extern "system" {
    fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Sample {
    pub cpu_temp_c: u32,
    /// 0.0 - 1.0 across all logical processors.
    pub cpu_util: f32,
    pub cpu_fan_rpm: u32,
    pub gpu_fan_rpm: u32,
    pub gpu: GpuSample,
    pub gpu_ok: bool,
}

fn filetime_u64(ft: FileTime) -> u64 {
    ((ft.high as u64) << 32) | ft.low as u64
}

pub struct Telemetry {
    pub acpi: Acpi,
    pub gpu: Option<Gpu>,
    prev_idle: u64,
    prev_busy: u64,
}

impl Telemetry {
    pub fn new() -> anyhow::Result<Self> {
        let acpi = Acpi::open()?;
        acpi.init().ok();
        let gpu = Gpu::open().ok();

        Ok(Self {
            acpi,
            gpu,
            prev_idle: 0,
            prev_busy: 0,
        })
    }

    /// CPU utilisation since the previous call. The first call has no baseline
    /// and reports zero.
    fn cpu_util(&mut self) -> f32 {
        let mut idle = FileTime::default();
        let mut kernel = FileTime::default();
        let mut user = FileTime::default();

        if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
            return 0.0;
        }

        let idle = filetime_u64(idle);
        // Kernel time already includes idle time, so total busy is
        // kernel + user - idle.
        let busy = filetime_u64(kernel) + filetime_u64(user);

        let d_idle = idle.saturating_sub(self.prev_idle);
        let d_busy = busy.saturating_sub(self.prev_busy);
        self.prev_idle = idle;
        self.prev_busy = busy;

        if d_busy == 0 {
            return 0.0;
        }
        let util = 1.0 - (d_idle as f32 / d_busy as f32);
        util.clamp(0.0, 1.0)
    }

    pub fn sample(&mut self) -> Sample {
        let cpu_util = self.cpu_util();
        let read = |id: u32| self.acpi.read(id).map(|v| v & 0xFFFF).unwrap_or(0);

        let (gpu, gpu_ok) = match &self.gpu {
            Some(g) => (g.sample(), true),
            None => (GpuSample::default(), false),
        };

        Sample {
            cpu_temp_c: read(devices::CPU_TEMP),
            cpu_util,
            cpu_fan_rpm: read(devices::CPU_FAN_RPM) * 100,
            gpu_fan_rpm: read(devices::GPU_FAN_RPM) * 100,
            gpu,
            gpu_ok,
        }
    }
}

//! NVIDIA GPU telemetry and control via NVML.
//!
//! NVML is NVIDIA's stable, documented management API and it ships with the
//! driver, so we bind it at runtime rather than linking against a SDK. It
//! covers three of the four GPU levers we care about:
//!
//!   * VRAM / utilisation / temperature / power draw  (the classifier inputs)
//!   * the power management limit                     (dynamic TGP)
//!   * locked graphics clocks                         (effective undervolt)
//!
//! Clock *offsets* are not exposed by NVML and need NVAPI; that lives
//! elsewhere. Everything here is reversible and nothing persists across a
//! driver reload.

use std::ffi::{c_int, c_uint, c_void, CString};
use std::mem;
use std::ptr;

use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

type NvmlReturn = c_int;
type NvmlDevice = *mut c_void;

const NVML_SUCCESS: NvmlReturn = 0;
const NVML_TEMPERATURE_GPU: c_uint = 0;
const NVML_CLOCK_GRAPHICS: c_uint = 0;
const NVML_CLOCK_MEM: c_uint = 2;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct NvmlMemory {
    total: u64,
    free: u64,
    used: u64,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct NvmlUtilization {
    gpu: c_uint,
    memory: c_uint,
}

/// One sample of everything the control loop needs from the GPU.
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuSample {
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    /// Fraction of VRAM in use, 0.0 - 1.0. The strongest single signal for
    /// telling a AAA title apart from a browser playing video.
    pub vram_frac: f32,
    pub util_gpu: u32,
    pub util_mem: u32,
    pub temp_c: u32,
    pub power_w: f32,
    pub power_limit_w: f32,
    pub clock_graphics_mhz: u32,
    pub clock_mem_mhz: u32,
    /// NVDEC activity. Video playback lights this up while leaving the 3D
    /// engine idle, which is what separates watching a film from doing nothing.
    pub util_decoder: u32,
    /// NVENC activity: recording, streaming, or a capture overlay.
    pub util_encoder: u32,
}

macro_rules! nvml_fn {
    ($lib:expr, $name:literal) => {{
        let cname = CString::new($name).unwrap();
        let sym = unsafe { GetProcAddress($lib, cname.as_ptr() as *const u8) };
        match sym {
            Some(p) => Ok(unsafe { mem::transmute(p) }),
            None => Err(anyhow::anyhow!("nvml.dll icinde {} bulunamadi", $name)),
        }
    }};
}

pub struct Gpu {
    lib: HMODULE,
    device: NvmlDevice,
    limit_min_w: f32,
    limit_max_w: f32,
    limit_default_w: f32,

    shutdown: unsafe extern "C" fn() -> NvmlReturn,
    get_memory: unsafe extern "C" fn(NvmlDevice, *mut NvmlMemory) -> NvmlReturn,
    get_util: unsafe extern "C" fn(NvmlDevice, *mut NvmlUtilization) -> NvmlReturn,
    get_temp: unsafe extern "C" fn(NvmlDevice, c_uint, *mut c_uint) -> NvmlReturn,
    get_power: unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> NvmlReturn,
    get_limit: unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> NvmlReturn,
    get_enforced: unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> NvmlReturn,
    set_limit: unsafe extern "C" fn(NvmlDevice, c_uint) -> NvmlReturn,
    get_clock: unsafe extern "C" fn(NvmlDevice, c_uint, *mut c_uint) -> NvmlReturn,
    get_decoder: unsafe extern "C" fn(NvmlDevice, *mut c_uint, *mut c_uint) -> NvmlReturn,
    get_encoder: unsafe extern "C" fn(NvmlDevice, *mut c_uint, *mut c_uint) -> NvmlReturn,
    lock_clocks: unsafe extern "C" fn(NvmlDevice, c_uint, c_uint) -> NvmlReturn,
    reset_clocks: unsafe extern "C" fn(NvmlDevice) -> NvmlReturn,
}

unsafe impl Send for Gpu {}

impl Gpu {
    pub fn open() -> anyhow::Result<Self> {
        let name: Vec<u16> = "nvml.dll".encode_utf16().chain(std::iter::once(0)).collect();
        let lib = unsafe { LoadLibraryW(name.as_ptr()) };
        if lib.is_null() {
            anyhow::bail!("nvml.dll yuklenemedi - NVIDIA surucusu kurulu mu?");
        }

        // Every entry point is resolved up front, so a driver that is too old
        // fails here with a clear message instead of midway through a control
        // loop that is already driving the hardware.
        let init: unsafe extern "C" fn() -> NvmlReturn = nvml_fn!(lib, "nvmlInit_v2")?;
        let get_handle: unsafe extern "C" fn(c_uint, *mut NvmlDevice) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetHandleByIndex_v2")?;
        let get_constraints: unsafe extern "C" fn(NvmlDevice, *mut c_uint, *mut c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetPowerManagementLimitConstraints")?;
        let get_default: unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetPowerManagementDefaultLimit")?;

        let shutdown: unsafe extern "C" fn() -> NvmlReturn = nvml_fn!(lib, "nvmlShutdown")?;
        let get_memory: unsafe extern "C" fn(NvmlDevice, *mut NvmlMemory) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetMemoryInfo")?;
        let get_util: unsafe extern "C" fn(NvmlDevice, *mut NvmlUtilization) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetUtilizationRates")?;
        let get_temp: unsafe extern "C" fn(NvmlDevice, c_uint, *mut c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetTemperature")?;
        let get_power: unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetPowerUsage")?;
        let get_limit: unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetPowerManagementLimit")?;
        let get_enforced: unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetEnforcedPowerLimit")?;
        let set_limit: unsafe extern "C" fn(NvmlDevice, c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceSetPowerManagementLimit")?;
        let get_clock: unsafe extern "C" fn(NvmlDevice, c_uint, *mut c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetClockInfo")?;
        let get_decoder: unsafe extern "C" fn(NvmlDevice, *mut c_uint, *mut c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetDecoderUtilization")?;
        let get_encoder: unsafe extern "C" fn(NvmlDevice, *mut c_uint, *mut c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceGetEncoderUtilization")?;
        let lock_clocks: unsafe extern "C" fn(NvmlDevice, c_uint, c_uint) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceSetGpuLockedClocks")?;
        let reset_clocks: unsafe extern "C" fn(NvmlDevice) -> NvmlReturn =
            nvml_fn!(lib, "nvmlDeviceResetGpuLockedClocks")?;

        if unsafe { init() } != NVML_SUCCESS {
            anyhow::bail!("nvmlInit basarisiz");
        }

        let mut device: NvmlDevice = ptr::null_mut();
        if unsafe { get_handle(0, &mut device) } != NVML_SUCCESS {
            anyhow::bail!("GPU 0 tanitici alinamadi");
        }

        let (mut min_mw, mut max_mw, mut def_mw) = (0u32, 0u32, 0u32);
        unsafe {
            get_constraints(device, &mut min_mw, &mut max_mw);
            get_default(device, &mut def_mw);
        }

        Ok(Self {
            lib,
            device,
            limit_min_w: min_mw as f32 / 1000.0,
            limit_max_w: max_mw as f32 / 1000.0,
            limit_default_w: def_mw as f32 / 1000.0,
            shutdown,
            get_memory,
            get_util,
            get_temp,
            get_power,
            get_limit,
            get_enforced,
            set_limit,
            get_clock,
            get_decoder,
            get_encoder,
            lock_clocks,
            reset_clocks,
        })
    }

    /// Watt range the driver will accept for the power limit.
    pub fn power_limit_range(&self) -> (f32, f32, f32) {
        (self.limit_min_w, self.limit_max_w, self.limit_default_w)
    }

    /// The limit actually in force, which is what we must restore to on exit.
    pub fn enforced_power_limit(&self) -> Option<f32> {
        let mut mw = 0u32;
        if unsafe { (self.get_enforced)(self.device, &mut mw) } == NVML_SUCCESS && mw > 0 {
            Some(mw as f32 / 1000.0)
        } else {
            None
        }
    }

    pub fn sample(&self) -> GpuSample {
        let mut s = GpuSample::default();

        let mut mem = NvmlMemory::default();
        if unsafe { (self.get_memory)(self.device, &mut mem) } == NVML_SUCCESS && mem.total > 0 {
            s.vram_used_mb = mem.used / 1_048_576;
            s.vram_total_mb = mem.total / 1_048_576;
            s.vram_frac = mem.used as f32 / mem.total as f32;
        }

        let mut util = NvmlUtilization::default();
        if unsafe { (self.get_util)(self.device, &mut util) } == NVML_SUCCESS {
            s.util_gpu = util.gpu;
            s.util_mem = util.memory;
        }

        let mut temp = 0u32;
        if unsafe { (self.get_temp)(self.device, NVML_TEMPERATURE_GPU, &mut temp) } == NVML_SUCCESS {
            s.temp_c = temp;
        }

        let mut mw = 0u32;
        if unsafe { (self.get_power)(self.device, &mut mw) } == NVML_SUCCESS {
            s.power_w = mw as f32 / 1000.0;
        }

        let mut limit_mw = 0u32;
        if unsafe { (self.get_limit)(self.device, &mut limit_mw) } == NVML_SUCCESS {
            s.power_limit_w = limit_mw as f32 / 1000.0;
        }

        let (mut util_v, mut period) = (0u32, 0u32);
        if unsafe { (self.get_decoder)(self.device, &mut util_v, &mut period) } == NVML_SUCCESS {
            s.util_decoder = util_v;
        }
        if unsafe { (self.get_encoder)(self.device, &mut util_v, &mut period) } == NVML_SUCCESS {
            s.util_encoder = util_v;
        }

        let mut clk = 0u32;
        if unsafe { (self.get_clock)(self.device, NVML_CLOCK_GRAPHICS, &mut clk) } == NVML_SUCCESS {
            s.clock_graphics_mhz = clk;
        }
        if unsafe { (self.get_clock)(self.device, NVML_CLOCK_MEM, &mut clk) } == NVML_SUCCESS {
            s.clock_mem_mhz = clk;
        }

        s
    }

    /// Set the board power limit, clamped to what the driver allows.
    pub fn set_power_limit(&self, watts: f32) -> anyhow::Result<f32> {
        let clamped = watts.clamp(self.limit_min_w, self.limit_max_w);
        let mw = (clamped * 1000.0) as c_uint;
        let rc = unsafe { (self.set_limit)(self.device, mw) };
        if rc != NVML_SUCCESS {
            anyhow::bail!("power limit {clamped:.0}W yazilamadi (NVML {rc}) - yonetici gerekiyor");
        }
        Ok(clamped)
    }

    /// Pin the graphics clock. Ampere picks the lowest voltage that sustains
    /// the requested clock, so this is how an undervolt is expressed through a
    /// supported API instead of a curve editor.
    pub fn lock_graphics_clock(&self, min_mhz: u32, max_mhz: u32) -> anyhow::Result<()> {
        let rc = unsafe { (self.lock_clocks)(self.device, min_mhz, max_mhz) };
        if rc != NVML_SUCCESS {
            anyhow::bail!("clock lock {min_mhz}-{max_mhz}MHz basarisiz (NVML {rc})");
        }
        Ok(())
    }

    pub fn unlock_graphics_clock(&self) -> anyhow::Result<()> {
        let rc = unsafe { (self.reset_clocks)(self.device) };
        if rc != NVML_SUCCESS {
            anyhow::bail!("clock lock kaldirilamadi (NVML {rc})");
        }
        Ok(())
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        unsafe {
            (self.shutdown)();
            if !self.lib.is_null() {
                FreeLibrary(self.lib);
            }
        }
    }
}

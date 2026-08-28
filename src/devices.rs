//! Known ASUS ACPI device identifiers.
//!
//! These are the addresses Armoury Crate writes to. Not every ROG model
//! implements every one, which is what `rogctl probe` is for: the BIOS answers
//! honestly about what it supports, so we verify against the machine in front
//! of us rather than trusting this table.

// ---------------------------------------------------------------------------
// Addresses confirmed on this machine (G533ZW, BIOS 328) by correlation:
// watching every supported address while the hardware and Armoury Crate moved.
// ---------------------------------------------------------------------------

/// CPU package temperature in degrees C. Swings 49-95 with load and the fans
/// chase it, which is what identified it. This is the sensor that makes
/// driverless CPU thermal control possible under HVCI.
pub const CPU_TEMP: u32 = 0x0012_0094;

/// Current dGPU board power budget in watts. Observed moving 130 -> 150 as
/// Dynamic Boost handed the GPU its extra headroom.
pub const GPU_TGP: u32 = 0x0012_0093;

pub const CPU_FAN_RPM: u32 = 0x0011_0013;
pub const GPU_FAN_RPM: u32 = 0x0011_0014;
pub const CPU_FAN_CURVE: u32 = 0x0011_0024;
pub const GPU_FAN_CURVE: u32 = 0x0011_0025;
pub const PERF_MODE: u32 = 0x0012_0075;

/// Reads 0 while the BIOS is managing the budget itself; writing a value
/// overrides it.
pub const PPT_TOTAL: u32 = 0x0012_00A0;
pub const PPT_SPL: u32 = 0x0012_00A3;

/// A device we know the meaning of, for labelled probe output.
pub struct Device {
    pub id: u32,
    pub name: &'static str,
    pub note: &'static str,
}

pub const KNOWN: &[Device] = &[
    // ---- Thermal / fans -------------------------------------------------
    Device { id: 0x0011_0013, name: "CPU_FAN_RPM",      note: "okunan deger x100 = RPM" },
    Device { id: 0x0011_0014, name: "GPU_FAN_RPM",      note: "okunan deger x100 = RPM" },
    Device { id: 0x0011_0031, name: "MID_FAN_RPM",      note: "3. fan, her modelde yok" },
    Device { id: 0x0011_0024, name: "CPU_FAN_CURVE",    note: "16 bayt: 8 sicaklik + 8 hiz" },
    Device { id: 0x0011_0025, name: "GPU_FAN_CURVE",    note: "16 bayt: 8 sicaklik + 8 hiz" },
    Device { id: 0x0011_0032, name: "MID_FAN_CURVE",    note: "16 bayt: 8 sicaklik + 8 hiz" },
    Device { id: 0x0011_0012, name: "THERMAL_ZONE?",    note: "dogrulanmadi" },

    // ---- Performance mode ----------------------------------------------
    Device { id: 0x0012_0075, name: "PERF_MODE",        note: "0=Dengeli 1=Turbo 2=Sessiz" },

    // ---- CPU power limits (Intel: SPL / SPPT / FPPT) --------------------
    Device { id: 0x0012_00A0, name: "PPT_TOTAL",        note: "toplam paket butcesi (W)" },
    Device { id: 0x0012_00A2, name: "PPT_FPPT",         note: "hizli anlik tavan (W)" },
    Device { id: 0x0012_00A3, name: "PPT_SPL",          note: "surekli limit / PL1 (W)" },
    Device { id: 0x0012_00A4, name: "PPT_SPPT",         note: "kisa sureli limit / PL2 (W)" },
    Device { id: 0x0012_00B5, name: "PPT_CPU_ALT",      note: "bazi modellerde CPU limiti" },

    // ---- GPU ------------------------------------------------------------
    Device { id: 0x0012_0099, name: "NV_DYNAMIC_BOOST", note: "dGPU'ya ek watt (0-25)" },
    Device { id: 0x0012_0098, name: "NV_TEMP_TARGET",   note: "dGPU sicaklik hedefi (C)" },
    Device { id: 0x0009_0020, name: "GPU_ECO_MODE",     note: "1=dGPU kapali (Optimus)" },
    Device { id: 0x0009_0016, name: "GPU_MUX",          note: "0=dGPU direkt 1=Optimus" },
    Device { id: 0x0009_0026, name: "GPU_MUX_VIVO",     note: "alternatif mux adresi" },

    // ---- Power / battery ------------------------------------------------
    Device { id: 0x0012_0057, name: "BATTERY_LIMIT",    note: "sarj tavani %" },
    Device { id: 0x0012_006C, name: "CHARGE_MODE",      note: "sarj davranisi" },

    // ---- Panel / input --------------------------------------------------
    Device { id: 0x0005_0021, name: "KB_BACKLIGHT",     note: "klavye isik seviyesi" },
    Device { id: 0x0006_0078, name: "PANEL_OVERDRIVE",  note: "ekran overdrive" },
    Device { id: 0x0010_0021, name: "SCREEN_?",         note: "dogrulanmadi" },
];

/// Address ranges worth sweeping. `DSTS` is a pure read, so walking these is
/// safe; it tells us what this specific BIOS implements.
pub const SWEEP_RANGES: &[(u32, u32, &str)] = &[
    (0x0005_0000, 0x0005_0040, "0x00050xxx  klavye / aydinlatma"),
    (0x0006_0000, 0x0006_0090, "0x00060xxx  panel / ekran"),
    (0x0009_0000, 0x0009_0040, "0x00090xxx  GPU mux / eco"),
    (0x0010_0000, 0x0010_0040, "0x00100xxx  cesitli"),
    (0x0011_0000, 0x0011_0040, "0x00110xxx  fanlar / termal"),
    (0x0012_0000, 0x0012_00C0, "0x00120xxx  guc / performans"),
];

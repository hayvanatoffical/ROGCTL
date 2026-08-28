//! Arayuzun disariyla konustugu tek katman.
//!
//! Arayuz donanima kendisi dokunmaz. Canli degerleri daemon'un yazdigi
//! `status.txt` dosyasindan okur, ayarlari `rogctl.yaml` uzerinden degistirir,
//! geri kalan islemleri `rogctl.exe`'ye devreder. Sebebi basit: ACPI kolu ve
//! NVML kilidi ayni anda iki surecten tutulursa hangisinin yazdigi belli olmaz.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use rogctl::config::Config;

/// Konsol penceresi acmadan calistir.
const PENCERESIZ: u32 = 0x0800_0000;

/// `rogctl.exe`, `rogctl.yaml` ve `status.txt`'in durdugu klasor.
///
/// Kurulumdan sonra arayuz de ayni klasorde durur, yani ilk aday exe'nin yani
/// basi. Gelistirirken `target\release`'ten calistirildigi icin bir de depo
/// kokundeki `bin` klasorune bakilir.
pub fn kok() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let yan = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    if yan.join("rogctl.exe").exists() {
        return yan;
    }
    // target\release\rogctl-gui.exe -> depo koku -> bin
    let mut p = yan.clone();
    for _ in 0..3 {
        if !p.pop() {
            break;
        }
        let aday = p.join("bin");
        if aday.join("rogctl.exe").exists() {
            return aday;
        }
    }
    yan
}

pub fn rogctl_yolu() -> PathBuf {
    kok().join("rogctl.exe")
}

pub fn ayar_yolu() -> PathBuf {
    kok().join("rogctl.yaml")
}

/// Daemon'un yayinladigi anlik durum. Alanlar `status.txt`'teki adlarla birebir.
#[derive(Debug, Clone, Default)]
pub struct Durum {
    pub yas_s: u64,
    pub calisma_s: u64,
    pub mod_ad: String,
    pub kaynak: String,
    pub batarya: u32,
    pub oyun: String,
    pub cpu_c: u32,
    pub gpu_c: u32,
    pub cpu_util: u32,
    pub gpu_util: u32,
    pub cpu_fan: u32,
    pub gpu_fan: u32,
    pub vram_mb: u32,
    pub vram_toplam_mb: u32,
    pub gpu_w: f32,
    pub tavan_mhz: u32,
    pub gpu_saat_mhz: u32,
    pub fan_araligi: String,
    pub hedef_cpu_c: u32,
    pub hedef_gpu_c: u32,
    pub askidaki_servisler: String,
    pub ram_toplam_mb: u64,
    pub ram_bos_mb: u64,
    pub ram_standby_mb: u64,
    pub ram_kazanilan_mb: u64,
    pub ram_temizlik: u64,
}

impl Durum {
    /// Daemon canli mi. Dosya 10 saniyeden eskiyse yazan durmus demektir.
    pub fn canli(&self) -> bool {
        self.yas_s <= 10
    }

    pub fn oku() -> Option<Self> {
        let govde = std::fs::read_to_string(kok().join("status.txt")).ok()?;
        let m: BTreeMap<&str, &str> = govde.lines().filter_map(|l| l.split_once('=')).collect();
        let s = |k: &str| m.get(k).copied().unwrap_or("").to_string();
        let n32 = |k: &str| m.get(k).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let n64 = |k: &str| m.get(k).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        let f = |k: &str| m.get(k).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);

        let simdi = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let guncelleme = n64("guncelleme");

        Some(Self {
            yas_s: simdi.saturating_sub(guncelleme),
            calisma_s: n64("calisma_s"),
            mod_ad: s("mod"),
            kaynak: s("kaynak"),
            batarya: n32("batarya"),
            oyun: s("oyun"),
            cpu_c: n32("cpu_c"),
            gpu_c: n32("gpu_c"),
            cpu_util: n32("cpu_util"),
            gpu_util: n32("gpu_util"),
            cpu_fan: n32("cpu_fan"),
            gpu_fan: n32("gpu_fan"),
            vram_mb: n32("vram_mb"),
            vram_toplam_mb: n32("vram_toplam_mb"),
            gpu_w: f("gpu_w"),
            tavan_mhz: n32("tavan_mhz"),
            gpu_saat_mhz: n32("gpu_saat_mhz"),
            fan_araligi: s("fan_araligi"),
            hedef_cpu_c: n32("hedef_cpu_c"),
            hedef_gpu_c: n32("hedef_gpu_c"),
            askidaki_servisler: s("askidaki_servisler"),
            ram_toplam_mb: n64("ram_toplam_mb"),
            ram_bos_mb: n64("ram_bos_mb"),
            ram_standby_mb: n64("ram_standby_mb"),
            ram_kazanilan_mb: n64("ram_kazanilan_mb"),
            ram_temizlik: n64("ram_temizlik"),
        })
    }
}

/// Ayarlari diskten oku. Dosya yoksa varsayilanlar yazilir.
pub fn ayar_yukle() -> (Config, Option<String>) {
    Config::load_or_create(&ayar_yolu())
}

pub fn ayar_kaydet(cfg: &Config) -> anyhow::Result<()> {
    cfg.save(&ayar_yolu())
}

/// `rogctl.exe`'yi calistirip ciktisini dondurur.
pub fn rogctl(argumanlar: &[&str]) -> anyhow::Result<String> {
    let exe = rogctl_yolu();
    if !exe.exists() {
        anyhow::bail!("rogctl.exe bulunamadı: {}", exe.display());
    }
    let cikti = komut(&exe.to_string_lossy(), argumanlar)?;
    Ok(cikti)
}

/// PowerShell uzerinden zamanlanmis gorev islemi.
pub fn gorev(islem: &str) -> anyhow::Result<String> {
    komut(
        "powershell",
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!("{islem} -TaskName rogctl -ErrorAction Stop; 'tamam'"),
        ],
    )
}

pub fn gorev_yeniden_baslat() -> anyhow::Result<String> {
    komut(
        "powershell",
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "Stop-ScheduledTask -TaskName rogctl -ErrorAction SilentlyContinue; \
             Start-Sleep -Milliseconds 600; \
             Start-ScheduledTask -TaskName rogctl -ErrorAction Stop; 'tamam'",
        ],
    )
}

fn komut(program: &str, argumanlar: &[&str]) -> anyhow::Result<String> {
    use std::os::windows::process::CommandExt;
    let c = Command::new(program)
        .args(argumanlar)
        .creation_flags(PENCERESIZ)
        .output()?;
    let out = String::from_utf8_lossy(&c.stdout).trim().to_string();
    if c.status.success() {
        Ok(out)
    } else {
        let err = String::from_utf8_lossy(&c.stderr).trim().to_string();
        anyhow::bail!(if err.is_empty() { out } else { err })
    }
}

/// Yonetici olarak mi calisiyoruz. Fan egrisi ve saat kilidi yazmak icin sart.
pub fn yonetici_mi() -> bool {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut yukseltme = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut boyut = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&mut yukseltme as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut boyut,
        );
        windows_sys::Win32::Foundation::CloseHandle(token);
        ok != 0 && yukseltme.TokenIsElevated != 0
    }
}

/// Kendini yonetici olarak yeniden baslatir. Basarili olursa cagiran cikmali.
pub fn yonetici_olarak_yeniden_baslat() -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let genis = |s: &std::ffi::OsStr| -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    };
    let fiil: Vec<u16> = "runas\0".encode_utf16().collect();
    let dosya = genis(exe.as_os_str());

    let sonuc = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            fiil.as_ptr(),
            dosya.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW 32'den buyuk bir deger dondururse baslatma basarili.
    sonuc as isize > 32
}

/// Saniyeyi "3 sa 12 dk" gibi okunur bir sureye cevirir.
pub fn sure_metni(saniye: u64) -> String {
    let s = saniye % 60;
    let d = (saniye / 60) % 60;
    let sa = saniye / 3600;
    if sa > 0 {
        format!("{sa} sa {d} dk")
    } else if d > 0 {
        format!("{d} dk {s} sn")
    } else {
        format!("{s} sn")
    }
}

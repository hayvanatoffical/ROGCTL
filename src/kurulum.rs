//! The setup wizard.
//!
//! Everything this daemon does rests on hardware that may or may not be there:
//! an ASUS BIOS bridge for the fans, NVML for the GPU clock, NVAPI for the
//! frame cap. Any of them can be absent, and until now the way you found out
//! was that some command failed in the middle with a message about a device
//! node.
//!
//! The wizard turns that into one deliberate pass: probe each capability, say
//! plainly which ones this machine has, and write a config that only claims
//! the levers that answered. It is the first thing a new user should run, and
//! the honest answer for a machine that is not an ASUS at all.

use std::io::{BufRead, Write};
use std::path::Path;

use crate::acpi::{Acpi, STATUS_SUPPORTED};
use crate::config::Config;
use crate::devices;
use crate::gpu;
use crate::nvapi;

/// What answered when we asked.
///
/// Each field is a `Result` rather than a `bool` because "not present" and
/// "present but refused" need different advice: the first means this machine
/// is not supported, the second usually means the shell is not elevated.
pub struct Yetenek {
    pub atkacpi: Result<usize, String>,
    pub nvml: Result<(), String>,
    pub nvapi: Result<(), String>,
    pub panel_hz: u32,
    pub vrr: bool,
}

impl Yetenek {
    pub fn olc() -> Self {
        let atkacpi = match Acpi::open() {
            Ok(acpi) => {
                acpi.init().ok();
                // A BIOS answers for every id it implements and clears the
                // supported bit for the rest, so counting the ones that answer
                // is a real measure of how much of this machine we can drive.
                let sayi = devices::KNOWN
                    .iter()
                    .filter(|d| acpi.read(d.id).is_ok_and(|v| v & STATUS_SUPPORTED != 0))
                    .count();
                Ok(sayi)
            }
            Err(e) => Err(e.to_string()),
        };

        let nvml = gpu::Gpu::open().map(|_| ()).map_err(|e| e.to_string());
        let nv = nvapi::NvApi::open();
        let nvapi_durum = nv.as_ref().map(|_| ()).map_err(|e| e.to_string());
        let vrr = nv.as_ref().map(nvapi::vrr_enabled).unwrap_or(false);

        Self {
            atkacpi,
            nvml,
            nvapi: nvapi_durum,
            panel_hz: nvapi::refresh_hz(),
            vrr,
        }
    }

    /// Can the thermal half of the daemon run at all?
    pub fn termal_calisir(&self) -> bool {
        self.atkacpi.is_ok()
    }
}

fn baslik(s: &str) {
    println!();
    println!("{s}");
    println!("{:-<62}", "");
}

/// Read one line, treating a closed stdin as "take the default".
///
/// The wizard has to survive being run from a script or a double click where
/// nothing is typed back; blocking forever on a prompt nobody can see is the
/// one behaviour a setup step must not have.
fn oku() -> Option<String> {
    let mut satir = String::new();
    let _ = std::io::stdout().flush();
    match std::io::stdin().lock().read_line(&mut satir) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(satir.trim().to_string()),
    }
}

fn sec(soru: &str, secenekler: &[&str], varsayilan: usize) -> usize {
    println!();
    println!("{soru}");
    for (i, s) in secenekler.iter().enumerate() {
        let isaret = if i == varsayilan { "*" } else { " " };
        println!("  {isaret} {}) {s}", i + 1);
    }
    print!("Secim [{}]: ", varsayilan + 1);

    match oku() {
        None => {
            println!("{}", varsayilan + 1);
            varsayilan
        }
        Some(s) if s.is_empty() => varsayilan,
        Some(s) => match s.parse::<usize>() {
            Ok(n) if n >= 1 && n <= secenekler.len() => n - 1,
            _ => {
                println!("  (anlasilmadi, varsayilan kullanildi)");
                varsayilan
            }
        },
    }
}

fn evet_mi(soru: &str, varsayilan: bool) -> bool {
    let ipucu = if varsayilan { "E/h" } else { "e/H" };
    print!("{soru} [{ipucu}]: ");
    match oku() {
        None => varsayilan,
        Some(s) if s.is_empty() => varsayilan,
        Some(s) => matches!(s.to_lowercase().as_str(), "e" | "evet" | "y" | "yes"),
    }
}

/// Print what the probe found, in the order a reader needs it.
fn rapor_et(y: &Yetenek) {
    baslik("DONANIM TARAMASI");

    match &y.atkacpi {
        Ok(n) => println!("  [+] ASUS ATKACPI : acildi, {n} aygit destekleniyor"),
        Err(e) => println!("  [-] ASUS ATKACPI : {e}"),
    }
    match &y.nvml {
        Ok(()) => println!("  [+] NVIDIA NVML  : acildi (GPU telemetrisi, saat kilidi)"),
        Err(e) => println!("  [-] NVIDIA NVML  : {e}"),
    }
    match &y.nvapi {
        Ok(()) => println!("  [+] NVIDIA NVAPI : acildi (kare hizi siniri, VSync)"),
        Err(e) => println!("  [-] NVIDIA NVAPI : {e}"),
    }

    if y.panel_hz > 0 {
        let vrr = if y.vrr { "acik" } else { "kapali" };
        println!("  [+] Panel        : {} Hz, VRR (G-SYNC) {vrr}", y.panel_hz);
    } else {
        println!("  [-] Panel        : tazeleme hizi okunamadi");
    }
}

/// The verdict, and what it means for what follows.
fn karar_ver(y: &Yetenek) -> bool {
    baslik("SONUC");

    if y.termal_calisir() {
        println!("  Bu makine tam destekleniyor: fan egrileri, guc modu ve GPU");
        println!("  kollari kullanilabilir.");
        return true;
    }

    println!("  Bu makinede ASUS ATKACPI koprusu yok ya da acilamadi.");
    println!();
    println!("  rogctl fan egrilerini ve guc modunu ASUS'un BIOS arayuzu");
    println!("  uzerinden yaziyor. O arayuz baska markalarda bulunmuyor, yani");
    println!("  termal yonetim bu makinede CALISMAZ.");
    println!();
    println!("  Iki olasilik var:");
    println!("    1. Makine ASUS degil -> desteklenmiyor, kurulumu birak.");
    println!("    2. Makine ASUS ama bu kabuk yonetici degil. ATKACPI yalnizca");
    println!("       yukseltilmis haklarla acilir; yonetici bir PowerShell'den");
    println!("       tekrar dene.");

    false
}

/// Apply the answers to a fresh default config.
///
/// The deltas are deliberately small and named: a wizard that silently invents
/// a tuning nobody can trace is worse than one that leaves the defaults alone,
/// because the defaults here were measured on real hardware.
fn yapilandir(cfg: &mut Config, oncelik: usize, sinir: u32, ram: bool, servis: bool) {
    cfg.memory.enabled = ram;
    cfg.suspend_asus_services = servis;
    cfg.nvidia.max_fps = sinir;

    for (anahtar, env) in cfg.modes.iter_mut() {
        let oyun = anahtar.contains("oyun") || anahtar == "render";
        match oncelik {
            // Performans: oyun modlarinda tavani yukselt, fani serbest birak.
            // Sicaklik hedefi yukselince yonetici saati daha gec kismaya
            // baslar - bu makinede kare hizini en cok etkileyen tek ayar.
            0 if oyun => {
                env.cpu_temp_target = (env.cpu_temp_target + 3).min(95);
                env.gpu_temp_target = (env.gpu_temp_target + 3).min(85);
                env.fan_max_pct = 100;
                env.demand_scaling = false;
            }
            // Sessiz: fan tavanini kis, sicaklik hedefini dusur.
            2 => {
                env.fan_max_pct = ((env.fan_max_pct as u32 * 80) / 100).max(20) as u8;
                env.cpu_temp_target = env.cpu_temp_target.saturating_sub(3);
                env.gpu_temp_target = env.gpu_temp_target.saturating_sub(3);
            }
            _ => {}
        }
    }
}

pub fn kurulum_cmd() -> anyhow::Result<()> {
    println!("rogctl kurulum sihirbazi");
    println!("Once donanim taranir, sonra birkac soru sorulur, en son");
    println!("yapilandirma yazilir. Sorulmadan hicbir sey degistirilmez.");

    let y = Yetenek::olc();
    rapor_et(&y);

    if !karar_ver(&y) {
        println!();
        if !evet_mi("Yine de devam edeyim mi?", false) {
            println!("Kurulum birakildi. Hicbir dosya degistirilmedi.");
            return Ok(());
        }
    }

    baslik("TERCIHLER");

    let oncelik = sec(
        "Bu makineden ne bekliyorsun?",
        &[
            "Performans - sicak ve sesli calissin, kare hizi onemli",
            "Denge      - olculmus varsayilanlar (onerilen)",
            "Sessiz     - serin ve sessiz, kare hizindan odun ver",
        ],
        1,
    );

    // The cap only means something if NVAPI answered; offering to set one on a
    // machine that cannot write it would be a promise the tool cannot keep.
    let sinir = if y.nvapi.is_ok() {
        match sec(
            "Kare hizi siniri nasil olsun?",
            &[
                "Panele gore otomatik (onerilen)",
                "Sinirsiz - kart istedigi kadar kare uretsin",
            ],
            0,
        ) {
            0 => 0,
            _ => u32::MAX,
        }
    } else {
        println!();
        println!("(NVAPI acilmadigi icin kare hizi sorusu atlandi)");
        0
    };

    println!();
    let ram = evet_mi("Oyun baslarken RAM standby listesi temizlensin mi?", true);

    let servis = if y.termal_calisir() {
        println!();
        println!("Armoury Crate calisirken fan egrisini geri yaziyor, yani");
        println!("durdurulmazsa rogctl'in egrisi birkac saniyede siliniyor.");
        evet_mi("ASUS termal servisleri calisirken durdurulsun mu?", true)
    } else {
        false
    };

    let yol = Config::path();
    let mut cfg = Config::default();

    if Path::new(&yol).exists() {
        println!();
        println!("Var olan yapilandirma bulundu: {}", yol.display());
        if !evet_mi("Uzerine yazayim mi? (mevcut dosya yedeklenir)", false) {
            println!("Yapilandirma korundu. Kurulum burada bitti.");
            return Ok(());
        }
        let yedek = yol.with_extension("yaml.eski");
        std::fs::copy(&yol, &yedek).ok();
        println!("  yedek: {}", yedek.display());
    }

    yapilandir(&mut cfg, oncelik, sinir, ram, servis);
    cfg.save(&yol)?;

    baslik("YAZILDI");
    println!("  {}", yol.display());
    println!();
    println!("Siradaki adim: daemon'i kalici hale getirmek icin kurucuyu");
    println!("calistir (kendini yonetici olarak yukseltir):");
    println!();
    println!("  .{}install.ps1", std::path::MAIN_SEPARATOR);
    println!();
    println!("Sonra 'rogctl status' ile calistigini dogrula.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Performance mode must only loosen the game envelopes. An earlier draft
    /// raised every mode's target, which made an idle machine run its fans for
    /// a preference that was only ever about frame rate.
    #[test]
    fn performans_yalnizca_oyun_modlarini_gevsetir() {
        let mut cfg = Config::default();
        let once_bosta = cfg.modes["bosta"].cpu_temp_target;
        let once_aaa = cfg.modes["aaa_oyun"].cpu_temp_target;

        yapilandir(&mut cfg, 0, 0, true, true);

        assert_eq!(cfg.modes["bosta"].cpu_temp_target, once_bosta);
        assert!(cfg.modes["aaa_oyun"].cpu_temp_target > once_aaa);
        assert_eq!(cfg.modes["aaa_oyun"].fan_max_pct, 100);
    }

    /// Quiet mode is the mirror image and must never leave a fan ceiling near
    /// zero behind, which would be a machine that refuses to cool itself.
    #[test]
    fn sessiz_mod_fani_sifirlamaz() {
        let mut cfg = Config::default();
        yapilandir(&mut cfg, 2, 0, false, false);
        for (anahtar, env) in &cfg.modes {
            assert!(env.fan_max_pct >= 20, "{anahtar} fan tavani cok dusuk");
        }
    }

    #[test]
    fn tercihler_yapilandirmaya_islenir() {
        let mut cfg = Config::default();
        yapilandir(&mut cfg, 1, 0, false, false);
        assert!(!cfg.memory.enabled);
        assert!(!cfg.suspend_asus_services);
    }
}

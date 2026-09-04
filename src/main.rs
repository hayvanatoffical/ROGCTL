use rogctl::{acpi, config, kurulum, cooling, control, devices, gpu, memory, nvapi,
             policy, power, process, report, services, telemetry, valorant};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::BOOL;
use windows_sys::Win32::System::Console::{FreeConsole, SetConsoleCtrlHandler};

use acpi::{Acpi, STATUS_SUPPORTED};
use policy::{Classifier, Governor};
use telemetry::Telemetry;

/// Set from the console handler so the main loop can unwind on its own terms
/// and hand the hardware back before the process dies.
static STOP: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn ctrl_handler(_kind: u32) -> BOOL {
    STOP.store(true, Ordering::SeqCst);
    1
}

/// Refuse to start if another instance is already driving the hardware.
///
/// Two controllers would write conflicting fan curves at each other for as
/// long as both lived. The mutex handle is deliberately leaked: it must stay
/// held for the life of the process.
fn claim_single_instance() -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let name: Vec<u16> = "Global\\rogctl_single_instance"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
    if handle.is_null() {
        // Could not arbitrate; allowing the run is friendlier than refusing it.
        return true;
    }
    unsafe { GetLastError() != ERROR_ALREADY_EXISTS }
}

/// Writes to the console when run interactively, to a file when detached.
struct Log {
    file: Option<std::fs::File>,
    path: Option<std::path::PathBuf>,
    bytes_written: u64,
}

/// Size at which the log is rolled over.
///
/// The daemon starts at every logon and writes a line a minute for as long as
/// the machine is on, so an append-only file has no natural end: the first ten
/// days of use produced 600KB. One generation is kept, which is enough to still
/// have yesterday's evidence after a rollover and bounded at twice this figure
/// forever.
const LOG_MAX_BYTES: u64 = 1_500_000;

impl Log {
    fn new(daemon: bool) -> Self {
        if daemon {
            let path = log_path();
            Self::rotate(&path);
            let bytes_written = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .ok();
            Self {
                file,
                path: Some(path),
                bytes_written,
            }
        } else {
            Self {
                file: None,
                path: None,
                bytes_written: 0,
            }
        }
    }

    /// Move an oversized log aside, replacing whatever was set aside before.
    fn rotate(path: &std::path::Path) {
        let too_big = std::fs::metadata(path).is_ok_and(|m| m.len() > LOG_MAX_BYTES);
        if !too_big {
            return;
        }
        Self::force_rotate(path);
    }

    /// Force rollover of the log to `.log.1`, even if under limit.
    fn force_rotate(path: &std::path::Path) {
        if !path.exists() {
            return;
        }
        let previous = path.with_extension("log.1");
        let _ = std::fs::remove_file(&previous);
        let _ = std::fs::rename(path, &previous);
    }

    fn line(&mut self, s: &str) {
        match &mut self.file {
            Some(f) => {
                use std::io::Write;
                let _ = writeln!(f, "{s}");
                let _ = f.flush();
                self.bytes_written = self.bytes_written.saturating_add(s.len() as u64 + 2);
                if self.bytes_written > LOG_MAX_BYTES {
                    if let Some(path) = self.path.as_deref() {
                        self.file = None;
                        Self::force_rotate(path);
                        self.bytes_written = 0;
                        self.file = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                            .ok();
                    }
                }
            }
            None => println!("{s}"),
        }
    }
}

/// Everything `rogctl status` needs to describe the running daemon.
///
/// The daemon owns the hardware, so a second process cannot simply read the
/// state out of it. It publishes a snapshot each tick instead, and `status`
/// reads that.
struct StatusSnapshot<'a> {
    mode: &'a str,
    power: &'a str,
    battery_pct: Option<u8>,
    game: Option<&'a str>,
    sample: &'a telemetry::Sample,
    ema_cpu: f32,
    ema_gpu: f32,
    ceiling_mhz: u32,
    env: &'a policy::Envelope,
    suspended: &'a [String],
    uptime_s: u64,
    mem: &'a memory::Reclaimer,
}

fn kok() -> std::path::PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let yan = exe.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    let in_target = yan.to_string_lossy().contains("\\target\\");
    if !in_target && yan.join("rogctl.exe").exists() {
        return yan;
    }
    let mut p = yan.clone();
    for _ in 0..4 {
        if !p.pop() {
            break;
        }
        let aday = p.join("bin");
        if aday.join("rogctl.exe").exists() {
            return aday;
        }
    }
    let cwd_bin = std::path::PathBuf::from("bin");
    if cwd_bin.join("rogctl.exe").exists() {
        return cwd_bin;
    }
    yan
}

fn sidecar(name: &str) -> std::path::PathBuf {
    kok().join(name)
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn write_status(s: &StatusSnapshot) {
    let body = format!(
        "guncelleme={}\ncalisma_s={}\nmod={}\nkaynak={}\nbatarya={}\noyun={}\n\
         cpu_c={}\ngpu_c={}\ncpu_util={:.0}\ngpu_util={:.0}\n\
         cpu_fan={}\ngpu_fan={}\nvram_mb={}\nvram_toplam_mb={}\ngpu_w={:.1}\n\
         tavan_mhz={}\ngpu_saat_mhz={}\nfan_araligi={}-{}\nhedef_cpu_c={}\nhedef_gpu_c={}\n\
         askidaki_servisler={}\n\
         ram_toplam_mb={}\nram_bos_mb={}\nram_standby_mb={}\nram_standby_dusuk_mb={}\n\
         ram_kazanilan_mb={}\nram_temizlik={}\n",
        now_unix(),
        s.uptime_s,
        s.mode,
        s.power,
        s.battery_pct.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
        s.game.unwrap_or("-"),
        s.sample.cpu_temp_c,
        s.sample.gpu.temp_c,
        s.ema_cpu * 100.0,
        s.ema_gpu * 100.0,
        s.sample.cpu_fan_rpm,
        s.sample.gpu_fan_rpm,
        s.sample.gpu.vram_used_mb,
        s.sample.gpu.vram_total_mb,
        s.sample.gpu.power_w,
        s.ceiling_mhz,
        s.sample.gpu.clock_graphics_mhz,
        s.env.fan_idle_pct,
        s.env.fan_max_pct,
        s.env.cpu_temp_target,
        s.env.gpu_temp_target,
        if s.suspended.is_empty() { "-".to_string() } else { s.suspended.join(",") },
        s.mem.state.total_mb,
        s.mem.state.free_mb,
        s.mem.state.standby_mb,
        s.mem.state.standby_low_mb,
        s.mem.total_freed_mb,
        s.mem.passes,
    );
    let _ = std::fs::write(sidecar("status.txt"), body);
}

/// Where a running daemon publishes its state, wherever it was started from.
///
/// There is only ever one daemon - a named mutex enforces that - so `status`
/// asking about "the daemon" has a single right answer no matter which copy of
/// the binary was typed. Without the installed-path fallback a build-tree copy
/// reports the machine as idle while the real daemon is running a metre away,
/// which is a lie told confidently.
fn status_file() -> std::path::PathBuf {
    let beside = sidecar("status.txt");
    if beside.exists() {
        return beside;
    }
    let installed = std::env::var("USERPROFILE")
        .map(|home| std::path::Path::new(&home).join("rogctl").join("bin").join("status.txt"));
    match installed {
        Ok(p) if p.exists() => p,
        _ => beside,
    }
}

/// Show what the running daemon is doing.
fn status_cmd() -> anyhow::Result<()> {
    let path = status_file();
    let Ok(body) = std::fs::read_to_string(&path) else {
        println!("rogctl calismiyor (durum dosyasi yok).");
        println!("Baslatmak icin:  Start-ScheduledTask rogctl");
        return Ok(());
    };

    let map: BTreeMap<&str, &str> = body
        .lines()
        .filter_map(|l| l.split_once('='))
        .collect();
    let get = |k: &str| map.get(k).copied().unwrap_or("-");

    let age = now_unix().saturating_sub(get("guncelleme").parse::<u64>().unwrap_or(0));
    let alive = age <= 10;

    println!("rogctl  -  {}", if alive { "CALISIYOR" } else { "YANIT YOK" });
    if !alive {
        println!("  son guncelleme {age} saniye once - daemon durmus olabilir.");
    }
    println!("{:-<52}", "");
    println!("  mod              : {}", get("mod"));
    if get("oyun") != "-" {
        println!("  oyun profili     : {}", get("oyun"));
    }
    println!("  guc kaynagi      : {} ({}%)", get("kaynak"), get("batarya"));
    println!("  calisma suresi   : {} sn", get("calisma_s"));
    println!();
    println!("  CPU              : {}C   yuk %{}", get("cpu_c"), get("cpu_util"));
    println!("  GPU              : {}C   yuk %{}   {}W", get("gpu_c"), get("gpu_util"), get("gpu_w"));
    println!("  VRAM             : {} / {} MB", get("vram_mb"), get("vram_toplam_mb"));
    println!("  fanlar           : CPU {} RPM / GPU {} RPM", get("cpu_fan"), get("gpu_fan"));
    println!();
    println!(
        "  RAM bos / standby: {} / {} MB  (dusuk oncelikli {} MB)",
        get("ram_bos_mb"),
        get("ram_standby_mb"),
        get("ram_standby_dusuk_mb")
    );
    println!(
        "  RAM temizligi    : {} kez, toplam {} MB geri kazanildi",
        get("ram_temizlik"),
        get("ram_kazanilan_mb")
    );
    println!();
    println!("  GPU saat / tavan : {} / {} MHz", get("gpu_saat_mhz"), get("tavan_mhz"));
    println!("  fan araligi      : %{}", get("fan_araligi"));
    println!("  hedefler         : CPU {}C / GPU {}C", get("hedef_cpu_c"), get("hedef_gpu_c"));
    println!("  askidaki servis  : {}", get("askidaki_servisler"));

    Ok(())
}

/// Parse the log into an HTML report and open it in the default browser.
fn rapor_cmd() -> anyhow::Result<()> {
    let out = report::run(log_path(), sidecar("rapor.html"))?;
    println!("Rapor olusturuldu: {}", out.display());
    if let Err(e) = std::process::Command::new("cmd")
        .args(["/C", "start", "", &out.display().to_string()])
        .spawn()
    {
        println!("Tarayici otomatik acilamadi ({e}) - dosyayi elle ac.");
    }
    Ok(())
}

/// Inspect or rotate the log file.
fn log_cmd(action: Option<&str>) -> anyhow::Result<()> {
    let path = log_path();
    let prev_path = path.with_extension("log.1");

    match action {
        Some("rotate" | "temizle" | "sifirla" | "devret") => {
            if !path.exists() {
                println!("Log dosyasi bulunamadi: {}", path.display());
                return Ok(());
            }
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            Log::force_rotate(&path);
            println!(
                "Log devredildi: {} ({:.2} MB) -> {}",
                path.display(),
                size as f64 / (1024.0 * 1024.0),
                prev_path.display()
            );
            println!("Yeni log dosyasi sonraki yazmada otomatik olusturulacak.");
        }
        _ => {
            println!("rogctl LOG DURUMU");
            println!("{:-<52}", "");
            println!("  aktif log   : {}", path.display());
            if path.exists() {
                let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let lines = content.lines().count();
                let sessions = content
                    .lines()
                    .filter(|l| l.starts_with("--- rogctl basladi"))
                    .count();
                println!(
                    "  boyut       : {:.2} MB / {:.2} MB limit (%{:.0})",
                    bytes as f64 / (1024.0 * 1024.0),
                    LOG_MAX_BYTES as f64 / (1024.0 * 1024.0),
                    (bytes as f64 / LOG_MAX_BYTES as f64) * 100.0
                );
                println!("  satir sayisi: {lines}");
                println!("  oturum sayisi: {sessions}");
            } else {
                println!("  durum       : Dosya henuz olusmadi");
            }

            println!();
            println!("  onceki log  : {}", prev_path.display());
            if prev_path.exists() {
                let bytes = std::fs::metadata(&prev_path).map(|m| m.len()).unwrap_or(0);
                let content = std::fs::read_to_string(&prev_path).unwrap_or_default();
                let lines = content.lines().count();
                println!(
                    "  boyut       : {:.2} MB ({lines} satir)",
                    bytes as f64 / (1024.0 * 1024.0)
                );
            } else {
                println!("  durum       : Onceki devir yedegi yok");
            }

            if path.exists() {
                println!();
                println!("Son 5 satir:");
                println!("{:-<52}", "");
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let last_lines: Vec<&str> = content.lines().rev().take(5).collect();
                for l in last_lines.into_iter().rev() {
                    println!("  {l}");
                }
            }
            println!();
            println!("Devretmek/sifirlamak icin: rogctl log rotate");
        }
    }
    Ok(())
}

fn log_path() -> std::path::PathBuf {
    sidecar("rogctl.log")
}

fn parse_mode(s: &str) -> Option<policy::Mode> {
    use policy::Mode::*;
    match s.to_lowercase().as_str() {
        "bosta" | "idle" => Some(Idle),
        "film" | "media" => Some(Media),
        "ofis" | "office" => Some(Office),
        "hafif" | "light" => Some(LightGame),
        "aaa" | "game" => Some(AaaGame),
        "render" => Some(Render),
        _ => None,
    }
}

/// List what the NVIDIA driver settings repository exposes on this driver.
///
/// Exists so that no setting id is ever written on faith: the driver names its
/// own settings, and this is where those names are read.
fn nv_list(filter: Option<String>) -> anyhow::Result<()> {
    let nv = nvapi::NvApi::open()?;
    let ids = nv.available_setting_ids();
    println!("Surucunun bildirdigi ayar sayisi: {}", ids.len());
    println!("{:-<74}", "");

    let f = filter.map(|s| s.to_uppercase());
    let mut shown = 0;
    for id in ids {
        let Some(name) = nv.setting_name(id) else { continue };
        if let Some(f) = &f {
            if !name.to_uppercase().contains(f.as_str()) {
                continue;
            }
        }
        let val = match nv.get_u32(id) {
            Some((v, true)) => format!("{v} (varsayilan)"),
            Some((v, false)) => format!("{v} (DEGISTIRILMIS)"),
            None => "-".to_string(),
        };
        println!("  0x{id:08X}  {name:<44} {val}");
        shown += 1;
    }
    if shown == 0 {
        println!("  (eslesen ayar yok)");
    }
    Ok(())
}

/// Measure how well the machine is shedding heat, and compare against a saved
/// reference.
///
/// This is the answer to "should the stand run at 700 or 1400, and is it doing
/// anything at all" - questions no amount of software control could settle,
/// because the dial is mechanical. Run it once per configuration under the same
/// load and the numbers decide.
fn cool_cmd(save: bool, secs: Option<u64>) -> anyhow::Result<()> {
    let window = secs.unwrap_or(60).clamp(10, 900);
    let ref_path = sidecar("cool-ref.txt");

    let mut tel = Telemetry::new()?;
    if tel.gpu.is_none() {
        anyhow::bail!("NVML acilamadi - GPU gucu okunamadan sogutma olculemez");
    }

    println!("{window} saniye olculuyor. Bu sure boyunca yuku DEGISTIRME.");
    println!("(Karsilastirmanin gecerli olmasi icin her iki olcum ayni yuk altinda olmali.)\n");

    let mut acc = cooling::Accumulator::new();
    let start = Instant::now();
    let mut last_print = 0u64;
    // One throwaway sample: CPU utilisation needs a previous reading to
    // difference against, so the first one is always zero.
    tel.sample();
    std::thread::sleep(Duration::from_millis(1000));

    while start.elapsed().as_secs() < window {
        let s = tel.sample();
        acc.push(&s);
        let el = start.elapsed().as_secs();
        if el >= last_print + 10 {
            last_print = el;
            println!(
                "  [{el:>3}s] CPU {}C  GPU {}C  {:.0}W  yuk %{}  fan {}/{}",
                s.cpu_temp_c, s.gpu.temp_c, s.gpu.power_w, s.gpu.util_gpu, s.cpu_fan_rpm, s.gpu_fan_rpm
            );
        }
        std::thread::sleep(Duration::from_millis(1000));
    }

    let r = acc.finish(window);
    println!("\n{:-<58}", "");
    println!("OLCUM  ({} ornek, {} sn)", r.samples, r.secs);
    println!("  CPU              : {:.1}C   yuk %{:.0}", r.cpu_temp, r.cpu_util);
    println!("  GPU              : {:.1}C   yuk %{:.0}   {:.1}W", r.gpu_temp, r.gpu_util, r.gpu_power);
    println!("  fanlar           : CPU {:.0} RPM / GPU {:.0} RPM", r.cpu_fan, r.gpu_fan);
    match r.c_per_w() {
        Some(k) => println!("  termal direnc    : {k:.3} C/W   (dusuk = iyi sogutma)"),
        None => println!("  termal direnc    : olculemedi (GPU cok az guc cekiyor)"),
    }

    if let Some((label, base)) = cooling::Reading::load(&ref_path) {
        println!("\nREFERANSLA KARSILASTIRMA  (\"{label}\")");
        match r.comparable_to(&base) {
            Err(why) => println!("  [!] {why}"),
            Ok(()) => {
                let d = |now: f32, was: f32| {
                    let x = now - was;
                    format!("{:+.1}", x)
                };
                println!("  CPU sicakligi    : {:.1}C -> {:.1}C  ({})", base.cpu_temp, r.cpu_temp, d(r.cpu_temp, base.cpu_temp));
                println!("  GPU sicakligi    : {:.1}C -> {:.1}C  ({})", base.gpu_temp, r.gpu_temp, d(r.gpu_temp, base.gpu_temp));
                println!("  CPU fani         : {:.0} -> {:.0} RPM  ({})", base.cpu_fan, r.cpu_fan, d(r.cpu_fan, base.cpu_fan));
                println!("  GPU fani         : {:.0} -> {:.0} RPM  ({})", base.gpu_fan, r.gpu_fan, d(r.gpu_fan, base.gpu_fan));
                if let (Some(a), Some(b)) = (base.c_per_w(), r.c_per_w()) {
                    println!("  termal direnc    : {a:.3} -> {b:.3} C/W  ({:+.1}%)", (b - a) / a * 100.0);
                }
                println!();
                for line in r.verdict(&base) {
                    println!("  => {line}");
                }
            }
        }
    } else {
        println!("\n(Kayitli referans yok.)");
    }

    if save {
        r.save(&ref_path, "referans")?;
        println!("\nReferans olarak kaydedildi -> {}", ref_path.display());
    } else {
        println!("\nBunu referans yapmak icin:  rogctl cool kaydet [saniye]");
    }
    Ok(())
}

/// Sample the panel's refresh rate over time, from outside the game.
///
/// Every reading so far has been taken at the desktop, and the desktop is not
/// where the complaint is. A game in exclusive fullscreen sets the display mode
/// itself, and if it sets 60Hz then nothing in the driver profile or the game's
/// own settings has to be capping anything - the panel simply is not running at
/// 144 while the game is on screen. That is invisible from a desktop reading and
/// it is the last explanation standing, so it gets measured rather than argued.
fn refresh_watch(seconds: u64) -> anyhow::Result<()> {
    use std::collections::BTreeMap;

    println!("{seconds} saniye boyunca ekran tazeleme hizi orneklenecek.");
    println!("SIMDI oyuna gec ve orada kal. Sure dolunca buraya don.\n");

    let mut seen: BTreeMap<u32, u32> = BTreeMap::new();
    let mut caps: BTreeMap<u32, u32> = BTreeMap::new();
    for _ in 0..seconds {
        std::thread::sleep(std::time::Duration::from_secs(1));
        *seen.entry(nvapi::refresh_hz()).or_insert(0) += 1;
        // The cap the driver is actually holding, sampled alongside, so a cap
        // that moves during the game shows up next to the rate that moved it.
        if let Ok(nv) = nvapi::NvApi::open() {
            if let Some((v, _)) = nv.get_u32(nvapi::FRAME_RATE_LIMITER.0) {
                *caps.entry(v).or_insert(0) += 1;
            }
        }
    }

    println!("{:-<50}", "");
    println!("Olculen tazeleme hizlari (saniye sayisi):");
    for (hz, n) in &seen {
        let label = if *hz == 0 { "okunamadi".into() } else { format!("{hz} Hz") };
        println!("  {label:>12}  {n:>4} sn  {}", "#".repeat((*n).min(40) as usize));
    }
    println!("\nO sirada surucudeki kare siniri:");
    for (cap, n) in &caps {
        println!("  {cap:>12}  {n:>4} sn");
    }

    let low = seen.keys().filter(|&&h| h > 0 && h < 100).copied().next();
    println!("{:-<50}", "");
    match low {
        Some(hz) => {
            println!("BULUNDU: panel oyun sirasinda {hz} Hz'e dusuyor.");
            println!("Kare hizini sinirlayan sey sürücü profili degil, ekran modu.");
            println!("Windows > Ekran > Gelismis ekran ayarlari > Yenileme hizi'ni");
            println!("kontrol et; oyunun tam ekran modu da kendi modunu secebiliyor.");
        }
        None => {
            println!("Panel tum orneklerde yuksek hizda kaldi - sinir ekran modunda degil.");
        }
    }
    Ok(())
}

/// The cap rule, assembled from configuration.
fn cap_policy(cfg: &config::NvidiaCfg) -> nvapi::CapPolicy {
    nvapi::CapPolicy {
        max_fps: cfg.max_fps,
        headroom: cfg.fps_headroom,
        vrr_margin: cfg.vrr_margin,
        refresh_hz: cfg.refresh_hz,
    }
}

/// What the user asked for on the Valorant command line.
#[derive(Clone, Copy, PartialEq)]
enum ValorantMode {
    /// Report only.
    Show,
    /// Every cap set to rogctl's target, so nothing can bind below the panel.
    Cap,
    /// Every cap switched off, in the game and in the driver, so the game runs
    /// at whatever rate it can reach.
    Uncap,
}

/// The driver-side half of `Uncap`.
///
/// The global cap is written to the base profile, which is the weakest entry in
/// the repository: an application profile naming the same setting wins for that
/// executable. So exempting one game does not mean turning rogctl's cap off and
/// hoping - it means writing 0 into Valorant's own profile, where it outranks
/// the global value permanently and without anything having to watch for the
/// game starting.
///
/// This matters more than it sounds. The alternative - detect the process, flip
/// the global cap, flip it back on exit - writes a persistent driver setting
/// from a timer, and this machine has already been bitten once by a cap written
/// at the wrong moment surviving for the entire session.
const DRS_LIMITER_OFF: u32 = 0;

fn valorant_driver_exemption(mode: ValorantMode) -> anyhow::Result<String> {
    let nv = nvapi::NvApi::open()?;
    let id = nvapi::FRAME_RATE_LIMITER.0;
    // Same rule as everywhere else in rogctl: an id is only written once the
    // driver confirms its name.
    if nv.setting_name(id).as_deref() != Some(nvapi::FRAME_RATE_LIMITER.1) {
        anyhow::bail!("surucu Frame Rate Limiter ayarini dogrulamadi - yazilmadi");
    }

    match mode {
        ValorantMode::Uncap => {
            let (profile, applied) = nv.write_u32_in_profile("Valorant", &[(id, DRS_LIMITER_OFF)])?;
            if applied.is_empty() {
                anyhow::bail!("'{profile}' profiline yazilamadi");
            }
            Ok(format!(
                "Surucu: '{profile}' profiline Frame Rate Limiter = 0 yazildi \
                 (genel sinir bu oyun icin gecersiz)"
            ))
        }
        _ => {
            let (profile, removed) = nv.clear_in_profile("Valorant", &[id])?;
            if removed.is_empty() {
                Ok(format!("Surucu: '{profile}' profilinde muafiyet zaten yoktu"))
            } else {
                Ok(format!(
                    "Surucu: '{profile}' profilindeki muafiyet kaldirildi - genel sinir yine gecerli"
                ))
            }
        }
    }
}

/// Show, repair, or lift Valorant's own frame caps.
///
/// This is the one game that gets its own command, because it is the one game
/// that keeps four independent frame caps in a file the driver cannot see. A
/// machine can be right at every level rogctl controls and still be held at 60
/// by a line in that file.
fn valorant_cmd(mode: ValorantMode, every_account: bool) -> anyhow::Result<()> {
    let (cfg, _) = config::Config::load_or_create(&config::Config::path());

    // The target is whatever rogctl would cap the driver at, so the game and the
    // driver are not arguing about two different numbers.
    let policy = cap_policy(&cfg.nvidia);
    let target = match nvapi::NvApi::open() {
        Ok(nv) => policy.resolve(nvapi::effective_vrr(&nv, cfg.nvidia.respect_vrr)),
        Err(_) => policy.refresh(),
    };
    if target == 0 && mode != ValorantMode::Uncap {
        anyhow::bail!("ekran tazeleme hizi okunamadi - hedef kare hizi belirlenemiyor");
    }

    // Editing anything is refused up front rather than per account, so a run
    // cannot fix half the machine and then stop.
    if mode != ValorantMode::Show {
        let blockers = valorant::blocking_processes();
        if !blockers.is_empty() {
            println!("[!] Once Valorant'i TAMAMEN kapat (Riot Client dahil).");
            println!("    Calisiyor: {}", blockers.join(", "));
            println!("    Oyun kapanirken bu dosyayi bellekten yeniden yaziyor;");
            println!("    acikken yapilan degisiklik sessizce siliniyor.");
            anyhow::bail!("Valorant acikken ayar dosyasi guvenle degistirilemez");
        }
    }

    let mut accounts = if every_account {
        valorant::Settings::all()?
    } else {
        vec![valorant::Settings::newest()?]
    };
    if accounts.is_empty() {
        anyhow::bail!("hicbir hesapta RiotUserSettings.ini bulunamadi");
    }

    match mode {
        ValorantMode::Uncap => println!("Mod   : SERBEST - Valorant'ta kare siniri yok"),
        _ => println!("Hedef : {target} fps  (rogctl'in surucuye yazdigi sinirla ayni)"),
    }
    if every_account {
        println!("Kapsam: makinedeki {} hesabin hepsi\n", accounts.len());
    } else {
        println!("Kapsam: en son oynanan hesap  (hepsi icin: rogctl valorant hepsi)\n");
    }

    let mut touched = 0usize;
    for s in accounts.iter_mut() {
        println!("{:-<62}", "");
        println!("Hesap : {}", s.account);
        println!("Dosya : {}", s.path.display());
        for (label, _key, value) in s.caps() {
            let shown = value.unwrap_or_else(|| "(dosyada YOK -> oyunun varsayilani)".to_string());
            println!("  {label}  {shown}");
        }

        let changed = match mode {
            ValorantMode::Show => {
                // An uncapped file still carries its old numbers, so listing
                // them and then complaining that one disagrees with the target
                // would be reporting on caps that cannot fire. The switches are
                // what decide; say so plainly and stop there.
                if s.is_uncapped() {
                    println!("  => SERBEST: hicbir sinir acik degil, oyun serbest calisiyor");
                    continue;
                }
                let bad = s.offenders(target);
                if bad.is_empty() {
                    println!("  => hedefle uyusmayan bir sinir gorunmuyor");
                } else {
                    for b in &bad {
                        println!("  [!] {b}");
                    }
                }
                continue;
            }
            ValorantMode::Cap => s.normalise(target),
            ValorantMode::Uncap => s.uncap(),
        };

        if changed.is_empty() {
            println!("  => degisecek bir sey yok, dosya zaten istenen durumda");
            continue;
        }
        let backup = s.save()?;
        touched += 1;
        for (key, old, new) in &changed {
            let short = key.rsplit("::").next().unwrap_or(key);
            println!("  + {short:<28} {old} -> {new}");
        }
        println!("  Yedek: {}", backup.display());
    }
    println!("{:-<62}", "");

    // The driver half is global, so it happens once rather than per account.
    if mode != ValorantMode::Show {
        match valorant_driver_exemption(mode) {
            Ok(msg) => println!("\n{msg}"),
            Err(e) => println!("\n[!] Surucu tarafi yapilamadi: {e}"),
        }
    }

    match mode {
        ValorantMode::Show => {
            let all = if every_account { "hepsi " } else { "" };
            println!("\nDuzeltmek icin:  rogctl valorant {all}duzelt");
            println!("Siniri tamamen kaldirmak icin:  rogctl valorant {all}serbest");
        }
        ValorantMode::Cap => {
            println!("\n{touched} hesap duzeltildi. Valorant'i simdi acabilirsin.");
        }
        ValorantMode::Uncap => {
            println!("\n{touched} hesap serbest birakildi. Valorant'i simdi acabilirsin.");
            println!("Geri almak icin:  rogctl valorant duzelt");
            println!("\nNot: sinir kalkinca kare hizi tazelemenin cok ustune cikabilir.");
            println!("     Fazladan kare ekranda GORUNMEZ ama ISI ve fan olarak geri");
            println!("     doner - bu makinede CPU zaten 95 C'ye vuruyor ve guc limiti");
            println!("     firmware'de kilitli. Sicakligi 'rogctl mon' ile izle.");
        }
    }
    Ok(())
}

/// Dump every setting a specific driver profile carries itself.
///
/// `nv kim` checks a handful of suspects for the frame rate; this checks all
/// ~129 settings the driver knows about, but only for the one profile named by
/// the filter. It exists for the moment the suspect list turns out to be the
/// wrong list.
fn nv_profile_dump(filter: String) -> anyhow::Result<()> {
    let nv = nvapi::NvApi::open()?;
    let hits = nv.profile_settings(&filter)?;
    if hits.is_empty() {
        println!("'{filter}' iceren, kendi ayari olan bir profil bulunamadi.");
        println!("(Profil hic yoksa ya da hicbir ayari degistirilmemisse bu bos cikar.)");
        return Ok(());
    }
    let mut last = String::new();
    for (profile, setting, value, predefined) in hits {
        if profile != last {
            println!("\n=== {profile} ===");
            last = profile;
        }
        let mark = if predefined { "(stok)" } else { "(DEGISTIRILMIS)" };
        println!("  {value:>10}  {mark:<16} {setting}");
    }
    Ok(())
}

/// Who is actually capping the frame rate?
///
/// `rogctl nv` reads the global profile, which is what rogctl writes - but an
/// application profile naming the same setting overrides it for that executable,
/// and a cap that disagrees with the global one will never show up there. This
/// walks every profile in the repository instead, so the answer to "the game is
/// pinned to a number nobody asked for" is a list rather than a guess.
fn nv_who() -> anyhow::Result<()> {
    let nv = nvapi::NvApi::open()?;

    // Everything that can pin a frame rate, whoever set it.
    let suspects: &[(u32, &str)] = &[
        nvapi::FRAME_RATE_LIMITER,
        nvapi::IDLE_MAX_FPS,
        nvapi::IDLE_TIMEOUT,
        (0x1011_5C8C, "Battery Boost Application FPS"),
        (0x00A8_79CF, "Vertical Sync"),
        (0x10CF_4125, "Override DLSSG Target Frame Rate"),
    ];

    println!("Kare hizini sinirlayabilecek TUM profiller taraniyor...");
    println!("(temel profil global, uygulama profili onu EZER)\n");

    for (id, label) in suspects {
        let hits = match nv.setting_across_profiles(*id) {
            Ok(h) => h,
            Err(e) => {
                println!("  0x{id:08X} {label}: taranamadi ({e})");
                continue;
            }
        };
        // A predefined value in a driver-supplied profile is NVIDIA's stock
        // configuration for that game and is not news. Anything else is.
        let notable: Vec<_> = hits
            .iter()
            .filter(|h| !(h.driver_supplied && h.predefined))
            .collect();

        println!("0x{id:08X}  {label}");
        if notable.is_empty() {
            println!("  hicbir profilde ayarli degil ({} profil tarandi)", hits.len());
        } else {
            for h in notable {
                let who = if h.driver_supplied { "surucu" } else { "YEREL" };
                let stock = if h.predefined { " (stok)" } else { " (DEGISTIRILMIS)" };
                println!("  {:>10}  [{who}]{stock}  {}", h.value, h.profile);
            }
        }
        println!();
    }
    Ok(())
}

/// Apply, or undo, the NVIDIA driver settings rogctl manages.
fn nv_apply(cfg: &config::NvidiaCfg, undo: bool) -> anyhow::Result<()> {
    let nv = nvapi::NvApi::open()?;

    if undo {
        let ids: Vec<u32> = nvapi::MANAGED.iter().map(|(id, _)| *id).collect();
        let results = nv.restore(&ids)?;
        for (id, st) in &results {
            let name = nvapi::MANAGED
                .iter()
                .find(|(m, _)| m == id)
                .map(|(_, n)| *n)
                .unwrap_or("?");
            if *st == 0 {
                println!("  + 0x{id:08X}  {name}  -> surucu varsayilani");
            } else {
                println!("  ! 0x{id:08X}  {name}  -> geri alinamadi (durum {st})");
            }
        }
        return Ok(());
    }

    let (changes, restore_ids, skipped) = nvapi::plan(
        &nv,
        cap_policy(cfg),
        nvapi::effective_vrr(&nv, cfg.respect_vrr),
        cfg.idle_max_fps,
        cfg.idle_timeout_s,
    );
    for s in &skipped {
        println!("  [!] {s}");
    }
    if !restore_ids.is_empty() {
        for (id, st) in nv.restore(&restore_ids)? {
            if st == 0 {
                println!("  + 0x{id:08X} surucu varsayilanina birakildi");
            } else {
                println!("  ! 0x{id:08X} geri alinamadi (durum {st})");
            }
        }
    }
    if changes.is_empty() {
        println!("Uygulanacak dogrulanmis ayar yok.");
        return Ok(());
    }

    let raw = nvapi::refresh_hz();
    let hz = nvapi::snap_refresh(raw);
    if hz != raw {
        println!("Ekran tazeleme hizi: {hz} Hz  (ham okuma {raw}, yuvarlandi)");
    } else {
        println!("Ekran tazeleme hizi: {hz} Hz");
    }
    match (nvapi::vrr_enabled(&nv), cfg.respect_vrr) {
        (true, true) => {
            println!("VRR (G-SYNC): ACIK  -> sinir tazelemenin {} kare ALTINA", cfg.vrr_margin);
            println!("  VRR penceresinin tavani tazeleme hizidir; ustune cikan kare");
            println!("  pencereden duser ve ekran yirtilmasi (tearing) baslar.");
            println!("  Senin +{} kuralini istiyorsan:  respect_vrr: false", cfg.fps_headroom);
        }
        (true, false) => {
            println!("VRR (G-SYNC): ACIK ama YOK SAYILIYOR (respect_vrr: false)");
            println!("  -> sinir tazelemenin +{} USTUNE kuruluyor, senin kuralin.", cfg.fps_headroom);
            println!("  Bedeli: VRR penceresinin ustunde tearing gorebilirsin.");
        }
        (false, _) => {
            println!("VRR (G-SYNC): KAPALI -> sinir tazelemenin +{} USTUNE", cfg.fps_headroom);
        }
    }
    let pairs: Vec<(u32, u32)> = changes.iter().map(|c| (c.id, c.value)).collect();
    let applied = nv.write_u32(&pairs)?;

    println!("{:-<66}", "");
    for c in &changes {
        match applied.iter().find(|(id, _, _)| *id == c.id) {
            Some((_, before, after)) => {
                let was = if *before == u32::MAX { "ayarsiz".to_string() } else { before.to_string() };
                let mark = if *after == c.value { "+" } else { "!" };
                println!("  {mark} {:<46} {was} -> {after}", c.name);
            }
            None => println!("  - {:<46} yazilamadi", c.name),
        }
    }
    println!("\nGeri almak icin:  rogctl nv sifirla");
    Ok(())
}

/// Show where physical memory is, and optionally reclaim some of it.
///
/// The breakdown matters more than the total: Task Manager's "in use" figure
/// lumps the standby list in with real allocations, which is why a machine with
/// nothing running looks full. Splitting standby by priority is what separates
/// cache that is earning its keep from pages read once and never wanted again.
fn mem_cmd(arg: Option<String>) -> anyhow::Result<()> {
    let show = |s: &memory::MemState, tag: &str| {
        println!("{tag}");
        println!("  toplam RAM        : {} MB", s.total_mb);
        println!("  gercekten bos     : {} MB   <- tahsis icin hazir", s.free_mb);
        println!("  standby (onbellek): {} MB", s.standby_mb);
        println!("     dusuk oncelikli: {} MB   <- atilabilir", s.standby_low_mb);
        println!("     normal         : {} MB   <- ise yariyor", s.standby_normal_mb);
        println!("     cekirdek/sicak : {} MB   <- dokunulmaz", s.standby_core_mb);
        let pr: Vec<String> = s
            .standby_by_priority_mb
            .iter()
            .enumerate()
            .map(|(i, v)| format!("p{i}:{v}"))
            .collect();
        println!("     oncelik dagilimi: {}", pr.join("  "));
        println!("  kirli (modified)  : {} MB", s.modified_mb);
        println!("  Windows'a gore    : %{} dolu, {} MB kullanilabilir", s.load_pct, s.avail_mb);
    };

    let Some(before) = memory::MemState::read() else {
        anyhow::bail!("bellek listesi okunamadi");
    };

    match arg.as_deref() {
        None => {
            show(&before, "FIZIKSEL BELLEK");
            println!();
            println!("Not: 'standby' bosa giden bellek degil - bir sey isteyince aninda geri");
            println!("verilir. Sadece dusuk oncelikli kismi atmak bedelsizdir; gerisini atmak");
            println!("diskten yeniden okuma demektir.");
            println!();
            println!("  rogctl mem temizle    dusuk oncelikli standby'i bosalt (guvenli)");
            println!("  rogctl mem tam        tum standby + dosya onbellegi (daha fazla, bedelli)");
            println!("  rogctl mem derin      ustune tum surec calisma kumeleri (masaustu takilir)");
        }
        Some(a) => {
            let depth = match a {
                "temizle" | "low" => memory::Depth::Low,
                "tam" | "full" => memory::Depth::Full,
                "derin" | "deep" => memory::Depth::Deep,
                other => anyhow::bail!("bilinmeyen secenek: {other} (temizle|tam|derin)"),
            };
            show(&before, "ONCESI");
            println!("\n{} temizlik calisiyor...\n", depth.label());
            match memory::reclaim(depth) {
                Some(r) => {
                    show(&r.after, "SONRASI");
                    println!("\n=> gercekten bos bellek {} MB artti", r.freed_mb());
                    if r.freed_mb() == 0 {
                        println!("   (hicbir sey degismediyse: yonetici olarak calistir)");
                    }
                }
                None => println!("temizlik calistirilamadi"),
            }
        }
    }
    Ok(())
}

/// Live sensors change constantly; they belong on a status line, not in the
/// change log, or they drown out the signal we are actually hunting for.
const LIVE_SENSORS: &[u32] = &[0x0011_0013, 0x0011_0014];

/// The command list, shared by `rogctl yardim` and by an unrecognised
/// command. One text, so the two can never drift apart.
fn usage() -> String {
    "Kullanim: rogctl <komut>\n\
     \n\
     GUNLUK\n\
       status                 calisan daemon'in anlik durumu\n\
       log [rotate|temizle]   log durumu, boyutu ve devretme\n\
       rapor                  log'dan HTML rapor uret ve tarayicida ac\n\
       mon [saniye]           canli telemetri\n\
       plan                   her modun yazacagi fan egrisini goster\n\
     \n\
     KARE HIZI\n\
       nv [uygula|sifirla]    NVIDIA surucu ayarlari\n\
       nv kim                 kare hizini hangi profil siniriyor\n\
       nv izle [sn]           oyun sirasinda tazeleme hizini olc\n\
       nv profil <ad>         bir profilin TUM ayarlarini dok\n\
       valorant               Valorant'in KENDI fps sinirlarini goster\n\
       valorant duzelt        sinirlari tazeleme hizina esitle\n\
       valorant serbest       sinirlari tamamen kaldir\n\
     hepsi (ek kelime)      son hesap yerine TUM hesaplara uygula\n\
     \n\
     ELLE MUDAHALE\n\
       force <mod> [sn]       modu dayat (bosta|film|ofis|hafif|aaa|render)\n\
       mem [temizle]          RAM dagilimi / standby temizligi\n\
       cool [kaydet] [sn]     sogutma verimi olcumu (stand A/B testi)\n\
     \n\
     CALISTIRMA\n\
       daemon                 arka planda surekli calistir\n\
       run [saniye]           onplanda calistir\n\
     \n\
     teshis: probe | watch | curves | gpu | selftest | ppt | perf | setdev".to_string()
}

fn main() {
    // Bare `rogctl` prints the command list. It used to run the hardware probe,
    // which is a diagnostic that reads every ACPI method on the machine - a
    // reasonable default while this was a scratch tool, and a startling one for
    // anybody typing the name to find out what it does.
    let cmd = std::env::args().nth(1).unwrap_or_else(|| "yardim".to_string());

    let result = match cmd.as_str() {
        "kurulum" | "setup" => kurulum::kurulum_cmd(),
        "probe" => probe(),
        "watch" => watch(),
        "curves" => curves(),
        "gpu" => gpu_status(),
        "mon" => mon(std::env::args().nth(2).and_then(|s| s.parse().ok())),
        "selftest" => selftest(),
        "ppt" => ppt_cmd(std::env::args().nth(2).and_then(|s| s.parse().ok())),
        "perf" => perf_cmd(std::env::args().nth(2).and_then(|s| s.parse().ok())),
        "setdev" => setdev_cmd(),
        "plan" => plan_cmd(),
        "status" => status_cmd(),
        "log" => log_cmd(std::env::args().nth(2).as_deref()),
        "rapor" => rapor_cmd(),
        "mem" => mem_cmd(std::env::args().nth(2)),
        "cool" => {
            let a2 = std::env::args().nth(2);
            let save = a2.as_deref() == Some("kaydet");
            let secs = if save { std::env::args().nth(3) } else { a2 };
            cool_cmd(save, secs.and_then(|s| s.parse().ok()))
        }
        "nv" => match std::env::args().nth(2).as_deref() {
            Some("uygula") => {
                let (cfg, _) = config::Config::load_or_create(&config::Config::path());
                nv_apply(&cfg.nvidia, false)
            }
            Some("sifirla") => {
                let (cfg, _) = config::Config::load_or_create(&config::Config::path());
                nv_apply(&cfg.nvidia, true)
            }
            Some("kim") => nv_who(),
            Some("profil") => nv_profile_dump(
                std::env::args().nth(3).unwrap_or_else(|| "VALORANT".to_string()),
            ),
            Some("izle") => refresh_watch(
                std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(30),
            ),
            other => nv_list(other.map(|s| s.to_string())),
        },
        // Both words are optional and order does not matter, because
        // "valorant hepsi duzelt" and "valorant duzelt hepsi" are the same
        // request and getting told off for the order is pure friction.
        "valorant" => {
            let words: Vec<String> = std::env::args().skip(2).collect();
            let has = |w: &str| words.iter().any(|a| a.eq_ignore_ascii_case(w));
            let mode = if has("serbest") {
                ValorantMode::Uncap
            } else if has("duzelt") || has("sinirli") {
                ValorantMode::Cap
            } else {
                ValorantMode::Show
            };
            valorant_cmd(mode, has("hepsi"))
        }
        "run" => run(std::env::args().nth(2).and_then(|s| s.parse().ok()), None, false),
        "daemon" => run(None, None, true),
        "force" => {
            let m = std::env::args().nth(2).and_then(|s| parse_mode(&s));
            match m {
                Some(mode) => run(std::env::args().nth(3).and_then(|s| s.parse().ok()), Some(mode), false),
                None => {
                    eprintln!("Kullanim: rogctl force <bosta|film|ofis|hafif|aaa|render> [saniye]");
                    std::process::exit(2);
                }
            }
        }
        "yardim" | "help" | "-h" | "--help" | "/?" => {
            println!("{}", usage());
            return;
        }
        other => {
            eprintln!("Bilinmeyen komut: {other}");
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    };

    if let Err(e) = result {
        eprintln!("\nHATA: {e:#}");
        std::process::exit(1);
    }
}

/// Walk the known address ranges and collect everything the BIOS implements.
fn supported_addresses(acpi: &Acpi) -> Vec<u32> {
    let mut found = Vec::new();
    for (start, end, _) in devices::SWEEP_RANGES {
        for id in *start..=*end {
            if let Ok(raw) = acpi.read(id) {
                if raw & STATUS_SUPPORTED != 0 {
                    found.push(id);
                }
            }
        }
    }
    found
}

fn label_for(id: u32) -> &'static str {
    devices::KNOWN
        .iter()
        .find(|d| d.id == id)
        .map(|d| d.name)
        .unwrap_or("(isimsiz)")
}

/// Interrogate the BIOS about what it actually implements. Read-only.
fn probe() -> anyhow::Result<()> {
    println!("rogctl yetenek taramasi");
    println!("=======================\n");

    let acpi = Acpi::open()?;
    println!("[+] \\\\.\\ATKACPI acildi");
    match acpi.init() {
        Ok(v) => println!("[+] BIOS INIT el sikismasi: 0x{v:08X}\n"),
        Err(e) => println!("[!] INIT basarisiz ({e})\n"),
    }

    println!("{:-<86}", "");
    println!("{:<12} {:<20} {:>10}  NOT", "ID", "AD", "DEGER");
    println!("{:-<86}", "");

    for d in devices::KNOWN {
        match acpi.read(d.id) {
            Ok(raw) if raw & STATUS_SUPPORTED != 0 => {
                println!("+ 0x{:08X} {:<20} {:>10}  {}", d.id, d.name, raw & 0xFFFF, d.note);
            }
            Ok(_) => println!("- 0x{:08X} {:<20} {:>10}  {}", d.id, d.name, "-", d.note),
            Err(_) => println!("! 0x{:08X} {:<20} {:>10}  BIOS'ta yok", d.id, d.name, "-"),
        }
    }

    println!("\nTAM TARAMA (salt okunur)");
    println!("{:-<86}", "");
    for (start, end, label) in devices::SWEEP_RANGES {
        let hits: Vec<_> = (*start..=*end)
            .filter_map(|id| match acpi.read(id) {
                Ok(raw) if raw & STATUS_SUPPORTED != 0 => Some((id, raw)),
                _ => None,
            })
            .collect();
        println!("\n{label}  ->  {} destekli adres", hits.len());
        for (id, raw) in hits {
            println!("    0x{id:08X}  ham=0x{raw:08X}  deger={:<6} {}", raw & 0xFFFF, label_for(id));
        }
    }

    Ok(())
}

/// Decode the eight-point fan curves the BIOS is currently running.
fn curves() -> anyhow::Result<()> {
    let acpi = Acpi::open()?;
    acpi.init().ok();

    for (id, name) in [(0x0011_0024u32, "CPU"), (0x0011_0025u32, "GPU")] {
        let blob = acpi.read_blob(id)?;
        println!("\n{name} FAN EGRISI  (0x{id:08X})  -  {} bayt dondu", blob.len());

        // Dump the raw reply before interpreting it. The curve layout has to be
        // confirmed from the bytes themselves, not assumed, because we write
        // back to this same address later.
        print!("  ham:");
        for (i, b) in blob.iter().take(32).enumerate() {
            if i % 16 == 0 {
                print!("\n    {i:02}: ");
            }
            print!("{b:02X} ");
        }
        println!();

        if blob.len() < 16 {
            println!("  ham: {blob:02X?}");
            println!("  (16 bayttan kisa - egri bu adresten okunamiyor)");
            continue;
        }

        // The final speed byte reads 0x64 = 100, which pins the scale: fan
        // speed is a straight percentage, not a 0-255 fraction.
        println!("  {:>8} {:>8}", "SICAKLIK", "HIZ");
        for i in 0..8 {
            println!("  {:>7}C {:>7}%", blob[i], blob[i + 8]);
        }
    }

    Ok(())
}

/// Print the curve every mode would write, without touching the hardware.
///
/// The curve read back from the BIOS is not a reliable mirror of what we sent,
/// so this shows the intent directly - useful both for checking the generator
/// and for deciding what to change.
fn plan_cmd() -> anyhow::Result<()> {
    let path = config::Config::path();
    let (cfg, note) = config::Config::load_or_create(&path);
    println!("config: {}", path.display());
    if let Some(n) = note {
        println!("[i] {n}");
    }

    for mode in policy::Mode::ALL {
        let env = cfg.envelope(mode);
        println!(
            "\n{}  (fan {}-{}%, diz {}C, GPU {}-{}MHz, talep kismasi: {})",
            mode.label().to_uppercase(),
            env.fan_idle_pct,
            env.fan_max_pct,
            env.fan_knee_c,
            env.gpu_clock_floor_mhz,
            env.gpu_clock_ceiling_mhz,
            if env.demand_scaling { "acik" } else { "kapali" }
        );
        for (name, target) in [("CPU", env.cpu_temp_target), ("GPU", env.gpu_temp_target)] {
            let c = control::build_curve(&env, target);
            let pts: Vec<String> = (0..8).map(|i| format!("{}C:{}%", c[i], c[i + 8])).collect();
            println!("  {name} (hedef {target}C)  {}", pts.join("  "));
        }
    }
    Ok(())
}

/// Write an arbitrary ACPI device, then watch the fans for a few seconds.
///
/// A workbench tool for identifying unnamed addresses: the sweep says which
/// exist, but only writing one and watching what moves says what it means.
fn setdev_cmd() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Kullanim: rogctl setdev <0xID> <deger>");
        eprintln!("DIKKAT: dogrudan ACPI yazar. Sadece ne yaptigini bildigin adreslerde kullan.");
        std::process::exit(2);
    }
    let id = u32::from_str_radix(args[2].trim_start_matches("0x"), 16)?;
    let val: u32 = args[3].parse()?;

    let acpi = Acpi::open()?;
    acpi.init().ok();

    println!("oncesi  0x{id:08X} = {}", acpi.read(id).map(|v| v & 0xFFFF).unwrap_or(0));
    match acpi.write(id, val) {
        Ok(rc) => println!("yazildi 0x{id:08X} <- {val} (rc=0x{rc:08X})"),
        Err(e) => println!("yazilamadi: {e}"),
    }

    println!("\n{:>4} {:>8} {:>10} {:>10}", "sn", "deger", "CPU fan", "GPU fan");
    for i in 0..10 {
        std::thread::sleep(Duration::from_millis(1000));
        println!(
            "{:>4} {:>8} {:>10} {:>10}",
            i + 1,
            acpi.read(id).map(|v| v & 0xFFFF).unwrap_or(0),
            acpi.read(devices::CPU_FAN_RPM).map(|v| (v & 0xFFFF) * 100).unwrap_or(0),
            acpi.read(devices::GPU_FAN_RPM).map(|v| (v & 0xFFFF) * 100).unwrap_or(0),
        );
    }
    Ok(())
}

/// Read or set the ASUS thermal policy, and show what it does to the curves.
///
/// Each policy carries its own fan curve in the BIOS, so switching modes can
/// quietly discard whatever we wrote. Printing the curve on either side of the
/// switch is how we find out whether ours survives.
fn perf_cmd(mode: Option<u32>) -> anyhow::Result<()> {
    let acpi = Acpi::open()?;
    acpi.init().ok();

    let show = |tag: &str| {
        if let Ok(b) = acpi.read_blob(devices::CPU_FAN_CURVE) {
            if b.len() >= 16 {
                println!("  {tag} egri: sic {:?} hiz {:?}", &b[..8], &b[8..16]);
            }
        }
    };

    match mode {
        None => {
            let cur = acpi.read(devices::PERF_MODE).map(|v| v & 0xFFFF).unwrap_or(9999);
            println!("PERF_MODE = {cur}  (0=Dengeli 1=Turbo 2=Sessiz 3=Manuel)");
            show("mevcut");
        }
        Some(m) => {
            show("oncesi");
            match acpi.write(devices::PERF_MODE, m) {
                Ok(rc) => println!("PERF_MODE <- {m} (rc=0x{rc:08X})"),
                Err(e) => println!("PERF_MODE yazilamadi: {e}"),
            }
            std::thread::sleep(Duration::from_millis(500));
            let got = acpi.read(devices::PERF_MODE).map(|v| v & 0xFFFF).unwrap_or(9999);
            println!("geri okuma = {got}");
            show("sonrasi");
        }
    }
    Ok(())
}

/// Read or set the CPU package budget directly.
///
/// Exists to settle a question the readback cannot answer on its own: these
/// addresses report 0 while the BIOS is managing the budget, so a write that
/// silently does nothing looks identical to one that worked. The only honest
/// test is to set a low budget, apply load, and see whether the chip obeys.
fn ppt_cmd(watts: Option<u32>) -> anyhow::Result<()> {
    let acpi = Acpi::open()?;
    acpi.init().ok();

    match watts {
        None => {
            for (id, name) in [(devices::PPT_TOTAL, "PPT_TOTAL"), (devices::PPT_SPL, "PPT_SPL")] {
                match acpi.read(id) {
                    Ok(raw) => println!("{name} (0x{id:08X}) = {}  (0 = BIOS yonetiyor)", raw & 0xFFFF),
                    Err(e) => println!("{name}: okunamadi ({e})"),
                }
            }
        }
        Some(w) => {
            println!("PPT <- {w}W yaziliyor...");
            match acpi.write(devices::PPT_TOTAL, w) {
                Ok(rc) => println!("  PPT_TOTAL yazildi (rc=0x{rc:08X})"),
                Err(e) => println!("  PPT_TOTAL yazilamadi: {e}"),
            }
            match acpi.write(devices::PPT_SPL, w) {
                Ok(rc) => println!("  PPT_SPL yazildi (rc=0x{rc:08X})"),
                Err(e) => println!("  PPT_SPL yazilamadi: {e}"),
            }
            for (id, name) in [(devices::PPT_TOTAL, "PPT_TOTAL"), (devices::PPT_SPL, "PPT_SPL")] {
                if let Ok(raw) = acpi.read(id) {
                    println!("  geri okuma {name} = {}", raw & 0xFFFF);
                }
            }
        }
    }
    Ok(())
}

/// The daemon: sample, classify, drive, repeat.
///
/// `secs` bounds the run for supervised testing; without it this runs until
/// interrupted.
fn run(secs: Option<u64>, forced: Option<policy::Mode>, daemon: bool) -> anyhow::Result<()> {
    unsafe { SetConsoleCtrlHandler(Some(ctrl_handler), 1) };

    if !claim_single_instance() {
        eprintln!("rogctl zaten calisiyor - bu ornek cikiyor.");
        return Ok(());
    }

    let mut log = Log::new(daemon);
    let mut tel = Telemetry::new()?;
    // NVML never opening and NVML opening but refusing a sample are different
    // faults with the same symptom. Reporting the first here marks the
    // per-sample warning as already said, so one cause never prints two lines.
    let nvml_missing = tel.gpu.is_none();
    if nvml_missing {
        log.line("[!] NVML acilamadi - GPU telemetrisi ve clock lock devre disi.");
    }

    let cfg_path = config::Config::path();
    let (cfg, cfg_note) = config::Config::load_or_create(&cfg_path);

    let mut controller = control::Controller::new(&tel.acpi, tel.gpu.as_ref());
    let mut classifier = Classifier::new(cfg.escalate_ticks, cfg.relax_ticks);
    // Worth saying exactly once per run rather than once per tick, and not at
    // all when the line above already explained it.
    let mut gpu_warned = nvml_missing;
    let mut governor = Governor::new();

    match forced {
        Some(m) => log.line(&format!("--- rogctl basladi, DAYATILMIS mod: {} ---", m.label())),
        None => log.line("--- rogctl basladi (otomatik mod) ---"),
    }
    if let Some(n) = cfg_note {
        log.line(&format!("[i] {n}"));
    }
    log.line(&format!("[i] config: {}", cfg_path.display()));

    // Driver profile settings persist across reboots, so this is idempotent
    // after the first run - but re-asserting it means the frame cap follows the
    // panel if the display mode ever changes.
    // What the frame cap was actually left at, so the periodic re-check below
    // can tell a real change from a repeat of itself.
    let mut applied_cap: u32 = 0;
    if cfg.nvidia.enabled {
        match nvapi::NvApi::open() {
            Ok(nv) => {
                let vrr = nvapi::effective_vrr(&nv, cfg.nvidia.respect_vrr);
                log.line(&format!(
                    "[i] VRR (G-SYNC) {} -> kare siniri tazeleme hizinin {}",
                    if nvapi::vrr_enabled(&nv) {
                        if cfg.nvidia.respect_vrr { "acik" } else { "acik (yok sayiliyor)" }
                    } else {
                        "kapali"
                    },
                    if vrr { "ALTINA kurulur" } else { "USTUNE kurulur" }
                ));
                let (changes, restore_ids, skipped) = nvapi::plan(
                    &nv,
                    cap_policy(&cfg.nvidia),
                    vrr,
                    cfg.nvidia.idle_max_fps,
                    cfg.nvidia.idle_timeout_s,
                );
                for s in skipped {
                    log.line(&format!("[!] NVIDIA {s}"));
                }
                if !restore_ids.is_empty() {
                    if let Ok(results) = nv.restore(&restore_ids) {
                        for (id, st) in results {
                            if st != 0 {
                                log.line(&format!(
                                    "[!] NVIDIA 0x{id:08X} varsayilana dondurulemedi (durum {st})"
                                ));
                            }
                        }
                    }
                }
                if let Some(c) = changes
                    .iter()
                    .find(|c| c.id == nvapi::FRAME_RATE_LIMITER.0)
                {
                    applied_cap = c.value;
                }
                let pairs: Vec<(u32, u32)> = changes.iter().map(|c| (c.id, c.value)).collect();
                match nv.write_u32(&pairs) {
                    Ok(applied) => {
                        for (id, before, after) in applied {
                            if before != after {
                                let name = changes.iter().find(|c| c.id == id).map(|c| c.name).unwrap_or("?");
                                let was = if before == u32::MAX { "ayarsiz".into() } else { before.to_string() };
                                log.line(&format!("[+] NVIDIA {name}: {was} -> {after}"));
                            }
                        }
                    }
                    Err(e) => log.line(&format!("[!] NVIDIA ayarlari yazilamadi: {e}")),
                }
            }
            Err(e) => log.line(&format!("[!] NVAPI acilamadi: {e}")),
        }
    }

    log.line(&format!(
        "{:>5} {:>11} {:>6} {:>6} {:>6} {:>6} {:>7} {:>7}",
        "t", "MOD", "CPU C", "GPU C", "CPU%", "GPU%", "TAVAN", "FAN"
    ));

    // Hold down the services that would otherwise rewrite our curve. Requires
    // elevation; unprivileged runs simply keep fighting Armoury Crate.
    let suspended = if cfg.suspend_asus_services {
        let s = services::suspend();
        log.line(&format!("[i] servisler -> {}", s.summary()));
        if !s.failed.is_empty() {
            log.line("[!] Durdurulamayan servis var - yonetici olarak calismiyor olabiliriz, egriler ezilebilir.");
        }
        s
    } else {
        services::Suspension::default()
    };
    let services_down = suspended.down();

    // Detach from the console last, so any startup failure above is still
    // visible to whoever launched us.
    if daemon {
        unsafe { FreeConsole() };
    }

    let start = Instant::now();
    let mut tick: u64 = 0;

    // Enumerating processes and querying power costs far more than reading a
    // sensor, and neither changes meaningfully within a second.
    const SLOW_POLL_TICKS: u64 = 5;
    let mut procs = process::running_names();
    let mut power = power::read();
    let mut last_game: Option<String> = None;

    let mut reclaimer = memory::Reclaimer::new(cfg.memory);
    // A game allocating several gigabytes has to take those pages from the
    // standby list, and that repurposing happens at fault time - a stutter.
    // Crossing this line either way is the moment reclaiming is worth doing.
    let heavy = |m: policy::Mode| m.intensity() >= policy::Mode::LightGame.intensity();
    let mut was_heavy: Option<bool> = None;

    // The frame cap is derived from the panel's refresh rate, and the panel is
    // not necessarily at its final mode when the daemon starts.
    //
    // This was measured going wrong: launched at logon, the display still read
    // 60Hz, so a 60Hz-derived cap was written to the driver profile - and DRS
    // settings are permanent, so the machine kept that cap for the rest of the
    // session even after the panel switched to 144Hz. Re-reading on a slow
    // timer means a bad early reading corrects itself within a minute instead
    // of lasting until the next restart.
    const REFRESH_RECHECK_TICKS: u64 = 60;
    // Raising the cap is safe to do on one reading; lowering it is not. A
    // reading can dip to 60 for reasons that have nothing to do with the panel's
    // real mode - the screen blanking, the lock screen, a game taking the
    // display exclusively - and because DRS is permanent, one bad dip would pin
    // the machine low until something read high again. Two agreeing readings a
    // minute apart is enough to tell a real mode change from a blink.
    const LOWER_CONFIRMATIONS: u32 = 2;
    let mut pending_lower: Option<(u32, u32)> = None;

    while !STOP.load(Ordering::SeqCst) {
        if tick.is_multiple_of(SLOW_POLL_TICKS) {
            procs = process::running_names();
            power = power::read();
        }

        if cfg.nvidia.enabled && tick > 0 && tick.is_multiple_of(REFRESH_RECHECK_TICKS) {
            match nvapi::NvApi::open() {
                Ok(nv) => {
                    let want = cap_policy(&cfg.nvidia)
                        .resolve(nvapi::effective_vrr(&nv, cfg.nvidia.respect_vrr));
                    // A lower cap has to be seen twice running before it is
                    // believed; anything else resets the count.
                    let confirmed = if want == 0 || want == applied_cap {
                        pending_lower = None;
                        false
                    } else if want > applied_cap {
                        pending_lower = None;
                        true
                    } else {
                        let seen = match pending_lower {
                            Some((v, n)) if v == want => n + 1,
                            _ => 1,
                        };
                        pending_lower = Some((want, seen));
                        seen >= LOWER_CONFIRMATIONS
                    };
                    if confirmed {
                        match nv.write_u32(&[(nvapi::FRAME_RATE_LIMITER.0, want)]) {
                            Ok(_) => {
                                log.line(&format!(
                                    "[+] NVIDIA kare siniri {applied_cap} -> {want} (ekran tazeleme hizi degisti)"
                                ));
                                applied_cap = want;
                                pending_lower = None;
                            }
                            Err(e) => {
                                log.line(&format!("[!] NVIDIA kare siniri guncellenemedi: {e}"))
                            }
                        }
                    }
                }
                Err(e) => log.line(&format!("[!] NVIDIA kare siniri okunamadi: {e}")),
            }
        }

        let s = tel.sample();

        // Without NVML there is no GPU utilisation, no VRAM figure and no
        // decoder counter - and the classifier reads all three as zero rather
        // than as missing, which looks exactly like an idle machine. A game
        // would then run in the quietest envelope with the fans down, and
        // nothing on screen would say why. Reported once, because a driver that
        // is not answering now will not answer on the next tick either.
        if !s.gpu_ok && !gpu_warned {
            gpu_warned = true;
            log.line(
                "[!] GPU telemetrisi okunamiyor (NVML). Yuk ve VRAM 0 gorunecegi icin                  siniflandirma BOSTA'ya sapabilir; fan egrisi CPU sicakligina gore                  calismaya devam eder. NVIDIA surucusu kurulu mu?",
            );
        }

        classifier.update(&s);

        // A matching game profile outranks the classifier: the process being
        // there is harder evidence than any utilisation pattern.
        let game = cfg.match_game(&procs);
        let classified = match forced {
            Some(m) => m,
            None => game
                .and_then(|g| g.mode.as_deref())
                .and_then(policy::Mode::from_key)
                .unwrap_or_else(|| classifier.current()),
        };

        // On battery, refuse to escalate past the configured cap.
        let mode = match cfg.battery.cap_mode.as_deref().and_then(policy::Mode::from_key) {
            Some(cap) if power.on_battery() && classified.intensity() > cap.intensity() => cap,
            _ => classified,
        };

        let mut env = cfg.envelope(mode);
        if let Some(g) = game {
            env = g.overrides.apply(env);
        }
        if power.on_battery() {
            env.fan_max_pct =
                ((env.fan_max_pct as f32 * cfg.battery.fan_max_scale) as u8).max(env.fan_idle_pct);
            env.gpu_clock_ceiling_mhz =
                (env.gpu_clock_ceiling_mhz as f32 * cfg.battery.clock_ceiling_scale) as u32;
            env.gpu_clock_floor_mhz = env.gpu_clock_floor_mhz.min(env.gpu_clock_ceiling_mhz);
        }

        let changed = controller.apply_mode(&tel.acpi, tel.gpu.as_ref(), mode, &env);
        if changed {
            governor.reset_to(env.gpu_clock_ceiling_mhz);
            let via = match game {
                Some(g) => format!(" [profil: {}]", g.process),
                None => String::new(),
            };
            log.line(&format!(
                "[{:>5.0}s] --> mod: {}{}  ({}, fan {}-{}%, GPU {}-{}MHz, hedef CPU {}C / GPU {}C)",
                start.elapsed().as_secs_f32(),
                mode.label(),
                via,
                power.label(),
                env.fan_idle_pct,
                env.fan_max_pct,
                env.gpu_clock_floor_mhz,
                env.gpu_clock_ceiling_mhz,
                env.cpu_temp_target,
                env.gpu_temp_target
            ));
        }

        // Physical memory. Only checked on the slow poll - the standby list does
        // not move meaningfully within a second, and the mode shift that matters
        // most is evaluated on the tick it happens.
        let shifted = was_heavy.map(|w| w != heavy(mode)).unwrap_or(false);
        if shifted || tick.is_multiple_of(SLOW_POLL_TICKS) {
            if let Some(line) = reclaimer.tick(shifted) {
                log.line(&format!("[{:>5.0}s] {line}", start.elapsed().as_secs_f32()));
            }
        }
        was_heavy = Some(heavy(mode));

        // Something else may still overwrite the curves; put them back on a timer.
        if cfg.curve_refresh_ticks > 0 && tick.is_multiple_of(cfg.curve_refresh_ticks) {
            controller.refresh_curves(&tel.acpi, &env);
        }

        let ceiling = governor.update(&s, &env);
        if let Some(uyari) = controller.apply_clock_ceiling(tel.gpu.as_ref(), ceiling) {
            log.line(&uyari);
        }

        let (ema_cpu, ema_gpu) = classifier.smoothed();
        write_status(&StatusSnapshot {
            mode: mode.label(),
            power: power.label(),
            battery_pct: power.battery_pct,
            game: game.map(|g| g.process.as_str()),
            sample: &s,
            ema_cpu,
            ema_gpu,
            ceiling_mhz: ceiling,
            env: &env,
            suspended: &services_down,
            uptime_s: start.elapsed().as_secs(),
            mem: &reclaimer,
        });

        let report_every = if daemon { 60 } else { 5 };
        if tick.is_multiple_of(report_every) || last_game.as_deref() != game.map(|g| g.process.as_str()) {
            log.line(&format!(
                "{:>5.0} {:>11} {:>6} {:>6} {:>5.0}% {:>5.0}% {:>6} {:>7}",
                start.elapsed().as_secs_f32(),
                mode.label(),
                s.cpu_temp_c,
                s.gpu.temp_c,
                ema_cpu * 100.0,
                ema_gpu * 100.0,
                ceiling,
                s.cpu_fan_rpm
            ));
        }
        last_game = game.map(|g| g.process.clone());

        tick += 1;
        if let Some(n) = secs {
            if start.elapsed().as_secs() >= n {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(cfg.tick_ms.clamp(200, 10_000)));
    }

    log.line("Durduruluyor - ayarlar geri aliniyor...");
    controller.restore(&tel.acpi, tel.gpu.as_ref());
    // Only put back what we took down; services that were already stopped when
    // we arrived are not ours to start.
    if !suspended.stopped_by_us.is_empty() {
        services::resume(&suspended.stopped_by_us);
        log.line(&format!(
            "[+] servisler geri baslatildi: {}",
            suspended.stopped_by_us.join(", ")
        ));
    }
    // Remove the snapshot so `status` reports honestly instead of showing a
    // frozen picture of a daemon that is no longer running.
    let _ = std::fs::remove_file(sidecar("status.txt"));
    log.line("--- rogctl durdu, makine baslangic durumunda ---");
    Ok(())
}

/// Verify every write path we intend to use, without changing the machine.
///
/// Each check either writes a value back onto itself or reverts immediately,
/// so a failure here is informative and a success leaves no trace. Knowing
/// whether writes need elevation decides how the daemon has to be installed.
fn selftest() -> anyhow::Result<()> {
    println!("YAZMA YOLU TESTI (hepsi geri alinabilir)");
    println!("{:-<60}", "");

    // ---- ACPI ----------------------------------------------------------
    let acpi = Acpi::open()?;
    acpi.init().ok();
    println!("[+] ATKACPI acildi");

    match acpi.read(devices::PERF_MODE) {
        Ok(raw) => {
            let mode = raw & 0xFFFF;
            match acpi.write(devices::PERF_MODE, mode) {
                Ok(rc) => println!("[+] ACPI yazma: PERF_MODE <- {mode} (ayni deger, rc=0x{rc:08X})"),
                Err(e) => println!("[-] ACPI yazma reddedildi: {e}"),
            }
        }
        Err(e) => println!("[-] PERF_MODE okunamadi: {e}"),
    }

    match acpi.read_blob(devices::CPU_FAN_CURVE) {
        Ok(blob) if blob.len() >= 16 => {
            let curve = &blob[..16];
            match acpi.write_blob(devices::CPU_FAN_CURVE, curve) {
                Ok(rc) => println!("[+] ACPI egri yazma: CPU egrisi aynen geri yazildi (rc=0x{rc:08X})"),
                Err(e) => println!("[-] ACPI egri yazma reddedildi: {e}"),
            }
        }
        _ => println!("[-] CPU fan egrisi okunamadi"),
    }

    // ---- NVML ----------------------------------------------------------
    let g = match gpu::Gpu::open() {
        Ok(g) => g,
        Err(e) => {
            println!("[-] NVML acilamadi: {e}");
            return Ok(());
        }
    };
    println!("[+] NVML acildi");

    let baseline = g.enforced_power_limit();
    match baseline {
        Some(w) => println!("    mevcut guc limiti: {w:.0}W"),
        None => println!("    mevcut guc limiti okunamadi"),
    }

    let target = baseline.unwrap_or_else(|| g.power_limit_range().2);
    match g.set_power_limit(target) {
        Ok(v) => println!("[+] NVML yazma: guc limiti <- {v:.0}W (ayni deger)"),
        Err(e) => println!("[-] NVML guc limiti yazilamadi: {e}"),
    }

    match g.lock_graphics_clock(210, 1200) {
        Ok(()) => {
            std::thread::sleep(Duration::from_millis(700));
            let s = g.sample();
            println!("[+] NVML clock lock 210-1200MHz uygulandi (olculen {}MHz)", s.clock_graphics_mhz);
            match g.unlock_graphics_clock() {
                Ok(()) => println!("[+] clock lock kaldirildi - GPU normale dondu"),
                Err(e) => println!("[!] DIKKAT: clock lock kaldirilamadi: {e}"),
            }
        }
        Err(e) => println!("[-] clock lock uygulanamadi: {e}"),
    }

    if let Some(w) = baseline {
        g.set_power_limit(w).ok();
    }

    println!("\nTest bitti - makine baslangic durumunda.");
    Ok(())
}

/// Live telemetry from both sides at once.
///
/// Reading the ACPI sensors next to NVML's GPU figures is what lets us confirm
/// what the unnamed BIOS addresses actually mean.
fn mon(secs: Option<u64>) -> anyhow::Result<()> {
    let acpi = Acpi::open()?;
    acpi.init().ok();
    let g = gpu::Gpu::open().ok();
    if g.is_none() {
        println!("(NVML acilamadi - sadece ACPI gosteriliyor)");
    }

    println!(
        "{:>5} {:>7} {:>7} {:>7} {:>6} | {:>6} {:>5} {:>8} {:>7} {:>6}",
        "t", "CPU C", "TGP W", "CPUfan", "GPUfan", "GPU C", "util", "VRAM MB", "GPU W", "MHz"
    );
    println!("{:-<82}", "");

    let start = Instant::now();
    loop {
        let cpu_t = acpi.read(devices::CPU_TEMP).map(|v| v & 0xFFFF).unwrap_or(0);
        let tgp = acpi.read(devices::GPU_TGP).map(|v| v & 0xFFFF).unwrap_or(0);
        let cfan = acpi.read(devices::CPU_FAN_RPM).map(|v| (v & 0xFFFF) * 100).unwrap_or(0);
        let gfan = acpi.read(devices::GPU_FAN_RPM).map(|v| (v & 0xFFFF) * 100).unwrap_or(0);
        let s = g.as_ref().map(|g| g.sample()).unwrap_or_default();

        println!(
            "{:>5.0} {:>7} {:>7} {:>7} {:>6} | {:>6} {:>4}% {:>8} {:>6.1} {:>6}",
            start.elapsed().as_secs_f32(),
            cpu_t,
            tgp,
            cfan,
            gfan,
            s.temp_c,
            s.util_gpu,
            s.vram_used_mb,
            s.power_w,
            s.clock_graphics_mhz
        );

        if let Some(n) = secs {
            if start.elapsed().as_secs() >= n {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}

/// Show what NVML reports and what range it will let us drive.
fn gpu_status() -> anyhow::Result<()> {
    let g = gpu::Gpu::open()?;
    let (min_w, max_w, def_w) = g.power_limit_range();

    println!("GPU (NVML)");
    println!("{:-<50}", "");
    println!("  power limit araligi : {min_w:.0}W - {max_w:.0}W  (varsayilan {def_w:.0}W)");

    let s = g.sample();
    println!("  VRAM                : {} / {} MB  ({:.0}%)", s.vram_used_mb, s.vram_total_mb, s.vram_frac * 100.0);
    println!("  kullanim            : GPU %{}  bellek %{}", s.util_gpu, s.util_mem);
    println!("  sicaklik            : {}C", s.temp_c);
    println!("  guc                 : {:.1}W  (limit {:.0}W)", s.power_w, s.power_limit_w);
    println!("  saat                : cekirdek {}MHz  bellek {}MHz", s.clock_graphics_mhz, s.clock_mem_mhz);

    Ok(())
}

/// Poll every supported address and report what moves.
///
/// This is how the unnamed addresses get identified: change one slider in
/// Armoury Crate and whichever address twitches is the one behind it.
fn watch() -> anyhow::Result<()> {
    let acpi = Acpi::open()?;
    acpi.init().ok();

    println!("Destekli adresler taraniyor...");
    let addrs = supported_addresses(&acpi);
    println!("{} adres izleniyor. Cikmak icin Ctrl+C.\n", addrs.len());
    println!("Simdi Armoury Crate'i ac ve TEK BIR ayari degistir.");
    println!("Hangi adresin oynadigini burada gorecegiz.\n");
    println!("{:-<70}", "");

    let mut last: BTreeMap<u32, u32> = BTreeMap::new();
    for &id in &addrs {
        if let Ok(v) = acpi.read(id) {
            last.insert(id, v & 0xFFFF);
        }
    }

    let start = Instant::now();
    let mut ticks: u64 = 0;

    loop {
        std::thread::sleep(Duration::from_millis(400));
        ticks += 1;

        for &id in &addrs {
            let Ok(raw) = acpi.read(id) else { continue };
            let val = raw & 0xFFFF;
            let prev = last.get(&id).copied().unwrap_or(val);

            if val != prev {
                if LIVE_SENSORS.contains(&id) {
                    last.insert(id, val);
                    continue;
                }
                println!(
                    "[{:>6.1}s] 0x{id:08X}  {prev:>6} -> {val:<6}  {}",
                    start.elapsed().as_secs_f32(),
                    label_for(id)
                );
                last.insert(id, val);
            }
        }

        // Heartbeat every ~4s so it is obvious the tool is alive.
        if ticks.is_multiple_of(10) {
            let cpu = acpi.read(0x0011_0013).map(|v| (v & 0xFFFF) * 100).unwrap_or(0);
            let gpu = acpi.read(0x0011_0014).map(|v| (v & 0xFFFF) * 100).unwrap_or(0);
            println!(
                "[{:>6.1}s] .. izleniyor  fan: CPU {cpu} RPM / GPU {gpu} RPM",
                start.elapsed().as_secs_f32()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rollover has to be true at the boundary in both directions: a log
    /// that is merely large must survive, and one that is over the line must
    /// be moved rather than truncated, so the previous generation is still
    /// readable after it happens.
    #[test]
    fn oversized_log_is_moved_aside_not_lost() {
        let dir = std::env::temp_dir().join("rogctl-log-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("rogctl.log");
        let previous = log.with_extension("log.1");

        // Under the limit: left exactly where it is.
        std::fs::write(&log, "kucuk").unwrap();
        Log::rotate(&log);
        assert!(log.exists());
        assert!(!previous.exists());

        // Over it: moved, with the content intact under the new name.
        std::fs::write(&log, vec![b'x'; (LOG_MAX_BYTES + 1) as usize]).unwrap();
        Log::rotate(&log);
        assert!(!log.exists(), "buyuk log yerinde birakilmis");
        assert_eq!(
            std::fs::metadata(&previous).unwrap().len(),
            LOG_MAX_BYTES + 1,
            "devredilen log kirpilmis"
        );

        // A second rollover replaces the kept generation rather than piling up.
        std::fs::write(&log, vec![b'y'; (LOG_MAX_BYTES + 1) as usize]).unwrap();
        Log::rotate(&log);
        let kept = std::fs::read(&previous).unwrap();
        assert!(kept.iter().all(|&b| b == b'y'), "eski nesil degistirilmemis");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn force_rotate_moves_any_size_log() {
        let dir = std::env::temp_dir().join("rogctl-log-force-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("rogctl.log");
        let previous = log.with_extension("log.1");

        std::fs::write(&log, "manuel devir testi").unwrap();
        Log::force_rotate(&log);
        assert!(!log.exists(), "orijinal log kalmis");
        assert!(previous.exists(), "devir dosyasi olusmamis");
        assert_eq!(std::fs::read_to_string(&previous).unwrap(), "manuel devir testi");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mid_run_rotation_rolls_file_when_limit_exceeded() {
        let dir = std::env::temp_dir().join("rogctl-log-midrun-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("rogctl.log");
        let previous = log.with_extension("log.1");

        let mut logger = Log {
            file: std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log)
                .ok(),
            path: Some(log.clone()),
            bytes_written: LOG_MAX_BYTES - 10,
        };

        // Writing a line that pushes it over the limit should trigger automatic rollover
        logger.line("bu satir limiti asirir ve rotasyonu tetikler");
        assert!(previous.exists(), "calisma zamani rotasyonu olusmadi");
        assert!(log.exists(), "yeni log dosyasi acilmadi");

        // Further writes should continue into the new active log
        logger.line("yeni log dosyasina yazildi");
        let new_content = std::fs::read_to_string(&log).unwrap();
        assert!(new_content.contains("yeni log dosyasina yazildi"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

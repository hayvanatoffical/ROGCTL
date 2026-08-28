// ROGCtl GUI - Tauri Backend Commands
// Bu dosya rogctl-gui/src-tauri/src/main.rs içine entegre edilecek

use std::process::Command;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SystemStatus {
    running: bool,
    cpu_temp: String,
    cpu_util: String,
    gpu_temp: String,
    gpu_util: String,
    cpu_fan: String,
    gpu_fan: String,
    gpu_power: String,
    power_source: String,
    mode: String,
    mode_details: String,
    uptime_s: u64,
}

// Status dosyasını oku ve parse et
#[tauri::command]
fn get_status() -> Result<SystemStatus, String> {
    let status_path = get_status_file_path();
    
    match fs::read_to_string(&status_path) {
        Ok(content) => {
            let mut status = SystemStatus {
                running: true,
                cpu_temp: "--".to_string(),
                cpu_util: "--".to_string(),
                gpu_temp: "--".to_string(),
                gpu_util: "--".to_string(),
                cpu_fan: "----".to_string(),
                gpu_fan: "----".to_string(),
                gpu_power: "--".to_string(),
                power_source: "--".to_string(),
                mode: "OFİS".to_string(),
                mode_details: "".to_string(),
                uptime_s: 0,
            };
            
            // Parse key=value formatını
            for line in content.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    match key {
                        "cpu_c" => status.cpu_temp = value.to_string(),
                        "cpu_util" => status.cpu_util = value.to_string(),
                        "gpu_c" => status.gpu_temp = value.to_string(),
                        "gpu_util" => status.gpu_util = value.to_string(),
                        "cpu_fan" => status.cpu_fan = value.to_string(),
                        "gpu_fan" => status.gpu_fan = value.to_string(),
                        "gpu_w" => status.gpu_power = value.to_string(),
                        "kaynak" => status.power_source = value.to_string(),
                        "mod" => status.mode = value.to_uppercase(),
                        "calisma_s" => status.uptime_s = value.parse().unwrap_or(0),
                        "fan_araligi" => {
                            let tavan_mhz = content.lines()
                                .find(|l| l.starts_with("tavan_mhz="))
                                .and_then(|l| l.split_once('='))
                                .map(|(_, v)| v)
                                .unwrap_or("--");
                            status.mode_details = format!("Fan: {} | GPU: {} MHz", value, tavan_mhz);
                        }
                        _ => {}
                    }
                }
            }
            
            Ok(status)
        }
        Err(_) => Err("Durum dosyası okunamadı. Daemon çalışmıyor olabilir.".to_string())
    }
}

// Rapor oluştur ve aç
#[tauri::command]
async fn generate_report() -> Result<String, String> {
    let output = Command::new("rogctl")
        .arg("rapor")
        .output()
        .map_err(|e| format!("Komut çalıştırılamadı: {}", e))?;
    
    if output.status.success() {
        Ok("Rapor oluşturuldu ve tarayıcıda açıldı".to_string())
    } else {
        Err("Rapor oluşturulamadı".to_string())
    }
}

// NVIDIA ayarlarını kontrol et
#[tauri::command]
async fn nvidia_check() -> Result<String, String> {
    let output = Command::new("rogctl")
        .arg("nv")
        .arg("kim")
        .output()
        .map_err(|e| format!("Komut çalıştırılamadı: {}", e))?;
    
    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        Ok(result.to_string())
    } else {
        Err("NVIDIA kontrol edilemedi".to_string())
    }
}

// Valorant FPS düzelt
#[tauri::command]
async fn valorant_fix() -> Result<String, String> {
    let output = Command::new("rogctl")
        .arg("valorant")
        .arg("hepsi")
        .arg("duzelt")
        .output()
        .map_err(|e| format!("Komut çalıştırılamadı: {}", e))?;
    
    if output.status.success() {
        Ok("Valorant FPS ayarları düzeltildi".to_string())
    } else {
        Err("Valorant FPS düzeltilemedi. Oyun kapalı mı?".to_string())
    }
}

// RAM temizle
#[tauri::command]
async fn mem_cleanup() -> Result<String, String> {
    let output = Command::new("rogctl")
        .arg("mem")
        .arg("temizle")
        .output()
        .map_err(|e| format!("Komut çalıştırılamadı: {}", e))?;
    
    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        // Kazanılan MB'ı parse et
        if let Some(line) = result.lines().find(|l| l.contains("gercekten bos")) {
            Ok(format!("RAM temizlendi: {}", line))
        } else {
            Ok("RAM temizlendi".to_string())
        }
    } else {
        Err("RAM temizlenemedi".to_string())
    }
}

// Log dosyasını aç
#[tauri::command]
async fn open_logs() -> Result<(), String> {
    let log_path = get_install_path().join("rogctl.log");
    
    #[cfg(target_os = "windows")]
    Command::new("notepad")
        .arg(&log_path)
        .spawn()
        .map_err(|e| format!("Log açılamadı: {}", e))?;
    
    Ok(())
}

// Config dosyasını aç
#[tauri::command]
async fn open_config() -> Result<(), String> {
    let config_path = get_install_path().join("rogctl.yaml");
    
    #[cfg(target_os = "windows")]
    Command::new("notepad")
        .arg(&config_path)
        .spawn()
        .map_err(|e| format!("Config açılamadı: {}", e))?;
    
    Ok(())
}

// Config klasörünü aç
#[tauri::command]
async fn open_config_folder() -> Result<(), String> {
    let config_path = get_install_path();
    
    #[cfg(target_os = "windows")]
    Command::new("explorer")
        .arg(&config_path)
        .spawn()
        .map_err(|e| format!("Klasör açılamadı: {}", e))?;
    
    Ok(())
}

// Daemon'u yeniden başlat
#[tauri::command]
async fn restart_daemon() -> Result<String, String> {
    // Önce durdur
    Command::new("schtasks")
        .args(&["/End", "/TN", "rogctl"])
        .output()
        .map_err(|e| format!("Durdurulamadı: {}", e))?;
    
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Sonra başlat
    let output = Command::new("schtasks")
        .args(&["/Run", "/TN", "rogctl"])
        .output()
        .map_err(|e| format!("Başlatılamadı: {}", e))?;
    
    if output.status.success() {
        Ok("Daemon yeniden başlatıldı".to_string())
    } else {
        Err("Daemon başlatılamadı".to_string())
    }
}

// Modu ayarla
#[tauri::command]
async fn set_mode(mode: String) -> Result<String, String> {
    if mode == "auto" {
        // Otomatik mod - force'u kaldır
        Ok("Otomatik mod aktif".to_string())
    } else {
        // Manuel mod - rogctl force komutu ile
        let output = Command::new("rogctl")
            .arg("force")
            .arg(&mode)
            .output()
            .map_err(|e| format!("Mod ayarlanamadı: {}", e))?;
        
        if output.status.success() {
            Ok(format!("{} modu aktif", mode.to_uppercase()))
        } else {
            Err("Mod ayarlanamadı".to_string())
        }
    }
}

// Yardımcı fonksiyonlar
fn get_install_path() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("ProgramFiles")
            .unwrap_or_else(|_| "C:\\Program Files".to_string())
    ).join("ROGCtl")
}

fn get_status_file_path() -> std::path::PathBuf {
    // Önce kurulum klasörüne bak
    let installed = get_install_path().join("status.txt");
    if installed.exists() {
        return installed;
    }
    
    // Sonra kullanıcı klasörüne
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let user_path = std::path::PathBuf::from(user_profile)
            .join("rogctl")
            .join("bin")
            .join("status.txt");
        if user_path.exists() {
            return user_path;
        }
    }
    
    // Varsayılan
    installed
}

// Tauri main.rs'de kullanılacak:
/*
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_status,
            generate_report,
            nvidia_check,
            valorant_fix,
            mem_cleanup,
            open_logs,
            open_config,
            open_config_folder,
            restart_daemon,
            set_mode
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
*/

# ROGCtl GUI Kurulum Rehberi

Modern, kullanıcı dostu grafik arayüzlü ROGCtl uygulaması oluşturma rehberi.

## 📋 Gereksinimler

### Zorunlu
- ✅ **Rust** (1.70+) - https://rustup.rs/
- ✅ **Node.js** (16+) - https://nodejs.org/
- ✅ **Inno Setup 6** - https://jrsoftware.org/isdl.php

### Opsiyonel
- **VS Code** - Geliştirme için önerilen IDE
- **Tauri CLI** - Otomatik kurulacak

## 🚀 Hızlı Başlangıç

### 1. GUI Proje Yapısını Oluştur

```powershell
# GUI kurulum scriptini çalıştır
.\setup-gui.ps1
```

### 2. Tauri Projesini Başlat

```powershell
cd rogctl-gui
npm create tauri-app@latest .
```

**Kurulum sırasında seçenekler:**
- **App name:** `ROGCtl`
- **Window title:** `ROGCtl - Termal Yönetim`
- **UI template:** `Vanilla` (veya React/Vue tercihinize göre)
- **TypeScript:** `Evet`
- **Package manager:** `npm`

### 3. Dependencies Yükle

```powershell
cd rogctl-gui
npm install
```

### 4. Template Dosyalarını Kopyala

GUI template dosyalarını proje içine kopyalayın:

```powershell
# HTML
Copy-Item ..\gui-templates\index.html .\src\

# CSS
Copy-Item ..\gui-templates\styles.css .\src\

# JavaScript
Copy-Item ..\gui-templates\app.js .\src\

# Rust backend
Copy-Item ..\gui-templates\tauri-commands.rs .\src-tauri\src\
```

### 5. Tauri Yapılandırması

`src-tauri/src/main.rs` dosyasını düzenleyin:

```rust
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod tauri_commands;
use tauri_commands::*;

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
        .expect("ROGCtl GUI başlatılamadı");
}
```

### 6. Tauri Config Düzenle

`src-tauri/tauri.conf.json` dosyasını güncelleyin:

```json
{
  "build": {
    "beforeBuildCommand": "",
    "beforeDevCommand": "",
    "devPath": "../src",
    "distDir": "../src"
  },
  "package": {
    "productName": "ROGCtl",
    "version": "1.0.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "all": false,
        "open": true
      },
      "window": {
        "all": true,
        "close": true,
        "hide": true,
        "show": true,
        "maximize": true,
        "minimize": true,
        "unmaximize": true,
        "unminimize": true,
        "startDragging": true
      }
    },
    "bundle": {
      "active": true,
      "targets": ["msi", "nsis"],
      "identifier": "com.rogctl.app",
      "icon": [
        "icons/icon.ico"
      ],
      "windows": {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      }
    },
    "windows": [
      {
        "title": "ROGCtl - Termal Yönetim",
        "width": 1000,
        "height": 700,
        "resizable": true,
        "fullscreen": false,
        "decorations": false,
        "transparent": false,
        "center": true,
        "minWidth": 800,
        "minHeight": 600
      }
    ],
    "systemTray": {
      "iconPath": "icons/icon.ico",
      "iconAsTemplate": true,
      "menuOnLeftClick": false
    }
  }
}
```

## 🎨 Geliştirme Modu

### Dev Server Başlat

```powershell
cd rogctl-gui
npm run tauri dev
```

Bu komut:
- ✅ GUI uygulamasını geliştirme modunda açar
- ✅ Hot reload aktif olur
- ✅ DevTools kullanılabilir

### Özellikler Test Et

Geliştirme modunda test edebileceğiniz özellikler:
- **Durum Kartları** - CPU/GPU/Fan gösterimi
- **Performans Modu** - Mod değiştirme
- **Sıcaklık Grafiği** - Canlı chart
- **Hızlı İşlemler** - Rapor, NVIDIA, Valorant, RAM
- **Ayarlar Modal** - Yapılandırma paneli

## 🏗️ Production Build

### GUI Build

```powershell
cd rogctl-gui
npm run tauri build
```

Build çıktıları:
- `src-tauri/target/release/rogctl-gui.exe` - Ana GUI uygulaması
- `src-tauri/target/release/bundle/` - Kurulum paketleri (MSI, NSIS)

### Tam Kurulum Paketi Oluştur

Tüm projeyi (CLI + GUI + Installer) derleyin:

```powershell
# Ana dizine geri dön
cd ..

# Build scriptini çalıştır
.\build-installer.ps1
```

Bu script:
1. ✅ CLI uygulamasını derler (`rogctl.exe`)
2. ✅ GUI uygulamasını derler (`rogctl-gui.exe`)
3. ✅ İkon ve görselleri hazırlar
4. ✅ Inno Setup ile kurulum paketi oluşturur
5. ✅ `release/` klasörüne çıktı verir

### Çıktı Dosyaları

```
release/
└── ROGCtl-Setup-v1.0.0.exe    # Kurulum paketi (~5-8 MB)
```

## 📦 Kurulum Paketinin Özellikleri

### Kurulum Sihirbazı İçeriği

1. **Hoş Geldiniz** - ROGCtl tanıtımı
2. **Lisans** - MIT License
3. **Bilgilendirme** - Sistem gereksinimleri
4. **Kurulum Konumu** - Varsayılan: `C:\Program Files\ROGCtl`
5. **Bileşen Seçimi**:
   - ✅ ROGCtl CLI (zorunlu)
   - ✅ ROGCtl GUI (seçimli)
   - ✅ Desktop kısayolu
   - ✅ Başlangıçta otomatik başlat
   - ✅ PATH'e ekle
6. **Kurulum** - Dosya kopyalama + yapılandırma
7. **Tamamlandı** - Başlatma seçenekleri

### Kurulum Sonrası

Kurulum tamamlandığında:
- ✅ CLI kullanılabilir: `rogctl status`
- ✅ GUI başlatılabilir: Desktop'tan veya Start Menu'den
- ✅ Sistem tray'de ikon belirir
- ✅ Daemon otomatik başlar (yönetici yetkisiyle)

## 🎮 GUI Kullanımı

### Ana Özellikler

#### 1. Durum Kartları
- **CPU** - Sıcaklık ve kullanım
- **GPU** - Sıcaklık, kullanım, güç
- **Fanlar** - CPU ve GPU fan hızları
- **Güç** - Pil/Priz durumu

#### 2. Performans Modu
- Otomatik mod (varsayılan)
- Manuel mod seçimi (Boşta/Film/Ofis/Hafif/AAA/Render)
- Mod detayları (fan aralığı, GPU saat)

#### 3. Sıcaklık Grafiği
- Son 30 veri noktası
- CPU ve GPU çizgileri
- Canlı güncelleme (2 saniye)

#### 4. Hızlı İşlemler
- **📊 Rapor** - HTML rapor oluştur
- **🎮 NVIDIA** - Sürücü ayarlarını kontrol et
- **🎯 Valorant** - FPS ayarlarını düzelt
- **💾 RAM** - Standby listesini temizle

#### 5. Ayarlar
- Otomatik başlatma
- Sistem tepsisine küçült
- Bildirimler
- Yapılandırma klasörü
- Daemon yeniden başlatma

### Klavye Kısayolları

- `Ctrl+R` - Durumu yenile
- `Ctrl+,` - Ayarları aç
- `Ctrl+L` - Logları görüntüle
- `Alt+F4` - Kapat

## 🐛 Sorun Giderme

### GUI Açılmıyor

**Sorun:** Uygulama başlamıyor
**Çözüm:**
```powershell
# Logları kontrol et
Get-Content "$env:LOCALAPPDATA\ROGCtl\logs\gui.log"

# Daemon durumunu kontrol et
rogctl status
```

### Durum Güncellenmiyor

**Sorun:** Kartlar "--" gösteriyor
**Çözüm:**
```powershell
# Daemon çalışıyor mu?
Get-Process rogctl

# Status dosyası var mı?
Get-Content "$env:ProgramFiles\ROGCtl\status.txt"

# Daemon'u yeniden başlat
schtasks /Run /TN rogctl
```

### NVIDIA Özellikleri Çalışmıyor

**Sorun:** GPU saat kontrolü yok
**Çözüm:**
```powershell
# NVIDIA sürücü kontrolü
nvidia-smi

# NVML testi
rogctl gpu
```

## 🔧 Özelleştirme

### Tema Değiştirme

`gui-templates/styles.css` dosyasında CSS değişkenleri:

```css
:root {
    --accent-primary: #ff0050;    /* Ana vurgu rengi */
    --accent-secondary: #ff3366;  /* İkincil vurgu */
    --bg-primary: #0f0f0f;        /* Ana arka plan */
    /* ... */
}
```

### Dil Desteği

Şu anda desteklenen diller:
- 🇹🇷 Türkçe
- 🇬🇧 English

Yeni dil eklemek için `app.js` içinde çeviri objesi oluşturun.

### Ek Özellikler

Yeni özellik eklemek için:
1. `tauri-commands.rs` - Backend komut ekle
2. `app.js` - Frontend fonksiyon ekle
3. `index.html` - UI elementi ekle
4. `main.rs` - Komutu register et

## 📚 Kaynaklar

- **Tauri Docs:** https://tauri.app/
- **Rust Book:** https://doc.rust-lang.org/book/
- **Inno Setup:** https://jrsoftware.org/ishelp/

## 🤝 Katkıda Bulunma

Geliştirme yapacaksanız:

```powershell
# Development build
npm run tauri dev

# Production build
npm run tauri build

# Kurulum paketi oluştur
.\build-installer.ps1
```

## ⚡ Sonraki Adımlar

- [ ] Sistem tray menüsü ekle
- [ ] Toast bildirimleri
- [ ] Mod geçmişi grafiği
- [ ] Fan eğrisi düzenleyici
- [ ] Otomatik güncelleme
- [ ] Multi-monitor desteği

---

**Not:** Bu rehber Tauri 1.x için hazırlanmıştır. Tauri 2.x kullanıyorsanız bazı API'ler değişmiş olabilir.

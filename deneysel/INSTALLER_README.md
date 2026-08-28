# 🎮 ROGCtl - Profesyonel Kurulum Paketi

Modern, kullanıcı dostu kurulum sihirbazı ve GUI uygulaması ile ROGCtl!

## ✨ Özellikler

### 🎯 Kurulum Sihirbazı
- ✅ **İki Dil Desteği:** Türkçe ve İngilizce
- ✅ **Modern UI:** Armoury Crate ilhamlı tasarım
- ✅ **Otomatik Yapılandırma:** Tek tıkla kurulum
- ✅ **Akıllı Kontroller:** Sistem gereksinim kontrolü
- ✅ **Temiz Kaldırma:** Tüm izleri temizler

### 🖥️ GUI Uygulaması
- ✅ **Canlı İzleme:** CPU/GPU/Fan durumu
- ✅ **Performans Modu:** Otomatik veya manuel seçim
- ✅ **Sıcaklık Grafiği:** Gerçek zamanlı görselleştirme
- ✅ **Hızlı İşlemler:** Tek tıkla komutlar
- ✅ **Sistem Tray:** Arka planda çalışma

### ⚙️ CLI Uygulaması
- ✅ **Güçlü Komutlar:** Terminal kontrolü
- ✅ **Detaylı Raporlama:** HTML rapor oluşturma
- ✅ **NVIDIA Entegrasyonu:** Sürücü ayar yönetimi
- ✅ **Valorant Desteği:** FPS ayar düzeltme
- ✅ **RAM Yönetimi:** Akıllı bellek temizliği

## 📦 Kurulum Paketi Oluşturma

### Hızlı Başlangıç (Sadece CLI)

```powershell
# 1. Derle
cargo build --release

# 2. Kurulum paketi oluştur
.\build-installer.ps1 -SkipGUI

# 3. Çıktı
# release/ROGCtl-Setup-v1.0.0.exe (~3 MB)
```

### Tam Paket (CLI + GUI)

```powershell
# 1. GUI projesi oluştur
.\setup-gui.ps1
cd rogctl-gui
npm create tauri-app@latest .
npm install
cd ..

# 2. Template dosyalarını kopyala
.\copy-templates.ps1

# 3. Tümünü derle ve paketle
.\build-installer.ps1

# 4. Çıktı
# release/ROGCtl-Setup-v1.0.0.exe (~8 MB)
```

## 📁 Proje Yapısı

```
rogctl/
├── src/                          # Rust CLI kaynak kodu
├── installer/                    # Kurulum dosyaları
│   ├── setup.iss                # Inno Setup script
│   ├── scripts/                 # Kurulum scriptleri
│   │   ├── check-requirements.ps1
│   │   ├── post-install.ps1
│   │   └── uninstall-cleanup.ps1
│   └── assets/                  # Görsel dosyalar
│       ├── rogctl.ico           # Ana ikon (ZORUNLU)
│       ├── wizard-image.bmp     # Sol panel (önerilen)
│       └── wizard-small.bmp     # Başlık ikonu (önerilen)
├── gui-templates/               # GUI template dosyaları
│   ├── index.html              # Ana sayfa
│   ├── styles.css              # Modern tema
│   ├── app.js                  # Frontend mantığı
│   └── tauri-commands.rs       # Backend komutları
├── rogctl-gui/                  # Tauri GUI projesi (oluşturulacak)
│   ├── src/                    # Frontend
│   └── src-tauri/              # Rust backend
├── build-installer.ps1          # Ana build scripti
├── setup-gui.ps1               # GUI kurulum scripti
├── copy-templates.ps1          # Template kopyalama
├── LICENSE.txt                 # MIT lisansı
├── INSTALL_INFO.txt            # Kurulum bilgilendirme
├── GUI_SETUP_GUIDE.md          # Detaylı GUI rehberi
├── QUICK_START.md              # Hızlı başlangıç
└── INSTALLER_README.md         # Bu dosya
```

## 🛠️ Gereksinimler

### Build İçin
- ✅ **Rust** 1.70+ (https://rustup.rs/)
- ✅ **Node.js** 16+ (GUI için) (https://nodejs.org/)
- ✅ **Inno Setup 6** (https://jrsoftware.org/isdl.php)

### Çalışma İçin (Son Kullanıcı)
- ✅ Windows 10/11 (64-bit)
- ✅ ASUS ROG Laptop (optimize edilmiş)
- ⚠️ NVIDIA GPU (önerilir, opsiyonel)
- ⚠️ Yönetici yetkisi (gerekli)

## 🎨 Özelleştirme

### Versiyon Numarası

```powershell
# Build sırasında versiyon belirtin
.\build-installer.ps1 -Version "1.2.0"
```

### Kurulum Konumu

`installer/setup.iss` dosyasında:
```ini
DefaultDirName={autopf}\{#MyAppName}
; Varsayılan: C:\Program Files\ROGCtl
```

### Tema Renkleri

`gui-templates/styles.css` dosyasında:
```css
:root {
    --accent-primary: #ff0050;    /* Ana vurgu rengi */
    --bg-primary: #0f0f0f;        /* Arka plan */
    /* ... */
}
```

### İkonlar

1. `installer/assets/rogctl.ico` dosyasını değiştirin
2. 256x256 PNG'den ICO'ya dönüştürün: https://icoconverter.com/

## 📋 Kurulum Akışı

### Kullanıcı Deneyimi

```
┌─────────────────────────────────┐
│  1. Hoş Geldiniz               │
│     ↓                           │
│  2. Lisans (MIT)               │
│     ↓                           │
│  3. Bilgilendirme              │
│     ↓                           │
│  4. Sistem Kontrolleri         │
│     • Windows 10/11 ✓          │
│     • ASUS Laptop ✓            │
│     • NVIDIA GPU ⚠             │
│     • Yönetici Yetkisi ✓      │
│     ↓                           │
│  5. Kurulum Konumu             │
│     ↓                           │
│  6. Bileşen Seçimi             │
│     [✓] ROGCtl CLI             │
│     [✓] ROGCtl GUI             │
│     [✓] Desktop Kısayolu       │
│     [✓] Otomatik Başlat        │
│     [✓] PATH'e Ekle            │
│     ↓                           │
│  7. Kurulum                    │
│     • Dosyalar kopyalanıyor    │
│     • Yapılandırılıyor         │
│     • Zamanlanmış görev        │
│     • NVIDIA ayarları          │
│     ↓                           │
│  8. Tamamlandı                 │
│     [✓] ROGCtl'yi Başlat      │
│     [✓] Yapılandırmayı Aç     │
└─────────────────────────────────┘
```

### Otomatik Yapılandırmalar

Kurulum sırasında otomatik olarak:
1. ✅ CLI ve GUI dosyaları kopyalanır
2. ✅ `rogctl.yaml` yapılandırma oluşturulur
3. ✅ PATH ortam değişkenine eklenir
4. ✅ Zamanlanmış görev oluşturulur (yönetici)
5. ✅ NVIDIA sürücü ayarları uygulanır
6. ✅ ASUS servisleri durdurulur
7. ✅ RAM temizlik yetkileri kontrol edilir
8. ✅ Daemon başlatılır

## 🚀 Build Komutları

### Temel Komutlar

```powershell
# Tam build (CLI + GUI + Installer)
.\build-installer.ps1

# Sadece CLI
.\build-installer.ps1 -SkipGUI

# Build'i atla, sadece paketle
.\build-installer.ps1 -SkipBuild

# Özel versiyon
.\build-installer.ps1 -Version "2.0.0"

# Kombine
.\build-installer.ps1 -SkipGUI -Version "1.5.0"
```

### GUI Geliştirme

```powershell
# Dev server (hot reload)
cd rogctl-gui
npm run tauri dev

# Production build
npm run tauri build

# CLI ile test
cd ..
cargo run -- status
```

## 📊 Çıktı Dosyaları

### Build Sonrası

```
target/release/
├── rogctl.exe                    # CLI (~2 MB)
└── ...

rogctl-gui/src-tauri/target/release/
├── rogctl-gui.exe                # GUI (~3-4 MB)
└── bundle/
    ├── msi/                      # Windows Installer
    └── nsis/                     # NSIS Installer

release/
└── ROGCtl-Setup-v1.0.0.exe      # Final kurulum paketi (~5-8 MB)
```

### Kurulum Sonrası (Kullanıcı)

```
C:\Program Files\ROGCtl\
├── rogctl.exe                    # CLI uygulaması
├── rogctl-gui.exe                # GUI uygulaması (opsiyonel)
├── rogctl.yaml                   # Yapılandırma
├── rogctl.log                    # Log dosyası
├── status.txt                    # Durum dosyası
└── README.md                     # Dokümantasyon
```

## 🐛 Sorun Giderme

### Build Hataları

**Sorun:** `cargo: command not found`
```powershell
# Rust kurulumu
winget install Rustlang.Rustup
# Veya https://rustup.rs/
```

**Sorun:** `ISCC.exe bulunamadı`
```powershell
# Inno Setup kurulumu
winget install JRSoftware.InnoSetup
# Veya https://jrsoftware.org/isdl.php
```

**Sorun:** `node: command not found`
```powershell
# Node.js kurulumu
winget install OpenJS.NodeJS
# Veya https://nodejs.org/
```

### Kurulum Hataları

**Sorun:** "Yönetici yetkisi gerekiyor"
- Kurulum EXE'sine sağ tık → "Yönetici olarak çalıştır"

**Sorun:** "NVIDIA GPU bulunamadı"
- Normal, GPU özellikleri devre dışı kalacak
- Fan kontrolü çalışmaya devam eder

**Sorun:** GUI açılmıyor
```powershell
# Logları kontrol et
Get-Content "$env:LOCALAPPDATA\ROGCtl\logs\gui.log"

# Daemon durumunu kontrol et
rogctl status
```

## 📚 Dokümantasyon

- **[QUICK_START.md](QUICK_START.md)** - 5 dakikada başla
- **[GUI_SETUP_GUIDE.md](GUI_SETUP_GUIDE.md)** - Detaylı GUI rehberi
- **[README.md](README.md)** - Ana proje dokümantasyonu
- **[installer/assets/README.md](installer/assets/README.md)** - Görsel dosyaları

## 🤝 Katkıda Bulunma

1. Fork edin
2. Feature branch oluşturun (`git checkout -b feature/amazing-feature`)
3. Commit edin (`git commit -m 'feat: add amazing feature'`)
4. Push edin (`git push origin feature/amazing-feature`)
5. Pull Request açın

## 📝 Lisans

MIT License - Detaylar için [LICENSE.txt](LICENSE.txt) dosyasına bakın.

## 🙏 Teşekkürler

- **Tauri** - Modern GUI framework
- **Inno Setup** - Güçlü kurulum sihirbazı
- **Rust Community** - Harika ekosistem
- **ASUS ROG** - Donanım ilhamı

## 📞 İletişim

- **GitHub Issues:** [Report Bug / Request Feature](https://github.com/yourusername/rogctl/issues)
- **Discussions:** [Community Forum](https://github.com/yourusername/rogctl/discussions)

---

<div align="center">

**ROGCtl** - ASUS ROG için akıllı termal yönetim

Made with ❤️ for ROG Community

[⬆ Başa Dön](#-rogctl---profesyonel-kurulum-paketi)

</div>

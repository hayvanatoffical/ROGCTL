# ROGCtl - Hızlı Başlangıç Rehberi

Profesyonel kurulum paketi oluşturmak için 5 dakika!

## 🎯 Seçenekleriniz

### Seçenek 1: Sadece CLI (Komut Satırı)
**Süre:** ~2 dakika  
**İçerik:** Terminal tabanlı rogctl

```powershell
# Derle
cargo build --release

# Kur
.\install.ps1

# Kullan
rogctl status
```

### Seçenek 2: CLI + GUI + Professional Installer
**Süre:** ~10 dakika  
**İçerik:** Modern grafik arayüzlü uygulama + Kurulum sihirbazı

## 📦 Profesyonel Kurulum Paketi Oluşturma

### Adım 1: Gereksinimleri Kur

```powershell
# Rust (zaten kurulu)
# Node.js yükle
winget install OpenJS.NodeJS

# Inno Setup yükle
winget install JRSoftware.InnoSetup
```

### Adım 2: CLI'yı Derle

```powershell
cargo build --release
```

**Çıktı:** `target/release/rogctl.exe` (~2MB)

### Adım 3: GUI Projesi Oluştur (İsteğe Bağlı)

```powershell
# GUI kurulum scriptini çalıştır
.\setup-gui.ps1

# GUI dizinine git
cd rogctl-gui

# Tauri projesi başlat
npm create tauri-app@latest .

# Ayarlar:
#  - App name: ROGCtl
#  - UI template: Vanilla
#  - TypeScript: Yes

# Dependencies yükle
npm install

# Ana dizine dön
cd ..
```

### Adım 4: Template Dosyalarını Kopyala

```powershell
# Otomatik kopyalama scripti
.\copy-templates.ps1

# VEYA Manuel:
Copy-Item gui-templates\*.html rogctl-gui\src\
Copy-Item gui-templates\*.css rogctl-gui\src\
Copy-Item gui-templates\*.js rogctl-gui\src\
Copy-Item gui-templates\tauri-commands.rs rogctl-gui\src-tauri\src\
```

### Adım 5: Kurulum Paketi Oluştur

```powershell
# Tek komutla tümünü derle ve paketle
.\build-installer.ps1
```

**Bu script:**
1. ✅ CLI derler
2. ✅ GUI derler (varsa)
3. ✅ İkonları hazırlar
4. ✅ Inno Setup çalıştırır
5. ✅ EXE kurulum dosyası oluşturur

**Çıktı:** `release/ROGCtl-Setup-v1.0.0.exe` (~5-8MB)

## 🚀 Kurulum Paketini Test Et

```powershell
# Kurulum paketini çalıştır
.\release\ROGCtl-Setup-v1.0.0.exe
```

Kurulum sihirbazı:
1. ✅ Hoş geldiniz ekranı
2. ✅ Lisans (MIT)
3. ✅ Sistem gereksinimleri kontrolü
4. ✅ Kurulum konumu seçimi
5. ✅ Bileşen seçimi (CLI/GUI)
6. ✅ Otomatik yapılandırma
7. ✅ Başlatma seçenekleri

## 🎮 Kurulum Sonrası

### CLI Kullanımı

```powershell
# Durum görüntüle
rogctl status

# HTML rapor oluştur
rogctl rapor

# Canlı izleme
rogctl mon 60

# NVIDIA ayarları
rogctl nv kim

# Valorant FPS
rogctl valorant hepsi duzelt
```

### GUI Kullanımı

1. **Desktop'tan aç** - ROGCtl ikonu
2. **Start Menu'den** - "ROGCtl"
3. **Sistem Tray'den** - Sağ tık → Aç

**Ana Özellikler:**
- 📊 Canlı durum kartları (CPU/GPU/Fan)
- 🎯 Performans modu seçici
- 📈 Sıcaklık grafiği
- ⚡ Hızlı işlemler (Rapor/NVIDIA/Valorant/RAM)
- ⚙️ Ayarlar paneli

## 🛠️ Sadece Gerekli Dosyalar

Minimum kurulum için bu dosyalar yeterli:

### CLI Only
```
rogctl.exe              # Ana uygulama
rogctl.yaml             # Yapılandırma (otomatik oluşur)
install.ps1             # Kurulum scripti
```

### CLI + GUI + Installer
```
target/release/rogctl.exe
rogctl-gui/src-tauri/target/release/rogctl-gui.exe
installer/setup.iss
installer/scripts/*.ps1
installer/assets/*.ico
```

## 🎨 Özelleştirme

### Kurulum Sihirbazı

**Logo değiştir:**
```powershell
# Kendi logonuzu ekleyin (256x256 PNG)
# ImageMagick veya online tool ile .ico'ya dönüştürün
Copy-Item your-logo.ico installer\assets\rogctl.ico
```

**Renk teması:**
`installer/setup.iss` içinde:
```ini
[Setup]
WizardStyle=modern
; Buraya özel renkler ekleyebilirsiniz
```

### GUI Teması

`gui-templates/styles.css` içinde:
```css
:root {
    --accent-primary: #ff0050;    /* Kırmızı yerine başka renk */
    --bg-primary: #0f0f0f;        /* Arka plan rengi */
}
```

## 📊 Build Boyutları

| Bileşen | Boyut |
|---------|-------|
| CLI exe | ~2 MB |
| GUI exe | ~3-4 MB |
| Kurulum paketi | ~5-8 MB |

## 🐛 Hızlı Sorun Giderme

### Build Hataları

```powershell
# Rust bağımlılıklarını temizle
cargo clean

# Node modüllerini temizle (GUI)
cd rogctl-gui
Remove-Item node_modules -Recurse -Force
npm install
```

### Kurulum Hataları

```powershell
# Inno Setup yolunu kontrol et
Test-Path "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"

# ISCC.exe yoksa:
# https://jrsoftware.org/isdl.php adresinden indir
```

### GUI Çalışmıyor

```powershell
# Tauri dependencies
cd rogctl-gui
npm install --save @tauri-apps/api
npm install --save-dev @tauri-apps/cli
```

## 📦 Dağıtım

### GitHub Release

```powershell
# Tag oluştur
git tag -a v1.0.0 -m "İlk kararlı sürüm"
git push origin v1.0.0

# Release oluştur ve EXE'yi yükle
# GitHub'da: Releases → New Release → Upload ROGCtl-Setup-v1.0.0.exe
```

### Manuel Dağıtım

Sadece kurulum EXE'sini paylaşın:
```
release/ROGCtl-Setup-v1.0.0.exe
```

Kullanıcılar çift tıklayarak kurabilir, başka bir şey gerekmez!

## ⚡ Daha Fazla Bilgi

- **Detaylı GUI Rehberi:** `GUI_SETUP_GUIDE.md`
- **Ana README:** `README.md`
- **Kurulum Dokümantasyonu:** `INSTALL_INFO.txt`

## 🎯 Sonraki Adımlar

1. ✅ CLI derle ve test et
2. ✅ GUI projesi oluştur (isteğe bağlı)
3. ✅ Template dosyalarını kopyala
4. ✅ Kurulum paketi oluştur
5. ✅ Test et ve dağıt!

---

**Tebrikler!** Artık profesyonel kurulum paketiniz hazır! 🎉

Sorularınız için: [GitHub Issues](https://github.com/yourusername/rogctl/issues)

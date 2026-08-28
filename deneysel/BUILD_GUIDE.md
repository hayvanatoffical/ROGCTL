# 🏗️ ROGCtl - Profesyonel Build Rehberi

**HATASIZ, GÜVENLİ, PROFESYONELKurulum paketi oluşturma rehberi.**

## ⚠️ Başlamadan Önce

### Zorunlu Gereksinimler

```powershell
# Test scriptini çalıştır - tüm gereksinimleri kontrol eder
.\test-build.ps1
```

Eğer hata varsa, önce bunları düzeltin:

| Gereksinim | Kurulum |
|------------|---------|
| **Rust 1.70+** | `winget install Rustlang.Rustup` veya https://rustup.rs/ |
| **Inno Setup 6** | `winget install JRSoftware.InnoSetup` veya https://jrsoftware.org/isdl.php |
| **Node.js 16+** | `winget install OpenJS.NodeJS` (sadece GUI için) |

---

## 📋 Build Kontrol Listesi

### Adım 1: CLI Derle

```powershell
# Ana dizinde
cargo build --release

# Kontrol et
Test-Path "target\release\rogctl.exe"  # TRUE olmalı
```

**Beklenen boyut:** ~2 MB

### Adım 2: Dokümantasyonu Kontrol Et

```powershell
# Gerekli dosyalar mevcut mu?
Test-Path "LICENSE.txt"         # TRUE olmalı
Test-Path "INSTALL_INFO.txt"    # TRUE olmalı
Test-Path "README.md"           # TRUE olmalı
```

Eksik dosya varsa, zaten oluşturulmuş olmalılar. Yoksa:
- `LICENSE.txt` - MIT lisansı
- `INSTALL_INFO.txt` - Kullanıcı bilgilendirme
- `README.md` - Ana dokümantasyon

### Adım 3: Kurulum Scriptlerini Kontrol Et

```powershell
# Tüm scriptler mevcut mu?
Test-Path "installer\scripts\check-requirements.ps1"     # TRUE
Test-Path "installer\scripts\post-install.ps1"           # TRUE
Test-Path "installer\scripts\uninstall-cleanup.ps1"      # TRUE
Test-Path "installer\setup.iss"                          # TRUE
```

### Adım 4: İkon Ekle (Opsiyonel ama Önerilen)

```powershell
# İkon yolu
$iconPath = "installer\assets\rogctl.ico"

# Mevcut mu kontrol et
Test-Path $iconPath
```

**İkon yoksa:**
1. 256x256 PNG logo bul
2. Online converter: https://icoconverter.com/
3. `installer\assets\rogctl.ico` olarak kaydet

**İkon olmadan da çalışır** - varsayılan Windows ikonu kullanılır.

### Adım 5: GUI Derle (Opsiyonel)

```powershell
# GUI projesi oluşturulmamışsa atla
cd rogctl-gui
npm install
npm run tauri build
cd ..

# Kontrol et
Test-Path "rogctl-gui\src-tauri\target\release\rogctl-gui.exe"
```

**Not:** GUI olmadan sadece CLI kurulumu yapılabilir.

---

## 🚀 Kurulum Paketi Oluşturma

### Seçenek 1: Otomatik Build (Önerilen)

```powershell
# Tüm kontrolleri yap
.\test-build.ps1

# Başarılıysa, build yap
.\build-installer.ps1

# GUI olmadan (daha hızlı)
.\build-installer.ps1 -SkipGUI

# Özel versiyon
.\build-installer.ps1 -Version "1.2.0"
```

**Çıktı:**
```
release/ROGCtl-Setup-v1.0.0.exe
```

### Seçenek 2: Manuel Build

```powershell
# 1. CLI derle
cargo build --release

# 2. Inno Setup çalıştır
& "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" "installer\setup.iss"

# 3. Çıktıyı kontrol et
Get-Item "release\ROGCtl-Setup-v*.exe"
```

---

## ✅ Build Sonrası Doğrulama

### Test 1: Kurulum Paketini Kontrol Et

```powershell
# Dosya mevcut mu?
$setup = Get-Item "release\ROGCtl-Setup-v*.exe"
Write-Host "Dosya: $($setup.Name)"
Write-Host "Boyut: $([math]::Round($setup.Length / 1MB, 2)) MB"

# Beklenen: 5-8 MB (CLI+GUI), 3-5 MB (sadece CLI)
```

### Test 2: Sanal Makinede Kur (Önerilir)

1. **Hyper-V / VirtualBox** ile Windows 10 VM oluştur
2. Setup EXE'yi VM'e kopyala
3. Yönetici olarak çalıştır
4. Tüm adımları takip et
5. Kurulum sonrası test et:
   ```powershell
   rogctl status
   rogctl mon 30
   ```

### Test 3: Kaldırmayı Test Et

```powershell
# VM'de kaldır
# Kontrol Paneli → Programlar → ROGCtl → Kaldır

# VEYA
& "$env:ProgramFiles\ROGCtl\uninst\unins000.exe"
```

**Kontrol edilecekler:**
- ✅ Tüm dosyalar silindi mi?
- ✅ PATH'ten çıkarıldı mı?
- ✅ Zamanlanmış görev silindi mi?
- ✅ Config dosyası korundu mu/silindi mi? (kullanıcı seçimine bağlı)

---

## 🐛 Sorun Giderme

### Hata: "rogctl.exe bulunamadı"

```powershell
# Çözüm:
cargo clean
cargo build --release --verbose
```

### Hata: "ISCC.exe bulunamadı"

```powershell
# Inno Setup yolu doğru mu kontrol et
Test-Path "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"

# Yoksa kur
winget install JRSoftware.InnoSetup
```

### Hata: "Script execution policy"

```powershell
# Geçici olarak execution policy değiştir
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass

# Sonra build script'ini çalıştır
.\build-installer.ps1
```

### Uyarı: "İkon dosyası bulunamadı"

**Sorun değil!** Kurulum varsayılan ikon ile devam eder.

Düzeltmek için:
```powershell
# 1. PNG logo bul (256x256)
# 2. ICO'ya dönüştür: https://icoconverter.com/
# 3. Kaydet
Copy-Item "your-logo.ico" "installer\assets\rogctl.ico"

# 4. Tekrar build
.\build-installer.ps1
```

### Build Çıktısı Çok Büyük

| Durum | Boyut | Çözüm |
|-------|-------|-------|
| Normal (CLI only) | 3-5 MB | ✓ İyi |
| Normal (CLI + GUI) | 5-8 MB | ✓ İyi |
| Çok büyük | >10 MB | Debug symbols var, `cargo build --release` doğru mu? |

Debug symbols kontrolü:
```toml
# Cargo.toml - Bu ayarlar olmalı
[profile.release]
opt-level = 3
lto = true
strip = true
```

---

## 📦 Dağıtım

### GitHub Release

```powershell
# 1. Tag oluştur
git tag -a v1.0.0 -m "İlk kararlı sürüm"
git push origin v1.0.0

# 2. GitHub'da Release oluştur
# Releases → New Release → Choose tag: v1.0.0
# Upload: release/ROGCtl-Setup-v1.0.0.exe

# 3. Release notes ekle (örnek):
```

```markdown
## ROGCtl v1.0.0

### ✨ Özellikler
- Otomatik termal yönetim
- Fan eğrisi kontrolü
- GPU saat optimizasyonu
- NVIDIA sürücü entegrasyonu
- Valorant FPS düzeltme
- RAM yönetimi

### 📦 Kurulum
1. `ROGCtl-Setup-v1.0.0.exe` indirin
2. Yönetici olarak çalıştırın
3. Kurulum sihirbazını takip edin

### 💻 Gereksinimler
- Windows 10/11 (64-bit)
- ASUS ROG Laptop
- NVIDIA GPU (önerilir)

### 📚 Dokümantasyon
- [README.md](README.md)
- [Hızlı Başlangıç](QUICK_START.md)
- [GUI Rehberi](GUI_SETUP_GUIDE.md)
```

### Doğrudan Dağıtım

Sadece EXE dosyasını paylaş:
```
ROGCtl-Setup-v1.0.0.exe
```

Kullanıcılar çift tıklayarak kurabilir!

---

## 🔒 Güvenlik Notları

### Dijital İmza (Opsiyonel)

Profesyonel dağıtım için dijital imza ekleyin:

```powershell
# Code signing certificate gerekir
# SignTool ile imzala
signtool sign /f "certificate.pfx" /p "password" /t "http://timestamp.digicert.com" "release\ROGCtl-Setup-v1.0.0.exe"
```

### Checksum Oluştur

```powershell
# SHA256 hash
$hash = Get-FileHash "release\ROGCtl-Setup-v1.0.0.exe" -Algorithm SHA256
Write-Host "SHA256: $($hash.Hash)"

# Dosyaya kaydet
$hash.Hash | Out-File "release\ROGCtl-Setup-v1.0.0.exe.sha256"
```

Kullanıcılar doğrulayabilir:
```powershell
# Kullanıcı tarafında
$downloaded = Get-FileHash "ROGCtl-Setup-v1.0.0.exe" -Algorithm SHA256
$expected = Get-Content "ROGCtl-Setup-v1.0.0.exe.sha256"
$downloaded.Hash -eq $expected  # TRUE olmalı
```

---

## 📊 Build Performans İpuçları

### Hızlandırma

```powershell
# Paralel derleme
$env:CARGO_BUILD_JOBS = [Environment]::ProcessorCount
cargo build --release

# İncremental build (development)
cargo build --release --incremental

# Temiz build (production)
cargo clean
cargo build --release
```

### Disk Alanı Tasarrufu

```powershell
# Build sonrası temizlik
cargo clean --release
Remove-Item "target\release\build" -Recurse -Force
Remove-Item "target\release\deps" -Recurse -Force
Remove-Item "target\release\incremental" -Recurse -Force

# Sadece EXE'ler kalır
```

---

## 🎯 Son Kontrol Listesi

Dağıtımdan önce:

- [ ] `.\test-build.ps1` başarılı
- [ ] CLI build tamam (`cargo build --release`)
- [ ] GUI build tamam (opsiyonel)
- [ ] İkon eklendi (önerilir)
- [ ] Dokümantasyon tam
- [ ] Kurulum paketi oluşturuldu
- [ ] Sanal makinede test edildi
- [ ] Kurulum testi başarılı
- [ ] Kaldırma testi başarılı
- [ ] Checksum oluşturuldu
- [ ] Release notes hazır

**✅ Hepsi tamam mı? Dağıtıma hazırsınız!**

---

## 📞 Yardım

Sorun mu yaşıyorsunuz?

```powershell
# Detaylı log ile build
.\build-installer.ps1 -Verbose

# Test script'i tekrar çalıştır
.\test-build.ps1

# Manuel kontroller
cargo --version
node --version
Test-Path "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
```

Hala sorun varsa:
1. Hata mesajını tamamen kopyalayın
2. `test-build.ps1` çıktısını ekleyin
3. GitHub Issues'ta paylaşın

---

**🎉 Başarılar! Profesyonel kurulum paketiniz hazır!**

**Dosya boyutları normal mi?**
- CLI only: 3-5 MB ✓
- CLI + GUI: 5-8 MB ✓

**Kurulum test edildi mi?**
- Sanal makine ✓
- Gerçek makine ✓
- Kaldırma ✓

**👉 Dağıtıma hazır!**

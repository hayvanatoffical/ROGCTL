# 🎯 ROGCtl - BURADAN BAŞLAYIN

**Profesyonel kurulum paketi oluşturmak için 3 basit adım!**

---

## ⚡ Hızlı Başlangıç

### 1️⃣ Sisteminizi Test Edin

```powershell
# Tüm gereksinimleri kontrol et
.\test-build.ps1
```

**✓ Başarılı mı?** → Adım 2'ye geçin  
**✗ Hata var mı?** → Hataları düzeltin, sonra tekrar deneyin

---

### 2️⃣ CLI'yı Derleyin

```powershell
# Rust projesini derle
cargo build --release
```

**Sonuç:** `target/release/rogctl.exe` (~2 MB)

---

### 3️⃣ Kurulum Paketini Oluşturun

```powershell
# Otomatik build (GUI olmadan, hızlı)
.\build-installer.ps1 -SkipGUI
```

**Sonuç:** `release/ROGCtl-Setup-v1.0.0.exe` (~3-5 MB)

**🎉 BİTTİ! Kurulum paketiniz hazır!**

---

## 📦 Kurulum Paketini Test Edin

```powershell
# Çift tıklayarak çalıştır
.\release\ROGCtl-Setup-v1.0.0.exe
```

**Kurulum adımları:**
1. Hoş geldiniz ✓
2. Lisans (MIT) ✓
3. Bilgilendirme ✓
4. Sistem kontrolleri ✓
5. Kurulum konumu ✓
6. Bileşenler ✓
7. Kurulum... ✓
8. Tamamlandı! ✓

---

## 🎮 GUI Eklemek İster misiniz? (Opsiyonel)

### Adım 1: GUI Projesi Oluştur

```powershell
# GUI kurulum scripti
.\setup-gui.ps1

# GUI dizinine git
cd rogctl-gui

# Tauri projesi başlat
npm create tauri-app@latest .
# Seçenekler:
#  - App name: ROGCtl
#  - UI template: Vanilla
#  - TypeScript: Yes

# Dependencies
npm install

# Ana dizine dön
cd ..
```

### Adım 2: Template Dosyalarını Kopyala

```powershell
# Otomatik kopyalama
.\copy-templates.ps1
```

### Adım 3: Tam Paketi Derle

```powershell
# CLI + GUI + Installer
.\build-installer.ps1
```

**Sonuç:** `release/ROGCtl-Setup-v1.0.0.exe` (~8 MB)

---

## 📚 Dokümantasyon

Hangi rehberi okuyacağınızdan emin değil misiniz?

| Dosya | Ne İçin? |
|-------|----------|
| **[START_HERE.md](START_HERE.md)** | 👈 Burdasınız - Hızlı başlangıç |
| **[QUICK_START.md](QUICK_START.md)** | 5 dakikada başlama rehberi |
| **[BUILD_GUIDE.md](BUILD_GUIDE.md)** | Detaylı build rehberi & sorun giderme |
| **[GUI_SETUP_GUIDE.md](GUI_SETUP_GUIDE.md)** | GUI oluşturma rehberi |
| **[INSTALLER_README.md](INSTALLER_README.md)** | Kurulum sistemi özeti |

---

## 🔧 Gereksinimler

### Zorunlu (CLI için)
- ✅ Windows 10/11 (64-bit)
- ✅ Rust 1.70+ → https://rustup.rs/
- ✅ Inno Setup 6 → https://jrsoftware.org/isdl.php

### Opsiyonel (GUI için)
- ⚪ Node.js 16+ → https://nodejs.org/

---

## ❓ Sık Sorulan Sorular

### Q: Build hatası alıyorum

```powershell
# 1. Test scriptini çalıştır
.\test-build.ps1

# 2. Detaylı log
.\build-installer.ps1 -Verbose

# 3. Temiz build
cargo clean
cargo build --release
```

### Q: İkon eklemek zorunlu mu?

**Hayır!** İkon olmadan da çalışır, varsayılan Windows ikonu kullanılır.

İkon eklemek için:
1. 256x256 PNG logo bul
2. https://icoconverter.com/ → ICO'ya dönüştür
3. `installer/assets/rogctl.ico` olarak kaydet

### Q: GUI olmadan kurulum yapabilir miyim?

**Evet!** CLI tamamen bağımsız çalışır.

```powershell
# GUI'siz build
.\build-installer.ps1 -SkipGUI
```

### Q: Kurulum paketi çok büyük

| Bileşen | Normal Boyut |
|---------|--------------|
| CLI only | 3-5 MB |
| CLI + GUI | 5-8 MB |

10 MB'dan büyükse debug symbols var demektir:
```powershell
cargo clean
cargo build --release
```

### Q: Dağıtım için ne yapmalıyım?

1. **Test et** (sanal makine önerilir)
2. **Checksum oluştur:**
   ```powershell
   Get-FileHash "release\ROGCtl-Setup-v1.0.0.exe" -Algorithm SHA256
   ```
3. **GitHub Release oluştur** veya direkt paylaş

---

## 🚨 Yaygın Hatalar ve Çözümleri

| Hata | Çözüm |
|------|-------|
| `cargo: command not found` | Rust kur: `winget install Rustlang.Rustup` |
| `ISCC.exe bulunamadı` | Inno Setup kur: `winget install JRSoftware.InnoSetup` |
| `rogctl.exe bulunamadı` | `cargo build --release` çalıştır |
| `Script execution policy` | `Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass` |

---

## 🎯 Sonraki Adımlar

✅ **Build başarılı mı?**

1. Kurulum paketini test edin
2. Sanal makinede deneyin
3. Dokümantasyonu okuyun
4. Dağıtım hazırlığı yapın

---

## 📞 Yardım & Destek

**Sorun mu yaşıyorsunuz?**

1. `.\test-build.ps1` çalıştırın
2. Hata mesajını kopyalayın
3. [BUILD_GUIDE.md](BUILD_GUIDE.md) → Sorun Giderme bölümüne bakın
4. Hala çözemiyorsanız → GitHub Issues

---

<div align="center">

## 🎉 BAŞARILAR!

**3 adımda profesyonel kurulum paketi!**

```powershell
.\test-build.ps1              # 1. Test
cargo build --release         # 2. Derle
.\build-installer.ps1 -SkipGUI  # 3. Paketle
```

**Sonuç:** `release/ROGCtl-Setup-v1.0.0.exe`

**→ Kullanıma hazır! 🚀**

</div>

---

**💡 İpucu:** İlk kez build yapıyorsanız, [QUICK_START.md](QUICK_START.md) okuyun.

**🔧 Sorun mu var?** [BUILD_GUIDE.md](BUILD_GUIDE.md) → Sorun Giderme

**🖥️ GUI eklemek mi istiyorsunuz?** [GUI_SETUP_GUIDE.md](GUI_SETUP_GUIDE.md)

# ⚙️ Kurulum Gereksinimleri

Build yapmadan önce bu araçları kurmanız gerekiyor.

## ✅ KURULU OLANLAR

- ✅ **Rust** - Kurulu ve çalışıyor
- ✅ **Cargo** - rogctl.exe başarıyla derlendi
- ✅ **CLI Binary** - 0.78 MB, hazır

## ❌ EKSİK OLAN

### Inno Setup 6 (Zorunlu)

Kurulum paketi oluşturmak için **Inno Setup 6** gerekli.

#### Seçenek 1: Winget ile Kur (Önerilen)

```powershell
winget install JRSoftware.InnoSetup
```

#### Seçenek 2: Manuel İndir

1. https://jrsoftware.org/isdl.php adresine git
2. "Inno Setup 6.x" indir (ücretsiz)
3. Kur (varsayılan ayarlarla)

**Kurulum sonrası kontrol:**
```powershell
Test-Path "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
# TRUE döner mi?
```

---

## ⚠️ OPSIYONEL

### İkon Dosyası (Önerilir)

Şu anda varsayılan ikon kullanılacak. Özel ikon eklemek için:

1. 256x256 PNG logo bul
2. https://icoconverter.com/ ile ICO'ya dönüştür
3. Kaydet: `installer/assets/rogctl.ico`

---

## 🚀 Hazırsınız!

Inno Setup kurduktan sonra:

```powershell
# Test et
powershell -ExecutionPolicy Bypass -File "quick-test.ps1"

# Build yap
powershell -ExecutionPolicy Bypass -File "build-installer.ps1" -SkipGUI
```

**Sonuç:** `release/ROGCtl-Setup-v1.0.0.exe`

---

## 📞 Yardım

**Sorun mu yaşıyorsunuz?**

```powershell
# Inno Setup kontrol
Test-Path "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"

# Winget kurulu mu?
winget --version

# Manuel kurulum
# https://jrsoftware.org/isdl.php
```

---

## ✨ Özet

| Gereksinim | Durum | Aksiyon |
|------------|-------|---------|
| Rust | ✅ Kurulu | - |
| CLI Binary | ✅ Hazır (0.78 MB) | - |
| **Inno Setup** | ❌ Eksik | **→ Kur!** |
| İkon | ⚠️ Opsiyonel | İsteğe bağlı |

**👉 Inno Setup'ı kurun, sonra tekrar deneyin!**

# ROGCtl Installer Assets

Bu klasör kurulum sihirbazı için gerekli görsel dosyaları içerir.

## Gerekli Dosyalar

### 1. rogctl.ico (Zorunlu)
- **Format:** ICO (Windows Icon)
- **Boyutlar:** 16x16, 32x32, 48x48, 256x256
- **Kullanım:** 
  - Uygulama ikonu
  - Kurulum sihirbazı ikonu
  - Kısayol ikonu

**Nasıl oluşturulur:**
```powershell
# Online tool kullanın:
# https://www.icoconverter.com/
# https://convertio.co/png-ico/

# VEYA ImageMagick ile:
magick convert logo.png -define icon:auto-resize=256,48,32,16 rogctl.ico
```

### 2. wizard-image.bmp (Opsiyonel)
- **Format:** BMP (24-bit)
- **Boyut:** 164x314 pixels
- **Kullanım:** Kurulum sihirbazı sol panel görseli

**Tasarım önerileri:**
- Dikey layout
- Koyu arka plan (#0f0f0f - #1a1a1a)
- ROGCtl logosu üstte
- Minimal, modern stil
- ROG teması (kırmızı vurgular #ff0050)

### 3. wizard-small.bmp (Opsiyonel)
- **Format:** BMP (24-bit)
- **Boyut:** 55x55 pixels
- **Kullanım:** Kurulum sihirbazı başlık ikonu

**Tasarım önerileri:**
- Kare format
- Basit, tanınabilir logo
- Şeffaf arka plan (beyaz olarak kaydedilir, InnoSetup otomatik maskeler)

## Hızlı Başlangıç

### Minimum Gereksinim (Sadece İkon)

Eğer profesyonel görseller yoksa, minimum olarak bir ikon dosyası yeterli:

1. Herhangi bir PNG logo bulun (256x256 veya daha büyük)
2. Online ICO converter kullanın
3. `rogctl.ico` olarak kaydedin

Kurulum bu ikon ile çalışacaktır. Wizard görselleri olmadan sade bir kurulum arayüzü gösterilir.

### Tam Özellikli Kurulum

Profesyonel görünüm için üç dosyayı da ekleyin:

```
installer/assets/
├── rogctl.ico          # Ana ikon (ZORUNLU)
├── wizard-image.bmp    # Sol panel (önerilen)
└── wizard-small.bmp    # Başlık ikonu (önerilen)
```

## Önerilen Tasarım Araçları

### Ücretsiz
- **GIMP** - Genel grafik düzenleme
- **Inkscape** - Vektör grafik (SVG → PNG → ICO)
- **Paint.NET** - Basit düzenleme
- **Online Tools** - icoconverter.com, convertio.co

### Ücretli
- **Adobe Photoshop** - Profesyonel düzenleme
- **Figma** - Modern UI tasarımı
- **Sketch** - macOS tasarım aracı

## Mevcut Dosyalar

Şu anda bu klasör **boş** durumda. Kendi görsellerinizi ekleyin veya placeholder kullanın.

### Placeholder Kullanımı

Eğer henüz görselleriniz yoksa:

1. **Geçici kullanım için:**
   ```powershell
   # Windows varsayılan ikonunu kopyalayın (test için)
   Copy-Item C:\Windows\System32\SHELL32.dll rogctl.ico
   ```

2. **Build script otomatik kontrol yapar:**
   `build-installer.ps1` çalıştırıldığında eksik görseller için uyarı verir ancak kurulumu engelmez.

## ROG Teması Renk Paleti

Armoury Crate tarzında tasarım için önerilen renkler:

```
Birincil:
  Kırmızı: #ff0050 (ROG signature)
  Koyu:    #0f0f0f (arka plan)
  Orta:    #1a1a1a (kart arka planı)

İkincil:
  Pembe:   #ff3366
  Mavi:    #00d4ff (vurgular)
  Yeşil:   #00ff88 (başarı)

Metin:
  Beyaz:   #ffffff (başlık)
  Gri:     #b0b0b0 (normal metin)
  Soluk:   #808080 (yardımcı metin)
```

## Kontrol Listesi

Kurulum paketini oluşturmadan önce:

- [ ] `rogctl.ico` dosyası mevcut
- [ ] İkon boyutları doğru (16, 32, 48, 256)
- [ ] İkon şeffaf arka planlı
- [ ] Wizard görselleri mevcut (opsiyonel)
- [ ] BMP dosyaları 24-bit formatında
- [ ] Görseller ROG temasına uygun

## Yardım

Görsel oluşturmada yardıma mı ihtiyacınız var?

1. GitHub Issues'ta soru açın
2. Örnek görselleri inceleyin: [ASUS ROG Brand Guidelines]
3. Community'den hazır template isteyin

---

**Not:** Bu dosyalar kurulum paketinin görsel kalitesini etkiler ancak işlevsellik için zorunlu değildir. Minimum olarak sadece `rogctl.ico` yeterlidir.

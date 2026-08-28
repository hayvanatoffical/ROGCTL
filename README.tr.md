# rogctl

[English](README.md) | **Türkçe**

ASUS ROG dizüstü bilgisayarlar için termal ve güç denetleyicisi. Armoury
Crate'in manuel modunun yerini alır: makinenin ne yaptığını ölçer, ona göre fan
eğrisini ve GPU saat tavanını değiştirir, açılışta kendiliğinden başlar.

Geliştirildiği ve ölçüldüğü makine bir **ROG Strix SCAR 15 (G533ZW,
i9-12900H + RTX 3070 Ti)**. Buradaki varsayılan sayılar o makinede ölçüldü.
Başka bir ASUS modelinde çalışır, ama sayıları `rogctl kurulum` ile kendine
göre ayarlaman gerekir.

## Ne yapar

- **İş yükünü sınıflandırır** (boşta / film / ofis / hafif oyun / AAA oyun /
  render) — CPU yükü, GPU yükü, VRAM doluluğu ve NVDEC sayacından.
- **Fan eğrisini** o moda göre ACPI üzerinden yazar.
- **GPU saat tavanını** bir sıcaklık hedefini tutacak şekilde kısar. Güç
  ≈ V²·f olduğu için saati kısmak sürücüyü daha düşük voltaja indirir; yani
  desteklenen bir API üzerinden yapılan undervolt.
- **Kare hızını sınırlar** (NVIDIA sürücü profili). Ekranda görünmeyen kare saf
  ısıdır; bu, elimizdeki en büyük tek ısı kolu.
- **Standby RAM'i** mod geçişlerinde ve gerçek bellek baskısında temizler.
- Prizde/pilde farklı davranır; pilde mod yükselmesini bir tavana kadar kısar.

## Donanım desteği

| Katman | Gereksinim |
|---|---|
| Fan eğrileri, güç modu, PPT, fan RPM | **Yalnızca ASUS** (ATKACPI sürücüsü) |
| GPU saat tavanı, GPU telemetrisi | Herhangi bir NVIDIA GPU (NVML) |
| Kare hızı sınırı, VSync, VRR okuma | Herhangi bir NVIDIA GPU (NVAPI) |
| İş yükü sınıflandırma, standby RAM, pil algılama | Marka bağımsız |

Şu an daemon açılırken ATKACPI'yi zorunlu tutuyor, yani **ASUS olmayan bir
makinede başlamaz**. NVIDIA tarafı (`rogctl nv`, `rogctl valorant`) aynı
sınırlamaya tabi değil, ama daemon çalışmadan otomatik işlemez.

## Kurulum

```powershell
cargo build --release --workspace
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

Betik gerekirse kendini yönetici olarak yeniden başlatır — GPU saat kilidi NVML
üzerinden yalnızca yükseltilmiş haklarla yazılabiliyor. Kurulum, oturum
açılışında yönetici olarak çalışan `rogctl` adlı bir zamanlanmış görev oluşturur
ve `bin\` klasörünü PATH'e ekler. Ayrıca arayüzü `bin\rogctl-gui.exe` olarak
kurar ve Başlat menüsüne ve masaüstüne kısayol koyar.

## Arayüz

Günlük kullanımın tamamı buradan yapılır — Başlat menüsünden **rogctl**. Komut
yazmak gerekmez. Açılırken bir kez UAC sorar (fan eğrisi ve saat kilidi
yükseltilmiş hak ister), sonra hiçbir şey sormaz.

| Sekme | Ne var |
|---|---|
| Durum | canlı sıcaklık, yük, fan RPM, GPU watt, VRAM, uygulanan zarf |
| Modlar | altı modun fan aralığı, dizi, GPU saat tabanı/tavanı, sıcaklık hedefleri — ve BIOS'a yazılacak sekiz noktalı fan eğrisinin grafiği |
| Kare hızı | panel tazeleme, VRR durumu, yazılacak sınırın tam değeri ve **neden o sayı olduğu**; hangi sürücü profilinin sınırladığı |
| Valorant | hesap başına ayrı fps sınırları, hepsini hedefe eşitleme veya serbest bırakma |
| Sistem | daemon başlat/durdur, ASUS servisleri, bellek temizliği, pil ölçekleri, donanım taraması |

Arayüz donanıma kendisi dokunmaz: canlı değerleri daemon'un yazdığı
`status.txt`'ten okur, ayarları `rogctl.yaml`'a yazar, gerisini `rogctl.exe`'ye
devreder. ACPI ve NVML kilidini iki süreç birden tutmaz.

**Hiçbir şey kendiliğinden uygulanmaz.** Değiştirdiğin her şey önce bellekte
durur, üst çubukta "kaydedilmemiş değişiklik" yazar; ancak **Uygula**'ya
basınca dosyaya yazılır ve daemon yeniden başlatılır. **Geri al** her şeyi
diskteki hâline döndürür.

Aynı ayarları terminalden yapmak istersen sihirbaz da duruyor:

```powershell
rogctl kurulum
```

Sihirbaz önce donanımı tarar (ATKACPI kaç aygıt destekliyor, NVML/NVAPI açıldı
mı, panel kaç Hz, VRR açık mı), sonucu düz Türkçe yazar, sonra dört şey sorar:
öncelik (performans / denge / sessiz), kare hızı sınırı, standby RAM temizliği,
Armoury Crate servislerinin durdurulması. **Sorulmadan hiçbir şey değişmez** ve
mevcut `rogctl.yaml` üzerine yazılmadan önce yedeklenir.

Kaldırmak için:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -Uninstall
```

Kaldırma fan eğrilerini Armoury Crate'e geri bırakır ve GPU saat kilidini
serbest bırakır. Sürücü profiline yazılan ayarlar KALICIDIR ve kaldırma onlara
dokunmaz — onları ayrıca geri almak için `rogctl nv sifirla`.

## Günlük kullanım

Hiçbir şey. Açılışta başlar, kendi kendine çalışır. Bir şeyi değiştirmek ya da
görmek istersen arayüzü aç. Terminali tercih edersen:

```
rogctl status     # daemon ne yapıyor
rogctl rapor      # log'dan HTML rapor üret ve tarayıcıda aç
rogctl mon 60     # canlı telemetri
rogctl kurulum    # sihirbazı tekrar çalıştır
rogctl            # komut listesi
```

## Bu makinede DENENMİŞ ve ÇALIŞMAYAN kollar

Bunlar varsayım değil; hepsi denendi ve reddedildi. Tekrar denemeye değmez:

| Kol | Sonuç |
|---|---|
| CPU güç limiti (ACPI PPT, PERF_MODE, powercfg PROCTHROTTLEMAX) | ASUS firmware kilitli. i9-12900H tasarımı gereği Tjmax'i hedefler; 95 °C normaldir. |
| NVML GPU güç limiti | `NVML_ERROR_NOT_SUPPORTED` (vBIOS kilidi) |
| MSR üzerinden her şey | Memory Integrity (HVCI) açık, WinRing0 yüklenemiyor |

Elimizde gerçekten çalışan kollar: ACPI fan eğrileri, NVML saat kilidi, NVAPI
sürücü profili, standby RAM temizliği.

**CPU 95 °C görüyorsan bu bir arıza değil.** Bu makinede sıcaklığı düşürecek bir
güç kolu yok; yapılabilecek tek şey ısıyı daha iyi atmak, o da fan eğrisi.

## Kare hızı ve Valorant

Kare sınırı NVIDIA sürücü profiline yazılır ve rogctl kapansa bile durur.
Sınırın tazeleme hızının hangi tarafına kurulacağını VRR belirler:

- **VRR/G-SYNC açık** (bu panelde varsayılan) → sınır tazelemenin `vrr_margin`
  kadar **altına** (144 → 141). Üstüne çıkan kare VRR penceresinden düşer.
- **VRR kapalı** → sınır tazelemenin `fps_headroom` kadar **üstüne** (144 → 151),
  çünkü sürücünün sınırlayıcısı hedefin birkaç kare altında kalıyor.

Hangi profilin sınırladığını görmek için:

```
rogctl nv kim
```

**Valorant ayrı bir durum.** Dört kare sınırını kendi dosyasında tutar
(`RiotUserSettings.ini`) ve bunlar sürücüden görünmez — makine sürücü seviyesinde
kusursuz ayarlıyken oyun 60 fps'te kilitli kalabilir. Sınırlar **hesap başınadır**;
bir hesabı düzeltmek diğerlerini düzeltmez.

```
rogctl valorant                 # son oynanan hesabın sınırlarını göster
rogctl valorant hepsi           # tüm hesaplar
rogctl valorant hepsi duzelt    # hepsini tazeleme hızına eşitle
rogctl valorant hepsi serbest   # sınırı tamamen kaldır
```

`serbest`, oyunun ini'sindeki anahtarları kapatmanın yanında sürücüdeki
**Valorant profiline** `Frame Rate Limiter = 0` yazar. Uygulama profili temel
profili ezdiği için genel sınır diğer her şeyde geçerli kalır. `duzelt` bunu
geri alır.

> Bu komutlar Valorant **kapalıyken** çalıştırılmalı. Oyun çıkarken ini dosyasını
> bellekten yeniden yazıp değişikliği sessizce siler; komut oyun açıkken
> çalışmayı zaten reddeder. Her yazmadan önce `.rogctl-yedek` yedeği alınır.

## Ayarlar

`bin\rogctl.yaml`. Düzenledikten sonra:

```powershell
Stop-ScheduledTask rogctl ; Start-ScheduledTask rogctl
```

Öne çıkanlar:

- `modes:` — her mod için fan aralığı, GPU saat tavanı/tabanı, sıcaklık hedefleri
- `games:` — belirli bir exe çalışırken modu dayat (sınıflandırıcıyı ezer)
- `battery:` — pilde fan ve saat ölçekleri, mod tavanı
- `nvidia.refresh_hz` — 0 paneli canlı okur; bir sayı yazarsan sınır her zaman o
  hızdan hesaplanır (oturum açılışında yanlış hız okunmasına karşı)
- `nvidia.respect_vrr` — false yaparsan VRR açıkken bile +`fps_headroom` kuralı

Dosya asla üzerine yazılmaz. Ölçülmüş bir varsayılan değiştiğinde
`CONFIG_VERSION` artar ve kurulum eskisini `rogctl.yaml.eski` diye yedekleyip
yenisini ürettirir.

## Kaçınılacak ayar

`Idle Application Max FPS Limit` (0x10835016) kapalı bırakılmalı. Sürücünün
"boşta" kararı oyun oynarken de tetikleniyor ve görüntüyü 30 fps'e kilitliyor.

## Bir şeyler ters giderse

```
rogctl status          # daemon ayakta mı, ne yapıyor
rogctl probe           # hangi donanım kolları açılabiliyor
rogctl selftest        # yazma yollarını dene
rogctl nv kim          # kare hızını kim sınırlıyor
```

Log: `bin\rogctl.log` (1.5 MB'ı geçince `rogctl.log.1` olarak devredilir).

## Kaynak düzeni

Depo bir cargo workspace'i: kök paket çekirdek ve komut satırı, `gui/` arayüz.
İkisi de `src/lib.rs`'e bakar, yani arayüzün gösterdiği değer ile daemon'un
uyguladığı değer aynı koddan gelir.

| Dosya | İçerik |
|---|---|
| `lib.rs` | çekirdeğin modül listesi — CLI ve arayüzün ortak tabanı |
| `main.rs` | komut dağıtımı, daemon döngüsü, durum/rapor çıktısı |
| `kurulum.rs` | kurulum sihirbazı: donanım taraması, sorular, ayar yazımı |
| `policy.rs` | iş yükü sınıflandırma, mod zarfları, saat governor'ı |
| `telemetry.rs` | CPU/GPU örneklemesi |
| `acpi.rs`, `control.rs`, `devices.rs` | ACPI fan eğrileri ve cihaz yazma |
| `gpu.rs` | NVML (saat kilidi, yük, VRAM) |
| `nvapi.rs` | NVIDIA sürücü profili (DRS) |
| `valorant.rs` | Valorant'ın kendi ayar dosyası |
| `memory.rs` | standby listesi temizliği |
| `report.rs` | HTML rapor üretimi |
| `gui/src/tema.rs` | tek renk paleti ve kart/başlık/ölçüm bileşenleri |
| `gui/src/veri.rs` | dışarıyla konuşan tek katman: status.txt, yaml, rogctl.exe, UAC |
| `gui/src/ekran_*.rs` | beş sekmenin içerikleri |

`deneysel/` yarıda bırakılmış bir GUI ve kurulum iskeleti tutuyor. İçindeki
hiçbir şey bağlı değil ve derlemenin parçası değil — kendi README'sine bak.

## Lisans

Bkz. [LICENSE.txt](LICENSE.txt).

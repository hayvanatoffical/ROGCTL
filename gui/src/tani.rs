//! Tanı: ölçülen durumdan "ne yapmalıyım" cümlesi üretir.
//!
//! Durum ekranı sayıları gösteriyordu ama sayı tek başına karar verdirmiyor.
//! 90 derece gören biri makinenin bozulduğunu sanıyor; oysa bu makinede 90,
//! yarım saniyelik bir turbo tepesi de olabilir, saatlerdir süren bir hedef de.
//! İkisi arasındaki farkı kullanıcı ham sayıdan çıkaramaz - bu modülün işi o
//! farkı çıkarıp tek cümleyle söylemek.
//!
//! Kural: her bulgu **neden** o sonuca varıldığını ölçülen sayıyla söyler ve
//! **hangi kolun** onu değiştirdiğini gösterir. Kullanıcıyı yönlendirecek bir
//! sekme varsa bulgu onu taşır, ekran da "Git" düğmesi koyar.

use rogctl::config::{Config, EnvelopeCfg};
use rogctl::control::build_curve;
use rogctl::policy::{Envelope, Mode};

use crate::gecmis::{self, Egilim, Gecmis};
use crate::veri::Durum;
use crate::Sekme;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Seviye {
    /// Kullanıcının bir şey yapması gerekiyor.
    Sorun,
    /// Bilmesi iyi olur ama acil değil.
    Uyari,
    /// Yalnızca açıklama - çoğu zaman "bu normal, panik yok" demek için.
    Bilgi,
    /// Doğrulanmış iyi durum.
    Iyi,
}

pub struct Bulgu {
    pub seviye: Seviye,
    pub baslik: String,
    /// Ne olduğu ve neden - ölçülen sayılarla.
    pub aciklama: String,
    /// Somut eylem. Yapacak bir şey yoksa boş.
    pub ne_yapmali: String,
    /// Kolun bulunduğu sekme.
    pub sekme: Option<Sekme>,
}

impl Bulgu {
    fn yeni(seviye: Seviye, baslik: &str, aciklama: String) -> Self {
        Self {
            seviye,
            baslik: baslik.to_string(),
            aciklama,
            ne_yapmali: String::new(),
            sekme: None,
        }
    }

    fn yap(mut self, metin: &str, sekme: Sekme) -> Self {
        self.ne_yapmali = metin.to_string();
        self.sekme = Some(sekme);
        self
    }
}

/// Eğrinin verilen sıcaklıkta istediği fan yüzdesi.
///
/// BIOS sekiz nokta arasını doğrusal bağladığı için burada da öyle hesaplanır;
/// amaç ekranda "zarf %8-70" gibi bir aralık yerine "şu an %43 isteniyor"
/// diyebilmek. Aralık, kullanıcıya fanın tavana kilitli olduğunu düşündürüyordu.
pub fn egrinin_istedigi(env: &EnvelopeCfg, sicaklik: u32) -> u32 {
    let egri = build_curve(&Envelope::from(*env), env.cpu_temp_target);
    let t = sicaklik as f32;

    if t <= egri[0] as f32 {
        return egri[8] as u32;
    }
    for i in 1..8 {
        let (t0, t1) = (egri[i - 1] as f32, egri[i] as f32);
        if t <= t1 {
            let (p0, p1) = (egri[i + 7] as f32, egri[i + 8] as f32);
            if (t1 - t0).abs() < 0.5 {
                return p1 as u32;
            }
            return (p0 + (p1 - p0) * (t - t0) / (t1 - t0)).round() as u32;
        }
    }
    egri[15] as u32
}

/// `status.txt` modu gosterim adiyla yaziyor; ayar dosyasinin anahtari baska.
/// Ikisini karistirmak sessizce bos zarf dondurur, o yuzden ceviri tek yerde.
fn mod_anahtari(mod_ad: &str) -> Option<&'static str> {
    Mode::ALL
        .into_iter()
        .find(|m| m.label() == mod_ad)
        .map(|m| m.key())
}

/// Daemon'un bildirdigi modun ayar dosyasindaki zarfi.
pub fn mod_zarfi(cfg: &Config, mod_ad: &str) -> Option<EnvelopeCfg> {
    mod_anahtari(mod_ad).and_then(|k| cfg.modes.get(k)).copied()
}

/// Tüm kontroller. Sıra önemli: en ağır bulgu en üstte.
pub fn topla(durum: Option<&Durum>, cfg: &Config, gecmis: &Gecmis, kirli: bool) -> Vec<Bulgu> {
    let mut b = Vec::new();

    let Some(d) = durum else {
        b.push(
            Bulgu::yeni(
                Seviye::Sorun,
                "Daemon hiç çalışmamış",
                "Durum dosyası yok, yani fan eğrileri ve saat tavanı bu makinede hiç \
                 uygulanmıyor. Armoury Crate ne yapıyorsa o geçerli."
                    .to_string(),
            )
            .yap("Sistem sekmesinden daemon'u başlat.", Sekme::Sistem),
        );
        return b;
    };

    if !d.canli() {
        b.push(
            Bulgu::yeni(
                Seviye::Sorun,
                "Daemon durmuş",
                format!(
                    "Son durum {} saniye önce yazılmış. Duran daemon fan eğrisini de \
                     bırakır; birkaç saniye içinde Armoury Crate kendi eğrisini geri yazar.",
                    d.yas_s
                ),
            )
            .yap("Sistem sekmesinden yeniden başlat.", Sekme::Sistem),
        );
    }

    if kirli {
        b.push(
            Bulgu::yeni(
                Seviye::Uyari,
                "Kaydedilmemiş değişiklik var",
                "Değiştirdiğin ayarlar şu an yalnızca bellekte. Daemon hâlâ diskteki eski \
                 değerleri uyguluyor."
                    .to_string(),
            )
            .yap("Üstteki Uygula düğmesine bas.", Sekme::Durum),
        );
    }

    sicaklik_bulgusu(&mut b, d, cfg, gecmis);
    sogutma_bulgusu(&mut b, cfg, gecmis, d);
    gpu_bulgusu(&mut b, d, gecmis);
    servis_bulgusu(&mut b, d, cfg);
    kare_bulgusu(&mut b, cfg);
    pil_bulgusu(&mut b, d, cfg);
    bellek_bulgusu(&mut b, d, cfg);

    if b.iter().all(|x| x.seviye == Seviye::Iyi || x.seviye == Seviye::Bilgi) {
        b.insert(
            0,
            Bulgu::yeni(
                Seviye::Iyi,
                "Yapman gereken bir şey yok",
                "Daemon çalışıyor, sıcaklıklar bu modun hedefinde ve fan eğrisini geri yazan \
                 bir servis yok."
                    .to_string(),
            ),
        );
    }

    b
}

/// Sıcaklık: bu makinede en çok yanlış anlaşılan sayı.
fn sicaklik_bulgusu(b: &mut Vec<Bulgu>, d: &Durum, cfg: &Config, g: &Gecmis) {
    if g.sayi() < 8 {
        return; // Karar vermek için henüz yeterli örnek yok.
    }

    let surekli = g.ortalama(gecmis::cpu_c);
    let tepe = g.tepe(gecmis::cpu_c);
    let yuk = g.ortalama(gecmis::cpu_util);
    let egilim = g.egilim(gecmis::cpu_c);
    let hedef = d.hedef_cpu_c;

    let env = mod_zarfi(cfg, &d.mod_ad);
    let istenen = env.map(|e| egrinin_istedigi(&e, surekli));

    // 95 derece bu yongada arıza değil; tasarım gereği Tjmax'i hedefler ve
    // bu makinede CPU gücünü kısacak bir kol yok. Bunu söylememek, kullanıcıyı
    // olmayan bir arızayı kovalamaya iter.
    if surekli >= 90 {
        let mut m = Bulgu::yeni(
            Seviye::Uyari,
            "CPU sürekli çok sıcak",
            format!(
                "Son bir dakikanın ortalaması {surekli} C (tepe {tepe} C), ortalama yük %{yuk}. \
                 Bu i9'da 95 C bir arıza değil - yonga tasarımı gereği sınırına kadar çıkar ve \
                 bu makinede CPU gücünü kısacak bir kol yok. Kalıcı çözüm fan tarafında: \
                 {}",
                match istenen {
                    Some(p) if p < 90 => format!(
                        "eğri şu an %{p} istiyor, yani {} puan pay duruyor",
                        100 - p
                    ),
                    _ => "eğri zaten tavanda".to_string(),
                }
            ),
        );
        if env.is_some() {
            m = m.yap(
                "Modlar sekmesinde bu modun sıcaklık hedefini düşür ya da fan tavanını yükselt.",
                Sekme::Modlar,
            );
        }
        b.push(m);
        return;
    }

    // Hedefe yapışmış: sistem bozuk değil, kendisine söyleneni yapıyor. Fark
    // buradan anlatılmazsa kullanıcı sayıyı arıza sanıyor.
    if hedef > 0 && surekli + 3 >= hedef && yuk < 40 {
        let mut m = Bulgu::yeni(
            Seviye::Uyari,
            "Sıcaklık hedefe yapışmış, yük ise düşük",
            format!(
                "Ortalama {surekli} C ve bu modun hedefi {hedef} C - yani makine bozulmuyor, \
                 hedefi tutuyor. Ama ortalama yük yalnızca %{yuk}. Bu yongada %{yuk} demek \
                 birkaç çekirdeğin tam turboda dönmesi demek; hedef yüksek olduğu için \
                 sıcaklık orada dengeleniyor{}.",
                match istenen {
                    Some(p) => format!(" ve fan %{p} ile yetiniyor"),
                    None => String::new(),
                }
            ),
        );
        if env.is_some() {
            m = m.yap(
                "Daha serin istiyorsan Modlar sekmesinde bu modun CPU hedefini düşür - \
                 fan erken devreye girer, ses artar.",
                Sekme::Modlar,
            );
        }
        b.push(m);
        return;
    }

    // Tepe yüksek ama ortalama değil: en sık panik sebebi, ve yapılacak bir şey yok.
    if tepe >= 88 && surekli + 12 <= tepe {
        b.push(Bulgu::yeni(
            Seviye::Bilgi,
            "Yüksek sayılar anlık tepe, sürekli değil",
            format!(
                "Son bir dakikada tepe {tepe} C görüldü ama ortalama {surekli} C. Tek bir \
                 çekirdeğin turboya çıkması bu yongada paket sıcaklığını bir saniyeden kısa \
                 sürede 20 derece zıplatıyor; soğutma bunu yakalayıp geri indiriyor. \
                 Yapılacak bir şey yok."
            ),
        ));
        return;
    }

    if egilim == Egilim::Yukseliyor && surekli >= 80 {
        b.push(Bulgu::yeni(
            Seviye::Bilgi,
            "Sıcaklık yükseliyor",
            format!("Ortalama {surekli} C ve son dakikada yükseliş var (yük %{yuk})."),
        ));
        return;
    }

    b.push(Bulgu::yeni(
        Seviye::Iyi,
        "Sıcaklık hedefin altında",
        format!(
            "Ortalama {surekli} C, tepe {tepe} C, hedef {hedef} C - {}.",
            egilim.metin()
        ),
    ));
}

/// Fan zaten sonuna kadar dönüyorsa yazılımda kalan kol yoktur.
///
/// Bunu söylemeyen bir arayüz kullanıcıyı, hiçbir şeyi değiştirmeyecek
/// ayarları kurcalamaya iter. Ölçülen tavan bu kasada 4600 RPM civarı.
fn sogutma_bulgusu(b: &mut Vec<Bulgu>, cfg: &Config, g: &Gecmis, d: &Durum) {
    if g.sayi() < 20 {
        return;
    }
    let devir = g.ortalama(gecmis::cpu_fan);
    let sicak = g.ortalama(gecmis::cpu_c);
    if devir < 4300 || sicak < 85 {
        return;
    }
    let tavan = mod_zarfi(cfg, &d.mod_ad).map(|e| e.fan_max_pct).unwrap_or(100);
    b.push(Bulgu::yeni(
        Seviye::Bilgi,
        "Fanlar zaten sonuna kadar",
        format!(
            "Son bir dakikada CPU fanı ortalama {devir} RPM ve sıcaklık {sicak} C. Bu modun              fan izni %{tavan}; yazılım tarafında verilecek bir şey kalmadı. Sıcaklığı buradan              daha aşağı çekmenin tek yolu makineye daha soğuk hava vermek - altlık kullanıyorsan              yüksek kademe, kullanmıyorsan makineyi düz ve kapalı olmayan bir yüzeye almak."
        ),
    ));
}

/// GPU hedefini aşıyorsa, saat tavanı kolunun ne yaptığını söyle.
fn gpu_bulgusu(b: &mut Vec<Bulgu>, d: &Durum, g: &Gecmis) {
    if g.sayi() < 20 || d.hedef_gpu_c == 0 {
        return;
    }
    let surekli = g.ortalama(gecmis::gpu_c);
    if surekli <= d.hedef_gpu_c {
        return;
    }
    b.push(
        Bulgu::yeni(
            Seviye::Uyari,
            "GPU hedefinin üstünde",
            format!(
                "GPU ortalaması {surekli} C, hedefi {} C. Bu makinede GPU güç limiti                  yazılamıyor (vBIOS reddediyor), o yüzden tek kol saat tavanı: şu an                  {} MHz'e kilitli ve hedefin üstünde kaldıkça daha da iniyor. Kare sınırı                  koymak bunu kaynağında keser - görünmeyen kare saf ısıdır.",
                d.hedef_gpu_c, d.tavan_mhz
            ),
        )
        .yap("Kare hızı sekmesinden sınırı ayarla.", Sekme::Kare),
    );
}

fn servis_bulgusu(b: &mut Vec<Bulgu>, d: &Durum, cfg: &Config) {
    let durduruldu = d.askidaki_servisler != "-" && !d.askidaki_servisler.is_empty();
    if durduruldu {
        b.push(Bulgu::yeni(
            Seviye::Iyi,
            "Fan eğrisini geri yazan servis yok",
            format!("Askıya alınanlar: {}.", d.askidaki_servisler),
        ));
        return;
    }
    if !cfg.suspend_asus_services {
        b.push(
            Bulgu::yeni(
                Seviye::Sorun,
                "Armoury Crate eğriyi geri yazıyor olabilir",
                "ASUS termal servisleri durdurulmuyor. Çalışırlarken rogctl'in yazdığı fan \
                 eğrisi birkaç saniye içinde siliniyor - yani Modlar sekmesinde ne ayarlarsan \
                 ayarla, kalıcı olmuyor."
                    .to_string(),
            )
            .yap(
                "Sistem sekmesinde 'ASUS termal servislerini durdur' seçeneğini aç.",
                Sekme::Sistem,
            ),
        );
    }
}

fn kare_bulgusu(b: &mut Vec<Bulgu>, cfg: &Config) {
    if !cfg.nvidia.enabled {
        b.push(
            Bulgu::yeni(
                Seviye::Bilgi,
                "Kare sınırını rogctl yönetmiyor",
                "Sürücüdeki kare sınırına dokunulmuyor. Oyun beklenmedik bir değerde \
                 kilitliyse sebebini rogctl bilmiyor."
                    .to_string(),
            )
            .yap(
                "Kare hızı sekmesinden açabilir, kimin sınırladığını tarayabilirsin.",
                Sekme::Kare,
            ),
        );
    }
    if cfg.nvidia.enabled && cfg.nvidia.idle_max_fps > 0 {
        b.push(
            Bulgu::yeni(
                Seviye::Uyari,
                "Boşta kare sınırı açık",
                format!(
                    "Boşta sınırı %{} fps'e kurulu. Sürücünün 'boşta' kararı bu makinede oyun \
                     oynarken de tetiklenip görüntüyü bu sınıra kilitliyor.",
                    cfg.nvidia.idle_max_fps
                ),
            )
            .yap("Kare hızı sekmesinde kapatman önerilir.", Sekme::Kare),
        );
    }
}

fn pil_bulgusu(b: &mut Vec<Bulgu>, d: &Durum, cfg: &Config) {
    if d.kaynak == "batarya" {
        b.push(Bulgu::yeni(
            Seviye::Bilgi,
            "Pilde çalışıyor - ölçekler devrede",
            format!(
                "Fan tavanı x{:.2}, GPU saat tavanı x{:.2} ile çarpılıyor{}. Performans \
                 fişteki kadar olmaz; bu beklenen davranış.",
                cfg.battery.fan_max_scale,
                cfg.battery.clock_ceiling_scale,
                match &cfg.battery.cap_mode {
                    Some(m) if !m.is_empty() => format!(" ve mod '{m}' üstüne çıkmıyor"),
                    _ => String::new(),
                }
            ),
        ));
    }
}

fn bellek_bulgusu(b: &mut Vec<Bulgu>, d: &Durum, cfg: &Config) {
    if d.ram_toplam_mb == 0 {
        return;
    }
    let bos_yuzde = (d.ram_bos_mb * 100 / d.ram_toplam_mb.max(1)) as u32;
    if bos_yuzde < 10 && !cfg.memory.enabled {
        b.push(
            Bulgu::yeni(
                Seviye::Uyari,
                "Boş RAM azaldı, temizlik kapalı",
                format!(
                    "Boş {} MB ({bos_yuzde}%), standby listesinde {} MB bekliyor. Oyun \
                     açılırken bu listenin atılabilir ucunu temizlemek takılmayı ölçülebilir \
                     şekilde azaltıyor.",
                    d.ram_bos_mb, d.ram_standby_mb
                ),
            )
            .yap("Sistem sekmesinde bellek temizliğini aç.", Sekme::Sistem),
        );
    }
}

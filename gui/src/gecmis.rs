//! Son birkaç dakikanın ölçüm geçmişi.
//!
//! Tek bir anlık sayı bu makinede yanıltıyor: 20 iş parçacıklı bir i9'da tek
//! bir çekirdeğin turboya çıkması paket sıcaklığını bir saniyeden kısa sürede
//! 20 derece zıplatıyor. Ölçülen bir örnekte yük %4-11 arasında sabitken
//! sıcaklık 63 ile 81 arasında gidip geldi. Ekranda yalnızca o anki değeri
//! göstermek, kullanıcıya sürekli "makine 90 derece" dedirtir - oysa sürekli
//! olan sayı ortalamadır, 90 ise yarım saniyelik bir tepe.
//!
//! Bu yüzden her yerde iki sayı tutuluyor: **sürekli** (ortalama) ve **tepe**.
//! Karar sürekli olana bakarak verilir.

use std::collections::VecDeque;

/// Tek bir örnek. Zaman, daemon'un yazdığı güncelleme değil, okuma anıdır -
/// daemon dursa bile geçmiş yanlış sıkışmaz.
#[derive(Clone, Copy)]
pub struct Ornek {
    pub cpu_c: u32,
    pub gpu_c: u32,
    pub cpu_util: u32,
    pub gpu_util: u32,
    pub cpu_fan: u32,
}

/// Yaklaşık beş dakikalık halka tampon. Arayüz saniyede bir okuduğu için
/// 320 örnek ~5 dakika eder; bellek maliyeti birkaç kilobayt.
const KAPASITE: usize = 320;

/// Ortalamanın alındığı pencere. Bir dakika, hem turbo tepelerini eritecek
/// kadar uzun hem de kullanıcının az önce yaptığı şeyi yansıtacak kadar kısa.
const PENCERE: usize = 60;

#[derive(Default)]
pub struct Gecmis {
    ornekler: VecDeque<Ornek>,
}

impl Gecmis {
    pub fn ekle(&mut self, o: Ornek) {
        if self.ornekler.len() == KAPASITE {
            self.ornekler.pop_front();
        }
        self.ornekler.push_back(o);
    }

    pub fn bos(&self) -> bool {
        self.ornekler.is_empty()
    }

    pub fn sayi(&self) -> usize {
        self.ornekler.len()
    }

    /// Son pencerede toplanan örnekler, en eskiden yeniye.
    fn son(&self) -> impl Iterator<Item = &Ornek> {
        let atla = self.ornekler.len().saturating_sub(PENCERE);
        self.ornekler.iter().skip(atla)
    }

    /// Son pencerenin ortalaması - "sürekli" olan sayı budur.
    pub fn ortalama(&self, alan: fn(&Ornek) -> u32) -> u32 {
        let mut toplam = 0u64;
        let mut adet = 0u64;
        for o in self.son() {
            toplam += alan(o) as u64;
            adet += 1;
        }
        toplam.checked_div(adet).unwrap_or(0) as u32
    }

    /// Son pencerenin tepesi.
    pub fn tepe(&self, alan: fn(&Ornek) -> u32) -> u32 {
        self.son().map(alan).max().unwrap_or(0)
    }

    /// Grafik için ham dizi.
    pub fn dizi(&self, alan: fn(&Ornek) -> u32) -> Vec<f32> {
        self.ornekler.iter().map(|o| alan(o) as f32).collect()
    }

    /// Sıcaklık yükseliyor mu, düşüyor mu, duruyor mu.
    ///
    /// Son pencerenin ilk ve son üçte birini karşılaştırır. Tek tek örneklere
    /// bakmak bu makinede anlamsız: ardışık iki örnek arasında 15 derece fark
    /// olabiliyor.
    pub fn egilim(&self, alan: fn(&Ornek) -> u32) -> Egilim {
        let v: Vec<u32> = self.son().map(alan).collect();
        if v.len() < 12 {
            return Egilim::Belirsiz;
        }
        let ucte_bir = v.len() / 3;
        let ort = |dilim: &[u32]| dilim.iter().map(|x| *x as u64).sum::<u64>() / dilim.len() as u64;
        let bas = ort(&v[..ucte_bir]) as i64;
        let son = ort(&v[v.len() - ucte_bir..]) as i64;
        match son - bas {
            f if f >= 5 => Egilim::Yukseliyor,
            f if f <= -5 => Egilim::Dusuyor,
            _ => Egilim::Duragan,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Egilim {
    Yukseliyor,
    Dusuyor,
    Duragan,
    Belirsiz,
}

impl Egilim {
    pub fn metin(self) -> &'static str {
        match self {
            Egilim::Yukseliyor => "yükseliyor",
            Egilim::Dusuyor => "düşüyor",
            Egilim::Duragan => "durağan",
            Egilim::Belirsiz => "ölçülüyor",
        }
    }
}

pub fn cpu_c(o: &Ornek) -> u32 {
    o.cpu_c
}
pub fn gpu_c(o: &Ornek) -> u32 {
    o.gpu_c
}
pub fn cpu_util(o: &Ornek) -> u32 {
    o.cpu_util
}
pub fn cpu_fan(o: &Ornek) -> u32 {
    o.cpu_fan
}

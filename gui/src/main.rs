//! rogctl arayuzu.
//!
//! Tek pencere, solda bes sekme, sagda o sekmenin karti. Armoury Crate'in
//! gizledigi ya da hic vermedigi kollar burada dogrudan gorunur: GPU saat
//! tavani, kare hizi siniri, VRR'in sinirin hangi tarafina dustugu, Valorant'in
//! hesap basina tuttugu ayri sinirlar, fan araliklari.
//!
//! Kural: hicbir sey kendiliginden uygulanmaz. Degistirilen her sey once
//! bellekte durur, ustte "kaydedilmemis" uyarisi cikar, kullanici "Uygula"ya
//! basinca dosyaya yazilir ve daemon yeniden baslatilir.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ekran_durum;
mod ekran_kare;
mod ekran_modlar;
mod ekran_sistem;
mod ekran_valorant;
mod gecmis;
mod tani;
mod tema;
mod veri;

use std::time::{Duration, Instant};

use egui::RichText;
use rogctl::config::Config;

use veri::Durum;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Sekme {
    Durum,
    Modlar,
    Kare,
    Valorant,
    Sistem,
}

impl Sekme {
    const HEPSI: [Sekme; 5] = [
        Sekme::Durum,
        Sekme::Modlar,
        Sekme::Kare,
        Sekme::Valorant,
        Sekme::Sistem,
    ];

    fn ad(self) -> &'static str {
        match self {
            Sekme::Durum => "Durum",
            Sekme::Modlar => "Modlar",
            Sekme::Kare => "Kare hızı",
            Sekme::Valorant => "Valorant",
            Sekme::Sistem => "Sistem",
        }
    }

    fn alt(self) -> &'static str {
        match self {
            Sekme::Durum => "canlı ölçümler",
            Sekme::Modlar => "fan ve saat zarfları",
            Sekme::Kare => "fps sınırı ve VRR",
            Sekme::Valorant => "hesap başına sınırlar",
            Sekme::Sistem => "servisler ve bellek",
        }
    }
}

/// Kullaniciya gosterilecek gecici bildirim.
pub struct Bildirim {
    pub metin: String,
    pub hata: bool,
    dogum: Instant,
}

impl Bildirim {
    pub fn iyi(metin: impl Into<String>) -> Self {
        Self { metin: metin.into(), hata: false, dogum: Instant::now() }
    }
    pub fn kotu(metin: impl Into<String>) -> Self {
        Self { metin: metin.into(), hata: true, dogum: Instant::now() }
    }
    fn bitti(&self) -> bool {
        self.dogum.elapsed() > Duration::from_secs(6)
    }
}

pub struct Uygulama {
    pub sekme: Sekme,
    pub durum: Option<Durum>,
    pub cfg: Config,
    /// Diske yazilmis hali. "Geri al" ve "degisti mi" bunun uzerinden.
    pub cfg_disk: Config,
    pub bildirim: Option<Bildirim>,
    pub yonetici: bool,
    son_okuma: Instant,
    /// Son birkac dakikanin olcum gecmisi. Tek bir anlik sayi bu makinede
    /// yaniltiyor, kararlar bunun ortalamasina bakilarak veriliyor.
    pub gecmis: gecmis::Gecmis,
    /// Gecmise ayni olcumu iki kez koymamak icin son islenen zaman damgasi.
    son_ornek_s: u64,
    /// Sekmelerin kendi durumlari.
    pub kare: ekran_kare::KareDurumu,
    pub modlar: ekran_modlar::ModlarDurumu,
    pub sistem_tarama: Option<Vec<(String, Result<String, String>)>>,
    pub valorant: ekran_valorant::ValorantDurumu,
}

impl Uygulama {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        tema::uygula(&cc.egui_ctx);

        let (cfg, not) = veri::ayar_yukle();
        let bildirim = not.map(Bildirim::kotu);

        Self {
            sekme: Sekme::Durum,
            durum: Durum::oku(),
            cfg_disk: cfg.clone(),
            cfg,
            bildirim,
            yonetici: veri::yonetici_mi(),
            son_okuma: Instant::now(),
            gecmis: gecmis::Gecmis::default(),
            son_ornek_s: 0,
            kare: ekran_kare::KareDurumu::default(),
            modlar: ekran_modlar::ModlarDurumu::default(),
            sistem_tarama: None,
            valorant: ekran_valorant::ValorantDurumu::default(),
        }
    }

    /// Bellekteki ayar diskten farkli mi.
    pub fn degisti_mi(&self) -> bool {
        serde_yaml::to_string(&self.cfg).ok() != serde_yaml::to_string(&self.cfg_disk).ok()
    }

    pub fn bildir(&mut self, b: Bildirim) {
        self.bildirim = Some(b);
    }

    /// Ayarlari yaz ve daemon'u yeniden baslat.
    fn uygula(&mut self) {
        if let Err(e) = veri::ayar_kaydet(&self.cfg) {
            self.bildir(Bildirim::kotu(format!("Ayar yazılamadı: {e}")));
            return;
        }
        self.cfg_disk = self.cfg.clone();
        match veri::gorev_yeniden_baslat() {
            Ok(_) => self.bildir(Bildirim::iyi("Kaydedildi, daemon yeniden başlatıldı.")),
            Err(e) => self.bildir(Bildirim::kotu(format!(
                "Ayar kaydedildi ama daemon yeniden başlatılamadı: {e}"
            ))),
        }
    }

    /// Canli bir olcumu gecmise ekler.
    ///
    /// Daemon durmussa eklenmez: durmus bir daemon'un son yazdigi satiri
    /// saniyede bir tekrar eklemek, ortalamayi olu bir degere dogru surukler ve
    /// ekranda "sicaklik duragan" yazdirir - oysa olcum yoktur.
    fn gecmise_isle(&mut self) {
        let Some(d) = &self.durum else { return };
        if !d.canli() {
            return;
        }
        // Daemon saniyede bir yaziyor, arayuz 900 ms'de bir okuyor; ayni satiri
        // iki kez saymamak icin dosyanin yasi degismediyse atlanir.
        let damga = d.calisma_s;
        if damga == self.son_ornek_s {
            return;
        }
        self.son_ornek_s = damga;
        self.gecmis.ekle(gecmis::Ornek {
            cpu_c: d.cpu_c,
            gpu_c: d.gpu_c,
            cpu_util: d.cpu_util,
            gpu_util: d.gpu_util,
            cpu_fan: d.cpu_fan,
        });
    }

    fn geri_al(&mut self) {
        self.cfg = self.cfg_disk.clone();
        self.bildir(Bildirim::iyi("Değişiklikler geri alındı."));
    }
}

impl eframe::App for Uygulama {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.son_okuma.elapsed() > Duration::from_millis(900) {
            self.durum = Durum::oku();
            self.son_okuma = Instant::now();
            self.gecmise_isle();
        }
        if self.bildirim.as_ref().is_some_and(Bildirim::bitti) {
            self.bildirim = None;
        }

        yan_panel(self, ctx);
        ust_panel(self, ctx);
        alt_panel(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.sekme {
                Sekme::Durum => ekran_durum::goster(self, ui),
                Sekme::Modlar => ekran_modlar::goster(self, ui),
                Sekme::Kare => ekran_kare::goster(self, ui),
                Sekme::Valorant => ekran_valorant::goster(self, ui),
                Sekme::Sistem => ekran_sistem::goster(self, ui),
            });
        });

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

fn yan_panel(app: &mut Uygulama, ctx: &egui::Context) {
    egui::SidePanel::left("yan")
        .exact_width(196.0)
        .resizable(false)
        .frame(
            egui::Frame::none()
                .fill(tema::KART)
                .inner_margin(egui::Margin::same(12.0)),
        )
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.label(RichText::new("rogctl").size(22.0).strong().color(tema::VURGU));
            ui.label(
                RichText::new("termal ve güç denetimi")
                    .size(11.0)
                    .color(tema::YAZI_SOLUK),
            );
            ui.add_space(18.0);

            for s in Sekme::HEPSI {
                let secili = app.sekme == s;
                let arka = if secili { tema::VURGU_KOYU } else { tema::KART };
                let cerceve = egui::Frame::none()
                    .fill(arka)
                    .rounding(egui::Rounding::same(7.0))
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0));

                let yanit = cerceve
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(s.ad())
                                    .size(13.5)
                                    .strong()
                                    .color(tema::YAZI),
                            );
                            ui.label(RichText::new(s.alt()).size(10.5).color(tema::YAZI_SOLUK));
                        });
                    })
                    .response
                    .interact(egui::Sense::click());

                if yanit.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if yanit.clicked() {
                    app.sekme = s;
                }
                ui.add_space(4.0);
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.add_space(6.0);
                if app.yonetici {
                    tema::rozet(ui, "yonetici", tema::IYI);
                } else {
                    tema::rozet(ui, "sınırlı yetki", tema::ORTA);
                    ui.label(
                        RichText::new("Bazı kollar yazılamaz.")
                            .size(10.5)
                            .color(tema::YAZI_SOLUK),
                    );
                }
            });
        });
}

fn ust_panel(app: &mut Uygulama, ctx: &egui::Context) {
    egui::TopBottomPanel::top("ust")
        .exact_height(58.0)
        .frame(
            egui::Frame::none()
                .fill(tema::ZEMIN)
                .inner_margin(egui::Margin::symmetric(18.0, 12.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(app.sekme.ad()).size(19.0).strong());
                ui.add_space(10.0);

                match &app.durum {
                    Some(d) if d.canli() => {
                        tema::rozet(ui, &format!("daemon çalışıyor  -  {}", d.mod_ad), tema::IYI)
                    }
                    Some(_) => tema::rozet(ui, "daemon durmuş", tema::KOTU),
                    None => tema::rozet(ui, "daemon hiç çalışmamış", tema::ORTA),
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.degisti_mi() {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Uygula").strong().color(egui::Color32::BLACK),
                                )
                                .fill(tema::VURGU),
                            )
                            .clicked()
                        {
                            app.uygula();
                        }
                        if ui.button("Geri al").clicked() {
                            app.geri_al();
                        }
                        ui.label(
                            RichText::new("kaydedilmemiş değişiklik")
                                .size(11.5)
                                .color(tema::ORTA),
                        );
                    }
                });
            });
        });
}

fn alt_panel(app: &mut Uygulama, ctx: &egui::Context) {
    let Some(b) = &app.bildirim else { return };
    let renk = if b.hata { tema::KOTU } else { tema::IYI };
    let metin = b.metin.clone();

    egui::TopBottomPanel::bottom("alt")
        .frame(
            egui::Frame::none()
                .fill(tema::KART)
                .inner_margin(egui::Margin::symmetric(18.0, 10.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(metin).size(12.5).color(renk));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("kapat").clicked() {
                        app.bildirim = None;
                    }
                });
            });
        });
}

fn main() -> eframe::Result {
    // Fan egrisi, saat kilidi ve gorev yonetimi yukseltilmis hak ister. Tek bir
    // UAC istemiyle acilmak, her dugmede ayri istem cikarmaktan iyidir.
    if !veri::yonetici_mi() && veri::yonetici_olarak_yeniden_baslat() {
        return Ok(());
    }

    let secenekler = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 720.0])
            .with_min_inner_size([920.0, 600.0])
            .with_title("rogctl"),
        ..Default::default()
    };

    eframe::run_native(
        "rogctl",
        secenekler,
        Box::new(|cc| Ok(Box::new(Uygulama::new(cc)))),
    )
}

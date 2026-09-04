//! Kare hizi ekrani.
//!
//! Armoury Crate bu kolu hic vermez; NVIDIA paneli verir ama sinirin neden o
//! sayi oldugunu soylemez. Burada uc sey ayni anda gorunur: panelin tazeleme
//! hizi, VRR'in acik olup olmadigi ve bu ikisinden cikan sinirin tam degeri.
//! Slider'i oynattikca sonuc aninda guncellenir, kaydetmeden once gorursun.

use std::time::Instant;

use egui::{RichText, Ui};

use rogctl::nvapi::{self, CapPolicy};

use crate::tema;
use crate::veri;
use crate::{Bildirim, Uygulama};

#[derive(Default)]
pub struct KareDurumu {
    pub panel_hz: u32,
    pub vrr: bool,
    pub nvapi_acildi: bool,
    /// `rogctl nv kim` ciktisi - hangi profilin sinirladigi.
    pub kim: Option<String>,
    okundu: Option<Instant>,
}

impl KareDurumu {
    /// Panel ve VRR durumunu surucuden oku. Pahali oldugu icin kendiliginden
    /// tekrarlanmaz; ekran ilk acildiginda ve kullanici isteyince calisir.
    fn tazele(&mut self) {
        self.panel_hz = nvapi::refresh_hz();
        match nvapi::NvApi::open() {
            Ok(nv) => {
                self.nvapi_acildi = true;
                self.vrr = nvapi::vrr_enabled(&nv);
            }
            Err(_) => {
                self.nvapi_acildi = false;
                self.vrr = false;
            }
        }
        self.okundu = Some(Instant::now());
    }
}

pub fn goster(app: &mut Uygulama, ui: &mut Ui) {
    if app.kare.okundu.is_none() {
        app.kare.tazele();
    }

    panel_karti(app, ui);
    ui.add_space(10.0);

    ui.horizontal_top(|ui| {
        let genislik = (ui.available_width() - 10.0) / 2.0;

        ui.allocate_ui(egui::vec2(genislik, 0.0), |ui| {
            sinir_karti(app, ui);
        });
        ui.allocate_ui(egui::vec2(genislik, 0.0), |ui| {
            ui.vertical(|ui| {
                bosta_karti(app, ui);
                ui.add_space(10.0);
                kim_karti(app, ui);
            });
        });
    });
}

/// Panelin ve surucunun su anki hali + hesaplanan sinir.
fn panel_karti(app: &mut Uygulama, ui: &mut Ui) {
    let politika = CapPolicy {
        max_fps: app.cfg.nvidia.max_fps,
        headroom: app.cfg.nvidia.fps_headroom,
        vrr_margin: app.cfg.nvidia.vrr_margin,
        refresh_hz: app.cfg.nvidia.refresh_hz,
    };
    let etkin_vrr = app.kare.vrr && app.cfg.nvidia.respect_vrr;
    let sinir = politika.resolve(etkin_vrr);
    let kaynak_hz = if app.cfg.nvidia.refresh_hz > 0 {
        app.cfg.nvidia.refresh_hz
    } else {
        nvapi::snap_refresh(app.kare.panel_hz)
    };

    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Sonuc").size(15.0).strong());
                ui.label(
                    RichText::new("bu ayarlarla sürücüye yazılacak sınır")
                        .size(11.5)
                        .color(tema::YAZI_SOLUK),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("panelden tekrar oku").clicked() {
                    app.kare.tazele();
                }
            });
        });
        ui.add_space(12.0);

        ui.horizontal_top(|ui| {
            tema::olcum(
                ui,
                "PANEL TAZELEME",
                &kaynak_hz.to_string(),
                "Hz",
                tema::VURGU,
                -1.0,
            );
            ui.add_space(24.0);
            tema::olcum(
                ui,
                "VRR / G-SYNC",
                if app.kare.vrr { "acik" } else { "kapali" },
                "",
                if app.kare.vrr { tema::IYI } else { tema::YAZI_SOLUK },
                -1.0,
            );
            ui.add_space(24.0);
            tema::olcum(
                ui,
                "YAZILACAK SINIR",
                &if sinir == 0 {
                    "yok".to_string()
                } else {
                    sinir.to_string()
                },
                if sinir == 0 { "" } else { "fps" },
                if sinir == 0 { tema::ORTA } else { tema::IYI },
                -1.0,
            );
        });

        ui.add_space(10.0);
        ui.label(RichText::new(gerekce(app, etkin_vrr, kaynak_hz, sinir)).size(12.0));

        if !app.kare.nvapi_acildi {
            ui.add_space(6.0);
            ui.label(
                RichText::new("NVAPI açılamadı - VRR durumu okunamıyor, kapalı varsayıldı.")
                    .size(11.5)
                    .color(tema::ORTA),
            );
        }
    });
}

/// Sinirin neden o sayi oldugunu duz cumleyle anlatir.
fn gerekce(app: &Uygulama, vrr: bool, hz: u32, sinir: u32) -> String {
    if app.cfg.nvidia.max_fps > 0 {
        return format!(
            "Elle {} fps yazıldı; panel ve VRR yok sayılıyor.",
            app.cfg.nvidia.max_fps
        );
    }
    if hz == 0 || sinir == 0 {
        return "Tazeleme hızı okunamadı, sınır yazılmayacak - uydurma bir sayı hiç sınır \
                koymamaktan kötüdür."
            .to_string();
    }
    if vrr {
        format!(
            "VRR açık: sınır tazelemenin {} kare altına kuruldu ({} - {} = {}). Üstüne çıkan \
             kare değişken tazeleme penceresinden düşer.",
            app.cfg.nvidia.vrr_margin, hz, app.cfg.nvidia.vrr_margin, sinir
        )
    } else {
        format!(
            "VRR kapalı: sınır tazelemenin {} kare üstüne kuruldu ({} + {} = {}). Sürücünün \
             sınırlayıcısı hedefin birkaç kare altında kaldığı için yüksek nişan alınıyor.",
            app.cfg.nvidia.fps_headroom, hz, app.cfg.nvidia.fps_headroom, sinir
        )
    }
}

fn sinir_karti(app: &mut Uygulama, ui: &mut Ui) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(ui, "Sınır kuralları", "sınırın nasıl hesaplanacağı");

        ui.checkbox(
            &mut app.cfg.nvidia.enabled,
            "kare sınırını rogctl yönetsin",
        );
        ui.add_space(8.0);

        let elle = app.cfg.nvidia.max_fps > 0;
        let mut elle_secili = elle;
        if ui
            .checkbox(&mut elle_secili, "sabit bir sınır yaz (panele bakma)")
            .changed()
        {
            app.cfg.nvidia.max_fps = if elle_secili { 141 } else { 0 };
        }

        if app.cfg.nvidia.max_fps > 0 {
            ui.add(
                egui::Slider::new(&mut app.cfg.nvidia.max_fps, 30..=360)
                    .text("sinir")
                    .suffix(" fps"),
            );
        } else {
            ui.add(
                egui::Slider::new(&mut app.cfg.nvidia.vrr_margin, 0..=20)
                    .text("VRR açıksa altına")
                    .suffix(" kare"),
            );
            ui.add(
                egui::Slider::new(&mut app.cfg.nvidia.fps_headroom, 0..=30)
                    .text("VRR kapalıysa üstüne")
                    .suffix(" kare"),
            );
            ui.add_space(6.0);
            ui.checkbox(
                &mut app.cfg.nvidia.respect_vrr,
                "VRR açıksa sınır tazelemenin altına insin",
            );
            ui.label(
                RichText::new(
                    "Kapatırsan VRR açıkken bile üstüne çıkılır: yırtılma olur ama kare kaybı \
                     olmaz. Bazıları bunu tercih ediyor.",
                )
                .size(11.0)
                .color(tema::YAZI_SOLUK),
            );
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        let mut sabit_hz = app.cfg.nvidia.refresh_hz > 0;
        if ui
            .checkbox(&mut sabit_hz, "tazeleme hızını sabitle")
            .changed()
        {
            app.cfg.nvidia.refresh_hz = if sabit_hz {
                let p = nvapi::snap_refresh(app.kare.panel_hz);
                if p > 0 {
                    p
                } else {
                    144
                }
            } else {
                0
            };
        }
        if app.cfg.nvidia.refresh_hz > 0 {
            ui.add(
                egui::Slider::new(&mut app.cfg.nvidia.refresh_hz, 60..=360)
                    .text("sabit tazeleme")
                    .suffix(" Hz"),
            );
        }
        ui.label(
            RichText::new(
                "Sabitlemek, oturum açılışında paneli yanlış okuyup düşük sınır yazma riskini \
                 tamamen kaldırır. Panelin modunu gerçekten değiştirirsen buradan güncelle.",
            )
            .size(11.0)
            .color(tema::YAZI_SOLUK),
        );
    });
}

fn bosta_karti(app: &mut Uygulama, ui: &mut Ui) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(
            ui,
            "Boşta kalan uygulama",
            "küçültülmüş oyunun kare üretmesini kesmek",
        );

        let mut acik = app.cfg.nvidia.idle_max_fps > 0;
        if ui.checkbox(&mut acik, "boşta sınırı uygula").changed() {
            app.cfg.nvidia.idle_max_fps = if acik { 30 } else { 0 };
        }
        if app.cfg.nvidia.idle_max_fps > 0 {
            ui.add(
                egui::Slider::new(&mut app.cfg.nvidia.idle_max_fps, 10..=120)
                    .text("boşta sınırı")
                    .suffix(" fps"),
            );
            ui.add(
                egui::Slider::new(&mut app.cfg.nvidia.idle_timeout_s, 5..=300)
                    .text("bekleme")
                    .suffix(" sn"),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "DİKKAT: sürücünün 'boşta' kararı oyun oynarken de tetiklenebiliyor ve \
                     görüntüyü bu sınıra kilitliyor. Bu makinede kapalı tutulması ölçüldü.",
                )
                .size(11.0)
                .color(tema::KOTU),
            );
        } else {
            ui.label(
                RichText::new("Kapalı - önerilen. Sürücünün boşta kararı güvenilir değil.")
                    .size(11.0)
                    .color(tema::YAZI_SOLUK),
            );
        }
    });
}

fn kim_karti(app: &mut Uygulama, ui: &mut Ui) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(
            ui,
            "Kim sınırlıyor",
            "sürücüdeki tüm profillerin kare sınırı değeri",
        );

        if ui.button("profilleri tara").clicked() {
            match veri::rogctl(&["nv", "kim"]) {
                Ok(c) => app.kare.kim = Some(c),
                Err(e) => {
                    app.kare.kim = None;
                    app.bildir(Bildirim::kotu(format!("Tarama başarısız: {e}")));
                }
            }
        }

        if let Some(k) = &app.kare.kim {
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    ui.label(RichText::new(k).size(11.5).monospace());
                });
        } else {
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Bir uygulama profili temel profili ezer. Oyun 60'ta kilitliyse suçluyu \
                     burada görürsün.",
                )
                .size(11.0)
                .color(tema::YAZI_SOLUK),
            );
        }
    });
}

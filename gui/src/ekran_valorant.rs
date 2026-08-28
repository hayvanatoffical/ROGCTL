//! Valorant ekrani.
//!
//! Oyunun kendi dosyasinda hesap basina dort ayri kare siniri var ve bunlar
//! surucuden gorunmez: makine surucu seviyesinde kusursuz ayarliyken oyun
//! 60 fps'te kilitli kalabilir. Bir hesabi duzeltmek digerlerini duzeltmez,
//! o yuzden burada hepsi ayri ayri listelenir.

use egui::{RichText, Ui};

use rogctl::nvapi::{self, CapPolicy};
use rogctl::valorant::{blocking_processes, Settings};

use crate::tema;
use crate::{Bildirim, Uygulama};

#[derive(Default)]
pub struct ValorantDurumu {
    /// Diskten okunan hesaplar. None = henuz taranmadi.
    pub hesaplar: Option<Result<Vec<Settings>, String>>,
    pub engelleyenler: Vec<String>,
}

impl ValorantDurumu {
    fn tara(&mut self) {
        self.engelleyenler = blocking_processes();
        self.hesaplar = Some(Settings::all().map_err(|e| e.to_string()));
    }
}

pub fn goster(app: &mut Uygulama, ui: &mut Ui) {
    if app.valorant.hesaplar.is_none() {
        app.valorant.tara();
    }

    let hedef = hedef_fps(app);

    ust_kart(app, ui, hedef);
    ui.add_space(10.0);

    let acik = !app.valorant.engelleyenler.is_empty();

    match &app.valorant.hesaplar {
        Some(Err(e)) => {
            tema::kart(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("Ayar dosyası okunamadı: {e}")).color(tema::ORTA));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Valorant kurulu değilse bu normaldir.")
                        .size(11.5)
                        .color(tema::YAZI_SOLUK),
                );
            });
        }
        Some(Ok(hesaplar)) if hesaplar.is_empty() => {
            tema::kart(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Hiç hesap bulunamadı.").color(tema::YAZI_SOLUK));
            });
        }
        Some(Ok(hesaplar)) => {
            // Once cizilecek veriyi topla; boylece dongude app odunc alinmaz.
            let kartlar: Vec<(String, bool, Vec<String>, Vec<(String, String)>)> = hesaplar
                .iter()
                .map(|h| {
                    (
                        h.account.clone(),
                        h.is_uncapped(),
                        h.offenders(hedef),
                        h.caps()
                            .into_iter()
                            .map(|(ad, _anahtar, deger)| {
                                (ad.trim().to_string(), deger.unwrap_or_else(|| "yok".into()))
                            })
                            .collect(),
                    )
                })
                .collect();

            for (ad, serbest, sorunlar, satirlar) in kartlar {
                hesap_karti(ui, &ad, serbest, &sorunlar, &satirlar);
                ui.add_space(8.0);
            }
        }
        None => {}
    }

    if acik {
        ui.add_space(4.0);
        tema::kart(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                tema::rozet(ui, "oyun açık", tema::KOTU);
                ui.label(
                    RichText::new(format!(
                        "Şu an çalışıyor: {}. Valorant çıkarken dosyayı bellekten yeniden \
                         yazar, yani şimdi yapılan değişiklik sessizce silinir.",
                        app.valorant.engelleyenler.join(", ")
                    ))
                    .size(12.0),
                );
            });
        });
    }
}

/// Duzeltmenin hedefleyecegi kare hizi: kare ekranindaki kuralla ayni.
fn hedef_fps(app: &Uygulama) -> u32 {
    let politika = CapPolicy {
        max_fps: app.cfg.nvidia.max_fps,
        headroom: app.cfg.nvidia.fps_headroom,
        vrr_margin: app.cfg.nvidia.vrr_margin,
        refresh_hz: app.cfg.nvidia.refresh_hz,
    };
    let vrr = app.kare.vrr && app.cfg.nvidia.respect_vrr;
    let c = politika.resolve(vrr);
    if c > 0 {
        c
    } else {
        nvapi::snap_refresh(nvapi::refresh_hz()).max(60)
    }
}

fn ust_kart(app: &mut Uygulama, ui: &mut Ui, hedef: u32) {
    let acik = !app.valorant.engelleyenler.is_empty();

    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Tüm hesaplar").size(15.0).strong());
                ui.label(
                    RichText::new(format!(
                        "Sınırlar hesap başınadır. Hedef kare hızı: {hedef} fps"
                    ))
                    .size(11.5)
                    .color(tema::YAZI_SOLUK),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("yeniden tara").clicked() {
                    app.valorant.tara();
                }

                let serbest = ui
                    .add_enabled(!acik, egui::Button::new("hepsini serbest bırak"))
                    .on_disabled_hover_text("Valorant açıkken değişiklik silinir");
                if serbest.clicked() {
                    calistir(app, &["valorant", "hepsi", "serbest"]);
                }

                let duzelt = ui
                    .add_enabled(!acik, egui::Button::new("hepsini hedefe eşitle"))
                    .on_disabled_hover_text("Valorant açıkken değişiklik silinir");
                if duzelt.clicked() {
                    calistir(app, &["valorant", "hepsi", "duzelt"]);
                }
            });
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "'Serbest bırak' oyunun içindeki anahtarları kapatmanın yaninda sürücüdeki \
                 Valorant profiline de sınırsız yazar. 'Hedefe eşitle' bunu geri alir. Her \
                 yazmadan önce .rogctl-yedek yedeği alınır.",
            )
            .size(11.0)
            .color(tema::YAZI_SOLUK),
        );
    });
}

fn hesap_karti(
    ui: &mut Ui,
    ad: &str,
    serbest: bool,
    sorunlar: &[String],
    satirlar: &[(String, String)],
) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(ad).size(13.5).strong().monospace());
            ui.add_space(8.0);
            if serbest {
                tema::rozet(ui, "sinirsiz", tema::IYI);
            } else if sorunlar.is_empty() {
                tema::rozet(ui, "hedefle uyumlu", tema::IYI);
            } else {
                tema::rozet(ui, &format!("{} uyumsuz sınır", sorunlar.len()), tema::KOTU);
            }
        });

        ui.add_space(8.0);
        egui::Grid::new(format!("val_{ad}"))
            .num_columns(2)
            .spacing([18.0, 4.0])
            .show(ui, |ui| {
                for (etiket, deger) in satirlar {
                    ui.label(RichText::new(etiket).size(11.5).color(tema::YAZI_SOLUK));
                    ui.label(RichText::new(deger).size(12.0).monospace());
                    ui.end_row();
                }
            });

        if !sorunlar.is_empty() {
            ui.add_space(6.0);
            for s in sorunlar {
                ui.label(RichText::new(format!("- {s}")).size(11.5).color(tema::ORTA));
            }
        }
    });
}

fn calistir(app: &mut Uygulama, argumanlar: &[&str]) {
    match crate::veri::rogctl(argumanlar) {
        Ok(_) => {
            app.valorant.tara();
            app.bildir(Bildirim::iyi("Valorant ayarları güncellendi."));
        }
        Err(e) => app.bildir(Bildirim::kotu(format!("İşlem başarısız: {e}"))),
    }
}

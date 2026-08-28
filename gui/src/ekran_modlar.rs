//! Modlar ekrani: her is yuku icin fan ve saat zarfi.
//!
//! Armoury Crate burada sadece uc hazir profil verir ve fan egrisini gizler.
//! Bu ekran alti modun her birinin fan araligini, dizini, GPU saat tabanini ve
//! tavanini, sicaklik hedeflerini acikca gosterir - ve grafikte daemon'un
//! BIOS'a gercekten yazacagi sekiz noktayi cizer, tahmini bir egri degil.

use egui::{RichText, Ui};

use rogctl::config::EnvelopeCfg;
use rogctl::control::build_curve;
use rogctl::policy::{Envelope, Mode};

use crate::tema;
use crate::Uygulama;

pub struct ModlarDurumu {
    pub secili: usize,
}

impl Default for ModlarDurumu {
    fn default() -> Self {
        // AAA oyun en cok bakilan mod, oradan basla.
        Self { secili: 4 }
    }
}

fn mod_adi(m: Mode) -> &'static str {
    match m {
        Mode::Idle => "Bosta",
        Mode::Media => "Film",
        Mode::Office => "Ofis",
        Mode::LightGame => "Hafif oyun",
        Mode::AaaGame => "AAA oyun",
        Mode::Render => "Render",
    }
}

fn mod_aciklama(m: Mode) -> &'static str {
    match m {
        Mode::Idle => "masaüstü, iş yok - sessizlik öncelikli",
        Mode::Media => "video oynatma, NVDEC çalışıyor",
        Mode::Office => "tarayıcı, ofis işlerinde orta yük",
        Mode::LightGame => "rekabetçi ve hafif oyunlar",
        Mode::AaaGame => "ağır oyun - kare hızı öncelikli",
        Mode::Render => "sürekli tam yük, sıcaklık kabul edilir",
    }
}

pub fn goster(app: &mut Uygulama, ui: &mut Ui) {
    mod_secici(app, ui);
    ui.add_space(12.0);

    let m = Mode::ALL[app.modlar.secili];
    let anahtar = m.key().to_string();
    let aktif = app
        .durum
        .as_ref()
        .map(|d| d.mod_ad.replace(' ', "_") == anahtar)
        .unwrap_or(false);

    let mevcut = app.cfg.modes.get(&anahtar).copied();
    let Some(mut env) = mevcut else {
        tema::kart(ui, |ui| {
            ui.label(
                RichText::new(format!(
                    "'{anahtar}' modu ayar dosyasında yok. Varsayılanlar kullanılıyor."
                ))
                .color(tema::ORTA),
            );
        });
        return;
    };

    ui.horizontal_top(|ui| {
        let sol = (ui.available_width() - 12.0) * 0.52;
        let sag = ui.available_width() - sol - 12.0;

        ui.allocate_ui(egui::vec2(sol, 0.0), |ui| {
            ui.vertical(|ui| {
                fan_karti(ui, &mut env);
                ui.add_space(10.0);
                saat_karti(ui, &mut env);
            });
        });

        ui.allocate_ui(egui::vec2(sag, 0.0), |ui| {
            ui.vertical(|ui| {
                egri_karti(ui, &env, aktif);
                ui.add_space(10.0);
                sicaklik_karti(ui, &mut env);
            });
        });
    });

    if Some(env) != mevcut {
        app.cfg.modes.insert(anahtar, env);
    }
}

fn mod_secici(app: &mut Uygulama, ui: &mut Ui) {
    let aktif_anahtar = app
        .durum
        .as_ref()
        .map(|d| d.mod_ad.replace(' ', "_"))
        .unwrap_or_default();

    ui.horizontal_wrapped(|ui| {
        for (i, m) in Mode::ALL.into_iter().enumerate() {
            let secili = app.modlar.secili == i;
            let calisiyor = m.key() == aktif_anahtar;

            let arka = if secili { tema::VURGU_KOYU } else { tema::KART };
            let kenar = if secili {
                tema::VURGU
            } else if calisiyor {
                tema::IYI
            } else {
                tema::CIZGI
            };

            let yanit = egui::Frame::none()
                .fill(arka)
                .stroke(egui::Stroke::new(1.0_f32, kenar))
                .rounding(egui::Rounding::same(7.0))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(mod_adi(m)).size(13.0).strong());
                            if calisiyor {
                                ui.label(RichText::new("* aktif").size(10.0).color(tema::IYI));
                            }
                        });
                        ui.label(
                            RichText::new(mod_aciklama(m))
                                .size(10.0)
                                .color(tema::YAZI_SOLUK),
                        );
                    });
                })
                .response
                .interact(egui::Sense::click());

            if yanit.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if yanit.clicked() {
                app.modlar.secili = i;
            }
        }
    });
}

fn fan_karti(ui: &mut Ui, env: &mut EnvelopeCfg) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(
            ui,
            "Fan",
            "dizin altında fan tamamen durur; üstünde tavana doğru hızlanır",
        );
        ui.add(
            egui::Slider::new(&mut env.fan_idle_pct, 0..=100)
                .text("taban %")
                .suffix("%"),
        );
        ui.add(
            egui::Slider::new(&mut env.fan_max_pct, 0..=100)
                .text("tavan %")
                .suffix("%"),
        );
        ui.add(
            egui::Slider::new(&mut env.fan_knee_c, 30..=95)
                .text("dizin")
                .suffix(" C"),
        );
        if env.fan_max_pct < env.fan_idle_pct {
            env.fan_max_pct = env.fan_idle_pct;
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "90 C üstü her modda korunur: sessiz bir profil bile o noktada tam hızlanır.",
            )
            .size(11.0)
            .color(tema::YAZI_SOLUK),
        );
    });
}

fn saat_karti(ui: &mut Ui, env: &mut EnvelopeCfg) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(
            ui,
            "GPU saati",
            "güç ~ V2*f olduğu için saati kısmak voltajı da düşürür",
        );
        ui.add(
            egui::Slider::new(&mut env.gpu_clock_ceiling_mhz, 300..=2100)
                .text("tavan")
                .suffix(" MHz"),
        );
        ui.add(
            egui::Slider::new(&mut env.gpu_clock_floor_mhz, 210..=1200)
                .text("taban")
                .suffix(" MHz"),
        );
        if env.gpu_clock_floor_mhz > env.gpu_clock_ceiling_mhz {
            env.gpu_clock_floor_mhz = env.gpu_clock_ceiling_mhz;
        }
        ui.add_space(4.0);
        ui.checkbox(
            &mut env.demand_scaling,
            "talebe göre tavanı düşür (yük azken saati kıs)",
        );
        ui.label(
            RichText::new(
                "Oyun modlarında kapalı tutulmalı: yükün anlık düşmesi kare hızını düşürmesin.",
            )
            .size(11.0)
            .color(tema::YAZI_SOLUK),
        );
    });
}

fn sicaklik_karti(ui: &mut Ui, env: &mut EnvelopeCfg) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(
            ui,
            "Sıcaklık hedefi",
            "GPU saati bu hedefi tutacak şekilde ayarlanır",
        );
        ui.add(
            egui::Slider::new(&mut env.gpu_temp_target, 50..=90)
                .text("GPU hedefi")
                .suffix(" C"),
        );
        ui.add(
            egui::Slider::new(&mut env.cpu_temp_target, 60..=100)
                .text("CPU hedefi")
                .suffix(" C"),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "CPU hedefi yalnızca fan eğrisini şekillendirir. Bu makinede CPU gücünü \
                 düşürecek bir kol yok - 95 C bir arıza değil.",
            )
            .size(11.0)
            .color(tema::YAZI_SOLUK),
        );
    });
}

fn egri_karti(ui: &mut Ui, env: &EnvelopeCfg, aktif: bool) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Fan eğrisi").size(15.0).strong());
                ui.label(
                    RichText::new("BIOS'a yazılacak sekiz nokta")
                        .size(11.5)
                        .color(tema::YAZI_SOLUK),
                );
            });
            if aktif {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    tema::rozet(ui, "şu an yürürlükte", tema::IYI);
                });
            }
        });
        ui.add_space(10.0);

        let e = Envelope::from(*env);
        let egri = build_curve(&e, env.cpu_temp_target);
        egri_ciz(ui, &egri);
    });
}

/// Sekiz noktayi sicaklik-yuzde duzleminde cizer.
fn egri_ciz(ui: &mut Ui, egri: &[u8; 16]) {
    let en = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(en, 180.0), egui::Sense::hover());
    let p = ui.painter();

    p.rect_filled(
        rect,
        egui::Rounding::same(6.0),
        egui::Color32::from_rgb(0x0E, 0x10, 0x16),
    );

    let x_min = 30.0f32;
    let x_max = 100.0f32;
    let ic = rect.shrink2(egui::vec2(30.0, 18.0));

    let nokta = |t: f32, y: f32| -> egui::Pos2 {
        let fx = ((t - x_min) / (x_max - x_min)).clamp(0.0, 1.0);
        let fy = (y / 100.0).clamp(0.0, 1.0);
        egui::pos2(ic.left() + ic.width() * fx, ic.bottom() - ic.height() * fy)
    };

    // Yatay kilavuz cizgileri ve yuzde etiketleri.
    for y in [0, 25, 50, 75, 100] {
        let a = nokta(x_min, y as f32);
        let b = nokta(x_max, y as f32);
        p.line_segment([a, b], egui::Stroke::new(1.0_f32, tema::CIZGI));
        p.text(
            egui::pos2(rect.left() + 4.0, a.y),
            egui::Align2::LEFT_CENTER,
            format!("{y}"),
            egui::FontId::proportional(9.5),
            tema::YAZI_SOLUK,
        );
    }
    // Dikey kilavuz ve sicaklik etiketleri.
    for t in [40, 60, 80, 100] {
        let a = nokta(t as f32, 0.0);
        p.line_segment(
            [a, nokta(t as f32, 100.0)],
            egui::Stroke::new(1.0_f32, tema::CIZGI.linear_multiply(0.6)),
        );
        p.text(
            egui::pos2(a.x, rect.bottom() - 6.0),
            egui::Align2::CENTER_CENTER,
            format!("{t}C"),
            egui::FontId::proportional(9.5),
            tema::YAZI_SOLUK,
        );
    }

    let noktalar: Vec<egui::Pos2> = (0..8)
        .map(|i| nokta(egri[i] as f32, egri[i + 8] as f32))
        .collect();

    // Egrinin altini hafifce doldur, okumasi kolaylassin.
    let mut dolgu = noktalar.clone();
    dolgu.push(egui::pos2(noktalar[7].x, ic.bottom()));
    dolgu.push(egui::pos2(noktalar[0].x, ic.bottom()));
    p.add(egui::Shape::convex_polygon(
        dolgu,
        tema::VURGU.linear_multiply(0.10),
        egui::Stroke::NONE,
    ));

    p.add(egui::Shape::line(
        noktalar.clone(),
        egui::Stroke::new(2.0_f32, tema::VURGU),
    ));

    for (i, n) in noktalar.iter().enumerate() {
        // Son iki nokta sabit koruma bandi; onlari ayirt et.
        let renk = if i >= 6 { tema::ORTA } else { tema::VURGU };
        p.circle_filled(*n, 3.5, renk);
    }
}

//! Sistem ekrani: daemon, servisler, bellek, pil ve donanim taramasi.
//!
//! Kurulum sihirbazinin sordugu her seyin kalici karsiligi burada durur.
//! Sihirbaz bir kez calisir; bu ekran her zaman acilabilir.

use egui::{RichText, Ui};

use rogctl::acpi::{Acpi, STATUS_SUPPORTED};
use rogctl::devices;
use rogctl::gpu;
use rogctl::nvapi;
use rogctl::policy::Mode;

use crate::tema;
use crate::veri;
use crate::{Bildirim, Uygulama};

pub fn goster(app: &mut Uygulama, ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        let genislik = (ui.available_width() - 10.0) / 2.0;

        ui.allocate_ui(egui::vec2(genislik, 0.0), |ui| {
            ui.vertical(|ui| {
                daemon_karti(app, ui);
                ui.add_space(10.0);
                servis_karti(app, ui);
                ui.add_space(10.0);
                pil_karti(app, ui);
            });
        });

        ui.allocate_ui(egui::vec2(genislik, 0.0), |ui| {
            ui.vertical(|ui| {
                bellek_karti(app, ui);
                ui.add_space(10.0);
                donanim_karti(app, ui);
            });
        });
    });
}

fn daemon_karti(app: &mut Uygulama, ui: &mut Ui) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(ui, "Daemon", "açılışta çalışan zamanlanmış görev");

        ui.horizontal(|ui| {
            match &app.durum {
                Some(d) if d.canli() => tema::rozet(ui, "calisiyor", tema::IYI),
                Some(_) => tema::rozet(ui, "durmus", tema::KOTU),
                None => tema::rozet(ui, "hiç çalışmamış", tema::ORTA),
            }
            if let Some(d) = &app.durum {
                if d.canli() {
                    ui.label(
                        RichText::new(veri::sure_metni(d.calisma_s))
                            .size(11.5)
                            .color(tema::YAZI_SOLUK),
                    );
                }
            }
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("baslat").clicked() {
                islem(app, veri::gorev("Start-ScheduledTask"), "Daemon başlatıldı.");
            }
            if ui.button("durdur").clicked() {
                islem(app, veri::gorev("Stop-ScheduledTask"), "Daemon durduruldu.");
            }
            if ui.button("yeniden başlat").clicked() {
                islem(
                    app,
                    veri::gorev_yeniden_baslat(),
                    "Daemon yeniden başlatıldı.",
                );
            }
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("Ayar dosyası: {}", veri::ayar_yolu().display()))
                .size(10.5)
                .color(tema::YAZI_SOLUK),
        );
    });
}

fn servis_karti(app: &mut Uygulama, ui: &mut Ui) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(
            ui,
            "ASUS servisleri",
            "Armoury Crate fan eğrisini geri yazıyor",
        );

        ui.checkbox(
            &mut app.cfg.suspend_asus_services,
            "rogctl çalışırken ASUS termal servislerini durdur",
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Durdurulmazsa rogctl'in yazdığı eğri birkaç saniye içinde silinir. \
                 Kaldırınca servisler geri başlatılır.",
            )
            .size(11.0)
            .color(tema::YAZI_SOLUK),
        );

        if let Some(d) = &app.durum {
            if d.askidaki_servisler != "-" && !d.askidaki_servisler.is_empty() {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    for s in d.askidaki_servisler.split(',') {
                        tema::rozet(ui, s, tema::IYI);
                    }
                });
            }
        }
    });
}

fn bellek_karti(app: &mut Uygulama, ui: &mut Ui) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(ui, "Bellek", "standby listesi temizliği");

        ui.checkbox(&mut app.cfg.memory.enabled, "bellek temizliği açık");
        ui.add_enabled_ui(app.cfg.memory.enabled, |ui| {
            ui.checkbox(
                &mut app.cfg.memory.on_mode_change,
                "mod değişiminde temizle (oyun açılırken)",
            );
            ui.add(
                egui::Slider::new(&mut app.cfg.memory.min_free_mb, 128..=8192)
                    .text("esik")
                    .suffix(" MB"),
            );
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Yalnızca listenin atılabilir ucu temizlenir. Oyun açılırken yapılan temizlik \
                 takılmayı ölçülebilir şekilde azaltıyor.",
            )
            .size(11.0)
            .color(tema::YAZI_SOLUK),
        );
    });
}

fn pil_karti(app: &mut Uygulama, ui: &mut Ui) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(ui, "Pilde", "fişten çekilince uygulanan ölçekler");

        ui.add(
            egui::Slider::new(&mut app.cfg.battery.fan_max_scale, 0.3..=1.0)
                .text("fan tavanı çarpanı"),
        );
        ui.add(
            egui::Slider::new(&mut app.cfg.battery.clock_ceiling_scale, 0.3..=1.0)
                .text("GPU saat tavanı çarpanı"),
        );

        ui.add_space(8.0);
        let mevcut = app.cfg.battery.cap_mode.clone().unwrap_or_default();
        let gosterim = if mevcut.is_empty() {
            "sınır yok".to_string()
        } else {
            mevcut.clone()
        };
        egui::ComboBox::from_label("mod tavanı")
            .selected_text(gosterim)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(mevcut.is_empty(), "sınır yok")
                    .clicked()
                {
                    app.cfg.battery.cap_mode = None;
                }
                for m in Mode::ALL {
                    if ui
                        .selectable_label(mevcut == m.key(), m.key())
                        .clicked()
                    {
                        app.cfg.battery.cap_mode = Some(m.key().to_string());
                    }
                }
            });

        ui.add_space(6.0);
        ui.label(
            RichText::new("Pilde bu modun üstüne çıkılmaz - en büyük pil ömrü kolu budur.")
                .size(11.0)
                .color(tema::YAZI_SOLUK),
        );
    });
}

fn donanim_karti(app: &mut Uygulama, ui: &mut Ui) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(
            ui,
            "Donanim",
            "hangi kolların bu makinede gerçekten açılabildiği",
        );

        // Tarama pahali degil ama her karede yapilmasi gereksiz; dugmeye bagli.
        if ui.button("donanımı tara").clicked() {
            app.sistem_tarama = Some(tara());
        }

        ui.add_space(8.0);
        match &app.sistem_tarama {
            None => {
                ui.label(
                    RichText::new(
                        "Tarama ATKACPI, NVML ve NVAPI'yi açmayı dener ve BIOS'un kaç aygıtı \
                         gerçekten desteklediğini sayar.",
                    )
                    .size(11.0)
                    .color(tema::YAZI_SOLUK),
                );
            }
            Some(satirlar) => {
                for (ad, sonuc) in satirlar {
                    ui.horizontal(|ui| {
                        match sonuc {
                            Ok(bilgi) => {
                                tema::rozet(ui, "acik", tema::IYI);
                                ui.label(
                                    RichText::new(format!("{ad}: {bilgi}")).size(12.0),
                                );
                            }
                            Err(hata) => {
                                tema::rozet(ui, "yok", tema::KOTU);
                                ui.label(
                                    RichText::new(format!("{ad}: {hata}"))
                                        .size(12.0)
                                        .color(tema::YAZI_SOLUK),
                                );
                            }
                        };
                    });
                }
            }
        }
    });
}

/// Donanim kollarini tek tek dene ve okunur bir ozet uret.
fn tara() -> Vec<(String, Result<String, String>)> {
    let mut out = Vec::new();

    match Acpi::open() {
        Ok(acpi) => {
            let n = devices::KNOWN
                .iter()
                .filter(|d| acpi.read(d.id).is_ok_and(|v| v & STATUS_SUPPORTED != 0))
                .count();
            out.push((
                "ASUS ATKACPI".to_string(),
                Ok(format!("{n} aygıt destekleniyor")),
            ));
        }
        Err(e) => out.push(("ASUS ATKACPI".to_string(), Err(e.to_string()))),
    }

    match gpu::Gpu::open() {
        Ok(_) => out.push((
            "NVIDIA NVML".to_string(),
            Ok("telemetri ve saat kilidi".to_string()),
        )),
        Err(e) => out.push(("NVIDIA NVML".to_string(), Err(e.to_string()))),
    }

    match nvapi::NvApi::open() {
        Ok(nv) => {
            let vrr = if nvapi::vrr_enabled(&nv) {
                "VRR açık"
            } else {
                "VRR kapalı"
            };
            out.push((
                "NVIDIA NVAPI".to_string(),
                Ok(format!("sürücü profili, {vrr}")),
            ));
        }
        Err(e) => out.push(("NVIDIA NVAPI".to_string(), Err(e.to_string()))),
    }

    let hz = nvapi::refresh_hz();
    out.push((
        "Panel".to_string(),
        if hz > 0 {
            Ok(format!("{} Hz", nvapi::snap_refresh(hz)))
        } else {
            Err("tazeleme hızı okunamadı".to_string())
        },
    ));

    out
}

fn islem(app: &mut Uygulama, sonuc: anyhow::Result<String>, basarili: &str) {
    match sonuc {
        Ok(_) => {
            app.durum = veri::Durum::oku();
            app.bildir(Bildirim::iyi(basarili));
        }
        Err(e) => app.bildir(Bildirim::kotu(format!("İşlem başarısız: {e}"))),
    }
}

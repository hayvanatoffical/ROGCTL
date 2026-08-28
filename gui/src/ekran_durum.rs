//! Durum ekrani: makinenin su an ne yaptigi.
//!
//! Burada hicbir sey degistirilemez, sadece okunur. Amaci tek bakista "her sey
//! yolunda mi" sorusuna cevap vermek; ayar yapmak icin digerleri var.

use egui::{RichText, Ui};

use crate::tema;
use crate::veri::{self, Durum};
use crate::Uygulama;

pub fn goster(app: &mut Uygulama, ui: &mut Ui) {
    let Some(d) = app.durum.clone() else {
        yok_uyarisi(ui, "Daemon hiç çalışmamış - durum dosyası yok.");
        return;
    };
    if !d.canli() {
        yok_uyarisi(
            ui,
            &format!(
                "Daemon durmuş. Son güncelleme {} önce.",
                veri::sure_metni(d.yas_s)
            ),
        );
        ui.add_space(10.0);
    }

    olcumler(ui, &d);
    ui.add_space(10.0);

    ui.horizontal_top(|ui| {
        let genislik = (ui.available_width() - 10.0) / 2.0;

        ui.allocate_ui(egui::vec2(genislik, 0.0), |ui| {
            tema::kart(ui, |ui| {
                ui.set_width(ui.available_width());
                tema::baslik(ui, "Şu anki mod", "iş yüküne göre seçildi");
                tema::satir(ui, "mod", deger(&d.mod_ad));
                tema::satir(
                    ui,
                    "oyun profili",
                    deger(if d.oyun == "-" || d.oyun.is_empty() {
                        "yok"
                    } else {
                        &d.oyun
                    }),
                );
                tema::satir(
                    ui,
                    "güç kaynağı",
                    deger(&format!("{} ({}%)", d.kaynak, d.batarya)),
                );
                tema::satir(
                    ui,
                    "çalışma süresi",
                    deger(&veri::sure_metni(d.calisma_s)),
                );
            });
        });

        ui.allocate_ui(egui::vec2(genislik, 0.0), |ui| {
            tema::kart(ui, |ui| {
                ui.set_width(ui.available_width());
                tema::baslik(ui, "Uygulanan zarf", "bu modun sınırları");
                tema::satir(
                    ui,
                    "GPU saat tavanı",
                    RichText::new(format!("{} MHz", d.tavan_mhz))
                        .size(13.0)
                        .strong()
                        .color(tema::VURGU),
                );
                tema::satir(
                    ui,
                    "GPU anlık saat",
                    deger(&format!("{} MHz", d.gpu_saat_mhz)),
                );
                tema::satir(ui, "fan aralığı", deger(&format!("%{}", d.fan_araligi)));
                tema::satir(
                    ui,
                    "sıcaklık hedefi",
                    deger(&format!("CPU {} C / GPU {} C", d.hedef_cpu_c, d.hedef_gpu_c)),
                );
            });
        });
    });

    ui.add_space(10.0);

    ui.horizontal_top(|ui| {
        let genislik = (ui.available_width() - 10.0) / 2.0;

        ui.allocate_ui(egui::vec2(genislik, 0.0), |ui| {
            tema::kart(ui, |ui| {
                ui.set_width(ui.available_width());
                tema::baslik(ui, "Bellek", "standby listesi ve boş RAM");
                let toplam = d.ram_toplam_mb.max(1) as f32;
                tema::satir(
                    ui,
                    "bos",
                    deger(&format!("{} MB", d.ram_bos_mb)),
                );
                tema::olcek(
                    ui,
                    d.ram_bos_mb as f32 / toplam,
                    tema::yuk_rengi(100 - ((d.ram_bos_mb as f32 / toplam) * 100.0) as u32),
                    ui.available_width(),
                );
                ui.add_space(6.0);
                tema::satir(ui, "standby", deger(&format!("{} MB", d.ram_standby_mb)));
                tema::satir(
                    ui,
                    "temizlik",
                    deger(&format!(
                        "{} kez, {} MB geri kazanıldı",
                        d.ram_temizlik, d.ram_kazanilan_mb
                    )),
                );
            });
        });

        ui.allocate_ui(egui::vec2(genislik, 0.0), |ui| {
            tema::kart(ui, |ui| {
                ui.set_width(ui.available_width());
                tema::baslik(ui, "ASUS servisleri", "fan eğrisini geri yazanlar");
                if d.askidaki_servisler == "-" || d.askidaki_servisler.is_empty() {
                    ui.horizontal(|ui| {
                        tema::rozet(ui, "hiçbiri durdurulmadı", tema::ORTA);
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Armoury Crate çalışırken fan eğrisini birkaç saniyede geri yazar.",
                        )
                        .size(11.5)
                        .color(tema::YAZI_SOLUK),
                    );
                } else {
                    for s in d.askidaki_servisler.split(',') {
                        ui.horizontal(|ui| {
                            tema::rozet(ui, s, tema::IYI);
                            ui.label(RichText::new("durduruldu").size(11.5).color(tema::YAZI_SOLUK));
                        });
                    }
                }
            });
        });
    });
}

fn olcumler(ui: &mut Ui, d: &Durum) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_top(|ui| {
            tema::olcum(
                ui,
                "CPU SICAKLIK",
                &d.cpu_c.to_string(),
                "C",
                tema::sicaklik_rengi(d.cpu_c),
                d.cpu_c as f32 / 100.0,
            );
            ayrac(ui);
            tema::olcum(
                ui,
                "CPU YÜK",
                &format!("%{}", d.cpu_util),
                "",
                tema::yuk_rengi(d.cpu_util),
                d.cpu_util as f32 / 100.0,
            );
            ayrac(ui);
            tema::olcum(
                ui,
                "GPU SICAKLIK",
                &d.gpu_c.to_string(),
                "C",
                tema::sicaklik_rengi(d.gpu_c),
                d.gpu_c as f32 / 100.0,
            );
            ayrac(ui);
            tema::olcum(
                ui,
                "GPU YÜK",
                &format!("%{}", d.gpu_util),
                "",
                tema::yuk_rengi(d.gpu_util),
                d.gpu_util as f32 / 100.0,
            );
            ayrac(ui);
            tema::olcum(
                ui,
                "GPU GÜÇ",
                &format!("{:.0}", d.gpu_w),
                "W",
                tema::VURGU,
                d.gpu_w / 125.0,
            );
            ayrac(ui);
            tema::olcum(
                ui,
                "VRAM",
                &format!("{}", d.vram_mb),
                &format!("/ {} MB", d.vram_toplam_mb),
                tema::VURGU,
                d.vram_mb as f32 / d.vram_toplam_mb.max(1) as f32,
            );
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("FANLAR").size(11.5).color(tema::YAZI_SOLUK));
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("CPU {} RPM", d.cpu_fan))
                    .size(14.0)
                    .strong(),
            );
            ui.add_space(14.0);
            ui.label(
                RichText::new(format!("GPU {} RPM", d.gpu_fan))
                    .size(14.0)
                    .strong(),
            );
        });
    });
}

fn ayrac(ui: &mut Ui) {
    ui.add_space(10.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 56.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, tema::CIZGI);
    ui.add_space(10.0);
}

fn deger(s: &str) -> RichText {
    RichText::new(s).size(13.0).strong()
}

fn yok_uyarisi(ui: &mut Ui, metin: &str) {
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            tema::rozet(ui, "uyari", tema::ORTA);
            ui.label(RichText::new(metin).size(13.0));
        });
        ui.add_space(6.0);
        ui.label(
            RichText::new("Sistem sekmesinden daemon'u başlatabilirsin.")
                .size(11.5)
                .color(tema::YAZI_SOLUK),
        );
    });
}

//! Durum ekranı: makine ne yapıyor ve senin ne yapman gerekiyor.
//!
//! Bu ekran önce ham sayıları gösteriyordu. Sorun şu ki ham sayı karar
//! verdirmiyor: 90 derece gören biri makinenin bozulduğunu sanıyor, oysa bu
//! yongada 90 hem yarım saniyelik bir turbo tepesi hem de saatlerdir süren bir
//! plato olabilir - ve ikisinin cevabı taban tabana zıt. Aradaki farkı
//! kullanıcının sayıdan çıkarmasını beklemek, bilgiyi göstermek değil.
//!
//! Bu yüzden sıra tersine çevrildi: önce **hüküm** (yapman gereken şey), sonra
//! sıcaklığın **süreklisi ve tepesi** yan yana, sonra ham ölçümler. Her bulgu
//! kendi kolunun bulunduğu sekmeye götüren bir düğme taşır - "şunu yap" deyip
//! kullanıcıyı aramaya bırakmak da bir tür saklamaktır.

use egui::{RichText, Ui};

use crate::gecmis::{self, Egilim};
use crate::tani::{self, Bulgu, Seviye};
use crate::tema;
use crate::veri::{self, Durum};
use crate::{Sekme, Uygulama};

pub fn goster(app: &mut Uygulama, ui: &mut Ui) {
    let durum = app.durum.clone();
    let kirli = app.degisti_mi();
    let bulgular = tani::topla(durum.as_ref(), &app.cfg, &app.gecmis, kirli);

    hukum(app, ui, &bulgular);
    ui.add_space(10.0);

    let Some(d) = durum else { return };

    sicaklik_karti(app, ui, &d);
    ui.add_space(10.0);

    olcumler(ui, &d);
    ui.add_space(10.0);

    if bulgular.len() > 1 {
        ui.label(
            RichText::new("DİĞER BULGULAR")
                .size(11.0)
                .color(tema::YAZI_SOLUK),
        );
        ui.add_space(6.0);
        for b in bulgular.iter().skip(1) {
            bulgu_karti(app, ui, b);
            ui.add_space(6.0);
        }
        ui.add_space(4.0);
    }

    ayrinti_kartlari(ui, &d, app);
}

/// En ağır bulgu, ekranın en üstünde, tek cümlede.
fn hukum(app: &mut Uygulama, ui: &mut Ui, bulgular: &[Bulgu]) {
    let Some(b) = bulgular.first() else { return };
    let renk = seviye_rengi(b.seviye);

    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            // Renkli dikey şerit: hükmün ağırlığı okumadan önce görünsün.
            let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 46.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, egui::Rounding::same(2.0), renk);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&b.baslik).size(17.0).strong().color(renk));
                ui.add_space(3.0);
                ui.label(
                    RichText::new(&b.aciklama)
                        .size(12.5)
                        .color(tema::YAZI)
                        .line_height(Some(17.0)),
                );
                if !b.ne_yapmali.is_empty() {
                    ui.add_space(7.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("→").size(13.0).color(tema::VURGU));
                        ui.label(
                            RichText::new(&b.ne_yapmali)
                                .size(12.5)
                                .strong()
                                .color(tema::VURGU),
                        );
                        if let Some(s) = b.sekme {
                            if s != Sekme::Durum && ui.small_button("git").clicked() {
                                app.sekme = s;
                            }
                        }
                    });
                }
            });
        });
    });
}

fn bulgu_karti(app: &mut Uygulama, ui: &mut Ui, b: &Bulgu) {
    let renk = seviye_rengi(b.seviye);
    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            tema::rozet(ui, seviye_adi(b.seviye), renk);
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&b.baslik).size(13.5).strong());
                ui.label(
                    RichText::new(&b.aciklama)
                        .size(12.0)
                        .color(tema::YAZI_SOLUK)
                        .line_height(Some(16.0)),
                );
                if !b.ne_yapmali.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&b.ne_yapmali).size(12.0).color(tema::VURGU),
                        );
                        if let Some(s) = b.sekme {
                            if s != Sekme::Durum && ui.small_button("git").clicked() {
                                app.sekme = s;
                            }
                        }
                    });
                }
            });
        });
    });
}

/// Sıcaklık: sürekli, tepe, eğilim ve son beş dakikanın çizgisi.
fn sicaklik_karti(app: &Uygulama, ui: &mut Ui, d: &Durum) {
    let g = &app.gecmis;
    let yeterli = g.sayi() >= 8;
    let surekli = g.ortalama(gecmis::cpu_c);
    let tepe = g.tepe(gecmis::cpu_c);
    let egilim = g.egilim(gecmis::cpu_c);

    tema::kart(ui, |ui| {
        ui.set_width(ui.available_width());
        tema::baslik(
            ui,
            "CPU sıcaklığı",
            "kararı veren sayı ortalamadır, anlık değer değil",
        );

        ui.horizontal_top(|ui| {
            if yeterli {
                tema::olcum(
                    ui,
                    "SÜREKLİ (1 dk)",
                    &surekli.to_string(),
                    "C",
                    tema::sicaklik_rengi(surekli),
                    surekli as f32 / 100.0,
                );
                ayrac(ui);
                tema::olcum(
                    ui,
                    "TEPE",
                    &tepe.to_string(),
                    "C",
                    tema::sicaklik_rengi(tepe),
                    tepe as f32 / 100.0,
                );
            } else {
                tema::olcum(
                    ui,
                    "ÖLÇÜLÜYOR",
                    &d.cpu_c.to_string(),
                    "C",
                    tema::sicaklik_rengi(d.cpu_c),
                    d.cpu_c as f32 / 100.0,
                );
            }
            ayrac(ui);
            tema::olcum(
                ui,
                "ŞU AN",
                &d.cpu_c.to_string(),
                "C",
                tema::YAZI_SOLUK,
                -1.0,
            );
            ayrac(ui);
            tema::olcum(
                ui,
                "HEDEF",
                &d.hedef_cpu_c.to_string(),
                "C",
                tema::YAZI_SOLUK,
                -1.0,
            );
            ayrac(ui);
            ui.vertical(|ui| {
                ui.label(RichText::new("EĞİLİM").size(11.5).color(tema::YAZI_SOLUK));
                ui.label(
                    RichText::new(egilim.metin())
                        .size(17.0)
                        .strong()
                        .color(match egilim {
                            Egilim::Yukseliyor => tema::ORTA,
                            Egilim::Dusuyor => tema::IYI,
                            _ => tema::YAZI,
                        }),
                );
            });
        });

        ui.add_space(10.0);
        // Sabit 40-100 ekseni: otomatik ölçekte 60 ile 63 arası bile uçurum
        // gibi görünüyor ve grafik yanlış bir alarm kaynağına dönüşüyor.
        tema::grafik(
            ui,
            &g.dizi(gecmis::cpu_c),
            40.0,
            100.0,
            tema::sicaklik_rengi(if yeterli { surekli } else { d.cpu_c }),
            56.0,
            d.hedef_cpu_c as f32,
        );
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("son {} sn  ·  40-100 C  ·  kesik çizgi: hedef", g.sayi()))
                    .size(10.5)
                    .color(tema::YAZI_SOLUK),
            );
        });
    });
}

fn ayrinti_kartlari(ui: &mut Ui, d: &Durum, app: &Uygulama) {
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
                    deger(&format!("{} (%{})", d.kaynak, d.batarya)),
                );
                tema::satir(ui, "çalışma süresi", deger(&veri::sure_metni(d.calisma_s)));
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
                tema::satir(ui, "GPU anlık saat", deger(&format!("{} MHz", d.gpu_saat_mhz)));

                // "fan aralığı %15-75" satırı, fanın tavana kilitli olduğunu
                // düşündürüyordu. Asıl merak edilen, eğrinin **bu sıcaklıkta**
                // ne istediği; aralık onun yanında ikincil bilgi.
                let istenen = tani::mod_zarfi(&app.cfg, &d.mod_ad)
                    .map(|e| tani::egrinin_istedigi(&e, d.cpu_c));
                tema::satir(
                    ui,
                    "eğri şu an istiyor",
                    match istenen {
                        Some(p) => RichText::new(format!("%{p}"))
                            .size(13.0)
                            .strong()
                            .color(tema::VURGU),
                        None => deger("-"),
                    },
                );
                tema::satir(
                    ui,
                    "fan izni",
                    deger(&format!("%{} arası", d.fan_araligi)),
                );
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
                tema::satir(ui, "boş", deger(&format!("{} MB", d.ram_bos_mb)));
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
            ui.add_space(14.0);
            ui.label(
                RichText::new("stand mekaniktir, buradan görünmez")
                    .size(10.5)
                    .color(tema::YAZI_SOLUK),
            );
        });
    });
}

fn seviye_rengi(s: Seviye) -> egui::Color32 {
    match s {
        Seviye::Sorun => tema::KOTU,
        Seviye::Uyari => tema::ORTA,
        Seviye::Bilgi => tema::YAZI_SOLUK,
        Seviye::Iyi => tema::IYI,
    }
}

fn seviye_adi(s: Seviye) -> &'static str {
    match s {
        Seviye::Sorun => "sorun",
        Seviye::Uyari => "dikkat",
        Seviye::Bilgi => "bilgi",
        Seviye::Iyi => "iyi",
    }
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

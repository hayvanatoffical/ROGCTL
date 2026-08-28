//! Arayuzun gorsel dili: renkler, araliklar, kart cercevesi.
//!
//! Tek yerde durmasinin sebebi tutarlilik. Her ekran ayni kart cercevesini,
//! ayni baslik boyutunu ve ayni sicaklik renk skalasini kullanir; boylece
//! kullanici bir ekranda ogrendigini digerinde tekrar ogrenmek zorunda kalmaz.

use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui};

pub const ZEMIN: Color32 = Color32::from_rgb(0x10, 0x12, 0x18);
pub const KART: Color32 = Color32::from_rgb(0x19, 0x1C, 0x25);
pub const KART_UST: Color32 = Color32::from_rgb(0x1F, 0x23, 0x2E);
pub const CIZGI: Color32 = Color32::from_rgb(0x2A, 0x2F, 0x3D);

pub const YAZI: Color32 = Color32::from_rgb(0xE6, 0xE9, 0xF0);
pub const YAZI_SOLUK: Color32 = Color32::from_rgb(0x8B, 0x93, 0xA6);

pub const VURGU: Color32 = Color32::from_rgb(0x35, 0xC9, 0xB0);
pub const VURGU_KOYU: Color32 = Color32::from_rgb(0x1F, 0x5E, 0x54);

pub const IYI: Color32 = Color32::from_rgb(0x4A, 0xD6, 0x8E);
pub const ORTA: Color32 = Color32::from_rgb(0xE8, 0xB3, 0x39);
pub const KOTU: Color32 = Color32::from_rgb(0xE8, 0x5D, 0x4E);

/// Sicakligi renge cevirir. Esikler bu makinede olculdu: 95 C bir ariza degil,
/// bu yuzden kirmizi 88'den once baslamaz - yoksa arayuz surekli alarm verir.
pub fn sicaklik_rengi(c: u32) -> Color32 {
    match c {
        0..=69 => IYI,
        70..=87 => ORTA,
        _ => KOTU,
    }
}

/// Yuzdelik bir yuk degerini renge cevirir.
pub fn yuk_rengi(p: u32) -> Color32 {
    match p {
        0..=59 => IYI,
        60..=84 => ORTA,
        _ => KOTU,
    }
}

pub fn uygula(ctx: &egui::Context) {
    let mut stil = (*ctx.style()).clone();

    stil.visuals.dark_mode = true;
    stil.visuals.panel_fill = ZEMIN;
    stil.visuals.window_fill = ZEMIN;
    stil.visuals.extreme_bg_color = Color32::from_rgb(0x0B, 0x0D, 0x12);
    stil.visuals.override_text_color = Some(YAZI);
    stil.visuals.hyperlink_color = VURGU;

    let w = &mut stil.visuals.widgets;

    w.noninteractive.bg_fill = KART;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, CIZGI);
    w.noninteractive.rounding = Rounding::same(6.0);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, YAZI);

    w.inactive.bg_fill = KART_UST;
    w.inactive.weak_bg_fill = KART_UST;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, CIZGI);
    w.inactive.rounding = Rounding::same(6.0);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, YAZI);

    w.hovered.bg_fill = Color32::from_rgb(0x2A, 0x30, 0x3E);
    w.hovered.weak_bg_fill = Color32::from_rgb(0x2A, 0x30, 0x3E);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, VURGU);
    w.hovered.rounding = Rounding::same(6.0);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, YAZI);

    w.active.bg_fill = VURGU_KOYU;
    w.active.weak_bg_fill = VURGU_KOYU;
    w.active.bg_stroke = Stroke::new(1.0_f32, VURGU);
    w.active.rounding = Rounding::same(6.0);
    w.active.fg_stroke = Stroke::new(1.0_f32, YAZI);

    w.open.bg_fill = KART_UST;
    w.open.weak_bg_fill = KART_UST;
    w.open.bg_stroke = Stroke::new(1.0_f32, CIZGI);
    w.open.rounding = Rounding::same(6.0);

    stil.visuals.selection.bg_fill = VURGU_KOYU;
    stil.visuals.selection.stroke = Stroke::new(1.0_f32, VURGU);

    stil.spacing.item_spacing = egui::vec2(10.0, 8.0);
    stil.spacing.button_padding = egui::vec2(12.0, 7.0);
    stil.spacing.slider_width = 200.0;
    stil.spacing.interact_size.y = 26.0;

    use egui::{FontFamily::Proportional, TextStyle};
    stil.text_styles = [
        (TextStyle::Heading, egui::FontId::new(19.0, Proportional)),
        (TextStyle::Body, egui::FontId::new(13.5, Proportional)),
        (TextStyle::Button, egui::FontId::new(13.5, Proportional)),
        (TextStyle::Small, egui::FontId::new(11.5, Proportional)),
        (
            TextStyle::Monospace,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        ),
    ]
    .into();

    ctx.set_style(stil);
}

/// Standart kart. Her panel bunun icinde durur.
pub fn kart<R>(ui: &mut Ui, ekle: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::none()
        .fill(KART)
        .stroke(Stroke::new(1.0_f32, CIZGI))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(14.0))
        .show(ui, ekle)
        .inner
}

/// Kart basligi: ustte ad, altinda ince aciklama.
pub fn baslik(ui: &mut Ui, ad: &str, aciklama: &str) {
    ui.label(RichText::new(ad).size(15.0).strong().color(YAZI));
    if !aciklama.is_empty() {
        ui.label(RichText::new(aciklama).size(11.5).color(YAZI_SOLUK));
    }
    ui.add_space(8.0);
}

/// "Etiket ....... deger" satiri. Deger saga yaslanir, sutun hizasi bozulmaz.
pub fn satir(ui: &mut Ui, etiket: &str, deger: RichText) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(etiket).size(12.5).color(YAZI_SOLUK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(deger);
        });
    });
}

/// Kucuk durum rozeti.
pub fn rozet(ui: &mut Ui, metin: &str, renk: Color32) {
    Frame::none()
        .fill(renk.linear_multiply(0.20))
        .stroke(Stroke::new(1.0_f32, renk.linear_multiply(0.55)))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(9.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(metin).size(11.5).color(renk).strong());
        });
}

/// Yatay dolum cubugu: sicaklik, yuk ve VRAM icin ayni gorsel dil.
pub fn olcek(ui: &mut Ui, oran: f32, renk: Color32, genislik: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(genislik, 6.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(3.0), Color32::from_rgb(0x24, 0x28, 0x33));
    let dolu = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width() * oran.clamp(0.0, 1.0), rect.height()),
    );
    p.rect_filled(dolu, Rounding::same(3.0), renk);
}

/// Buyuk sayi + birim + altinda etiket. Panolarin temel tasi.
pub fn olcum(ui: &mut Ui, etiket: &str, deger: &str, birim: &str, renk: Color32, oran: f32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(etiket).size(11.5).color(YAZI_SOLUK));
        ui.horizontal(|ui| {
            ui.label(RichText::new(deger).size(26.0).strong().color(renk));
            if !birim.is_empty() {
                ui.label(RichText::new(birim).size(12.0).color(YAZI_SOLUK));
            }
        });
        if oran >= 0.0 {
            olcek(ui, oran, renk, 132.0);
        }
    });
}

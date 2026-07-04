//! Yandex Music dark theme colors (`ym-dark-theme`) and proprietary fonts
//! (YS Text, YSMusic Headline).
//!
//! Color values and font files are extracted from the unpacked desktop client
//! (`app/app/_next/static/css/*.css` and `music.yandex.ru/fonts/*`).
//!
//! Several palette constants are declared for completeness even if not currently used.
#![allow(dead_code)]

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Visuals};

/// Font family name for headings (YSMusic Headline).
pub const HEADING_FAMILY: &str = "YSMusicHeadline";

pub const BG_BASIC: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
pub const BG_CONTENT: Color32 = Color32::from_rgb(0x12, 0x12, 0x12);
pub const BG_POPOVER: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);
pub const BG_PLAYER: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);

pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0xff, 0x00);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0xda, 0xda, 0x0b);
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0xff, 0xff, 0x80);
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);

pub const SECONDARY_BG: Color32 = Color32::from_rgba_premultiplied(0x14, 0x14, 0x14, 0xff);
pub const SECONDARY_BG_HOVER: Color32 = Color32::from_rgb(0x24, 0x24, 0x24);
pub const SECONDARY_BG_ACTIVE: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe6, 0xe6, 0xe6);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);

pub const SUCCESS: Color32 = Color32::from_rgb(0x37, 0xc8, 0x4f);
pub const ERROR: Color32 = Color32::from_rgb(0xfb, 0x29, 0x04);
pub const WARNING: Color32 = Color32::from_rgb(0xd5, 0x59, 0x4d);

pub const OUTLINE: Color32 = Color32::from_rgb(0x4d, 0x4d, 0x4d);

/// Applies the Yandex Music fonts and dark color palette to the egui context.
pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_visuals(visuals());
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "YSText-Regular".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/YSText-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "YSText-Medium".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/YSText-Medium.ttf")).into(),
    );
    fonts.font_data.insert(
        "YSMusic-HeadlineBold".to_owned(),
        FontData::from_static(include_bytes!(
            "../../assets/fonts/YSMusic-HeadlineBold.ttf"
        ))
        .into(),
    );

    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "YSText-Medium".to_owned());
    proportional.insert(0, "YSText-Regular".to_owned());

    fonts.families.insert(
        FontFamily::Name(HEADING_FAMILY.into()),
        vec!["YSMusic-HeadlineBold".to_owned()],
    );

    ctx.set_fonts(fonts);
}

fn visuals() -> Visuals {
    let mut v = Visuals::dark();

    v.dark_mode = true;
    v.override_text_color = Some(TEXT_PRIMARY);
    v.panel_fill = BG_CONTENT;
    v.window_fill = BG_POPOVER;
    v.extreme_bg_color = BG_BASIC;
    v.faint_bg_color = BG_PLAYER;
    v.hyperlink_color = ACCENT;
    v.warn_fg_color = WARNING;
    v.error_fg_color = ERROR;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    let rounding = CornerRadius::same(8);

    v.widgets.inactive.bg_fill = SECONDARY_BG;
    v.widgets.inactive.weak_bg_fill = SECONDARY_BG;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.inactive.corner_radius = rounding;

    v.widgets.hovered.bg_fill = SECONDARY_BG_HOVER;
    v.widgets.hovered.weak_bg_fill = SECONDARY_BG_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, OUTLINE);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    v.widgets.hovered.corner_radius = rounding;

    v.widgets.active.bg_fill = SECONDARY_BG_ACTIVE;
    v.widgets.active.weak_bg_fill = SECONDARY_BG_ACTIVE;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    v.widgets.active.corner_radius = rounding;

    v.widgets.open.bg_fill = BG_POPOVER;
    v.widgets.open.weak_bg_fill = SECONDARY_BG_HOVER;
    v.widgets.open.bg_stroke = Stroke::new(1.0, OUTLINE);
    v.widgets.open.corner_radius = rounding;

    v.widgets.noninteractive.bg_fill = BG_CONTENT;
    v.widgets.noninteractive.weak_bg_fill = BG_CONTENT;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x24, 0x24, 0x24));
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);

    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(8);

    v
}

/// Returns a `RichText` rendered in the YSMusic Headline font family.
#[must_use]
pub fn heading(text: impl Into<String>, size: f32) -> egui::RichText {
    egui::RichText::new(text.into())
        .font(egui::FontId::new(
            size,
            FontFamily::Name(HEADING_FAMILY.into()),
        ))
        .color(TEXT_PRIMARY)
}

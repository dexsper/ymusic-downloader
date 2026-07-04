//! Shared UI widgets: window control buttons rendered as a global overlay.

use egui_material_icons::icons::{
    ICON_CLOSE, ICON_CROP_SQUARE, ICON_FILTER_NONE, ICON_HORIZONTAL_RULE,
};

use crate::app::theme;

/// Height of the custom title bar in pixels.
pub const TITLE_H: f32 = 30.0;
/// Width of each window control button in pixels.
pub const BTN_W: f32 = 46.0;
/// Width of the left sidebar (logo + profile island column).
pub const SIDEBAR_W: f32 = 210.0;

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn ease_out_quart(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(4)
}

/// Draws the three window control buttons (minimize / maximize / close) as a
/// floating `Foreground` area pinned to the top-right corner of the viewport.
///
/// Call this once per frame from the root `ui()` so the buttons appear on
/// every screen without each screen needing to know about them.
pub fn show_win_controls(ctx: &egui::Context) {
    let screen = ctx.input(|i| i.viewport_rect());
    let pos = egui::pos2(screen.max.x - BTN_W * 3.0, screen.min.y);

    egui::Area::new(egui::Id::new("win_controls"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            ui.set_width(BTN_W * 3.0);
            ui.set_height(TITLE_H);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));

                win_btn(
                    ui,
                    ICON_CLOSE,
                    egui::Color32::from_rgb(0xc4, 0x2b, 0x1c),
                    |ctx| ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                );

                win_btn(
                    ui,
                    if maximized {
                        ICON_FILTER_NONE
                    } else {
                        ICON_CROP_SQUARE
                    },
                    egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 0x22),
                    move |ctx| ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized)),
                );

                win_btn(
                    ui,
                    ICON_HORIZONTAL_RULE,
                    egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 0x22),
                    |ctx| ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                );
            });
        });
}

/// Single window control button: transparent background, coloured fill on hover.
pub fn win_btn(
    ui: &mut egui::Ui,
    icon: egui_material_icons::MaterialIcon,
    hover_fill: egui::Color32,
    on_click: impl FnOnce(&egui::Context),
) {
    let size = egui::vec2(BTN_W, TITLE_H);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, hover_fill);
    }

    let color = if response.hovered() {
        egui::Color32::WHITE
    } else {
        theme::TEXT_MUTED
    };

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.codepoint,
        egui::FontId::new(16.0, icon.font_family()),
        color,
    );

    if response.clicked() {
        on_click(ui.ctx());
    }
}

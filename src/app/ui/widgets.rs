//! Shared UI widgets: window control buttons and the unified logo renderer.

use egui_material_icons::icons::{
    ICON_CLOSE, ICON_CROP_SQUARE, ICON_FILTER_NONE, ICON_HORIZONTAL_RULE,
};

use crate::app::{LogoTarget, YmdApp, theme};

/// Vertical distance from title-bar bottom to logo centre on auth/project screens.
const AUTH_LOGO_CY_OFFSET: f32 = 72.0;
/// Horizontal padding from sidebar edge for the sidebar logo.
const SIDEBAR_LOGO_PAD: f32 = 20.0;
/// Vertical padding from title-bar bottom to sidebar logo top edge.
const SIDEBAR_LOGO_TOP_PAD: f32 = 30.0;

/// Height of the custom title bar in pixels.
pub const TITLE_H: f32 = 30.0;
/// Width of each window control button in pixels.
pub const BTN_W: f32 = 46.0;
/// Width of the left sidebar (logo + profile island column).
pub const SIDEBAR_W: f32 = 210.0;

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}


/// Draws the logo using the app's unified [`crate::app::LogoAnim`] state.
///
/// Pass `layer` to control Z-ordering; use [`egui::Order::Middle`] for most screens
/// so the logo sits below foreground overlays and islands.
pub fn draw_logo(ctx: &egui::Context, app: &YmdApp, layer: egui::LayerId) {
    let Some(tex) = &app.logo_texture else { return };
    let tex_size = tex.size_vec2();
    let screen = ctx.input(|i| i.viewport_rect());

    let te = app.logo_anim.ease();

    let (from_center, from_display) = logo_abs_pos(app.logo_anim.from, screen, tex_size);
    let (to_center, to_display) = logo_abs_pos(app.logo_anim.to, screen, tex_size);

    let center = egui::pos2(
        lerp(from_center.x, to_center.x, te),
        lerp(from_center.y, to_center.y, te),
    );
    let display = egui::vec2(
        lerp(from_display.x, to_display.x, te),
        lerp(from_display.y, to_display.y, te),
    );

    ctx.layer_painter(layer).image(
        tex.id(),
        egui::Rect::from_center_size(center, display),
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );

    if !app.logo_anim.is_done() {
        ctx.request_repaint();
    }
}

/// Computes the absolute (viewport-space) center and size for a [`LogoTarget`].
fn logo_abs_pos(
    target: LogoTarget,
    screen: egui::Rect,
    tex_size: egui::Vec2,
) -> (egui::Pos2, egui::Vec2) {
    match target {
        LogoTarget::Splash => {
            let w = 280.0f32;
            let display = tex_size * (w / tex_size.x);
            (screen.center(), display)
        }
        LogoTarget::AuthCenter => {
            let w = 240.0f32;
            let display = tex_size * (w / tex_size.x);
            let cy = TITLE_H + AUTH_LOGO_CY_OFFSET;
            (egui::pos2(screen.center().x, cy), display)
        }
        LogoTarget::Sidebar => {
            let w = SIDEBAR_W - SIDEBAR_LOGO_PAD * 2.0;
            let display = tex_size * (w / tex_size.x);
            let cx = SIDEBAR_W / 2.0;
            let cy = TITLE_H + SIDEBAR_LOGO_TOP_PAD + display.y * 0.5;
            (egui::pos2(cx, cy), display)
        }
    }
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

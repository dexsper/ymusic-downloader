//! Main screen: sidebar (logo + profile island) and content area.

use egui_material_icons::icons::ICON_CLOSE;

use crate::app::theme;
use crate::app::ui::widgets::{self, SIDEBAR_W, TITLE_H};
use crate::app::YmdApp;

/// Logo width on the splash screen (start of animation). Must match `splash::LOGO_MAX_WIDTH`.
const SPLASH_LOGO_W: f32 = 280.0;
/// Logo width once settled in the sidebar.
const SIDEBAR_LOGO_W: f32 = SIDEBAR_W - 40.0;
/// Padding between the bottom of the title bar and the top of the logo.
const LOGO_TOP_PAD: f32 = 30.0;
/// Animation duration for the logo fly-in to the sidebar.
const ANIM_SECS: f32 = 0.9;

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    egui::Panel::left("sidebar")
        .exact_size(SIDEBAR_W)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            egui::Frame::new()
                .fill(theme::BG_CONTENT)
                .inner_margin(egui::Margin::ZERO),
        )
        .show(ui, |_ui| {
            // Content is drawn as foreground layers (logo, island) to allow
            // the logo to animate across panel boundaries from the splash position.
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(theme::BG_CONTENT)
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ui, |ui| {
            super::queue::show(ui, app);
        });

    // Logo: animates from splash centre to the top of the sidebar.
    if let Some(tex) = app.logo_texture.as_ref().map(|t| t.id()) {
        let tex_size = app.logo_texture.as_ref().unwrap().size_vec2();

        let t = (app.main_started.elapsed().as_secs_f32() / ANIM_SECS).min(1.0);
        let te = widgets::ease_out_quart(t);

        let screen = ui.ctx().input(|i| i.viewport_rect());

        let logo_w = widgets::lerp(SPLASH_LOGO_W, SIDEBAR_LOGO_W, te);
        let scale = logo_w / tex_size.x;
        let display = tex_size * scale;

        let target_cx = SIDEBAR_W / 2.0;
        let target_cy = TITLE_H + LOGO_TOP_PAD + display.y * 0.5;

        let logo_cx = widgets::lerp(screen.center().x, target_cx, te);
        let logo_cy = widgets::lerp(screen.center().y, target_cy, te);

        ui.ctx()
            .layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("sidebar_logo"),
            ))
            .image(
                tex,
                egui::Rect::from_center_size(egui::pos2(logo_cx, logo_cy), display),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );

        if t < 1.0 {
            ui.ctx().request_repaint();
        }
    }

    // Settings overlay
    if app.show_settings {
        let screen = ui.ctx().input(|i| i.viewport_rect());
        let win_w = (screen.width() - 80.0).min(520.0);
        let win_h = screen.height() - TITLE_H - 40.0;
        let win_pos = egui::pos2(screen.center().x - win_w * 0.5, TITLE_H + 20.0);

        ui.painter()
            .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(140));

        egui::Area::new(egui::Id::new("settings_overlay"))
            .fixed_pos(win_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(theme::BG_CONTENT)
                    .corner_radius(12.0)
                    .stroke(egui::Stroke::new(1.0, theme::OUTLINE))
                    .show(ui, |ui| {
                        ui.set_width(win_w);
                        ui.set_height(win_h);

                        egui::Frame::new()
                            .inner_margin(egui::Margin {
                                left: 16,
                                right: 16,
                                top: 10,
                                bottom: 4,
                            })
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(theme::heading("Настройки", 20.0));
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let close_resp = ui.add(
                                                egui::Button::new(
                                                    egui::RichText::new(ICON_CLOSE.codepoint)
                                                        .font(egui::FontId::new(
                                                            18.0,
                                                            ICON_CLOSE.font_family(),
                                                        ))
                                                        .color(theme::TEXT_MUTED),
                                                )
                                                .fill(egui::Color32::TRANSPARENT)
                                                .frame(false),
                                            );
                                            if close_resp.clicked() {
                                                app.show_settings = false;
                                            }
                                            if close_resp.hovered() {
                                                ui.ctx().set_cursor_icon(
                                                    egui::CursorIcon::PointingHand,
                                                );
                                            }
                                        },
                                    );
                                });
                            });

                        ui.separator();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Frame::new()
                                .inner_margin(egui::Margin::symmetric(16, 0))
                                .show(ui, |ui| {
                                    super::settings::show(ui, app);
                                });
                        });
                    });
            });

        // area_rect uses the previous-frame rect; skip on the very first frame.
        if ui.ctx().input(|i| i.pointer.any_click()) {
            if let Some(click_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                let overlay_rect = ui
                    .ctx()
                    .memory(|m| m.area_rect(egui::Id::new("settings_overlay")))
                    .unwrap_or(egui::Rect::NOTHING);
                if !overlay_rect.contains(click_pos) {
                    app.show_settings = false;
                }
            }
        }
    }

    super::auth::show_island(ui.ctx(), app);
}

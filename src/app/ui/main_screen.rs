//! Main screen: custom title bar, tab navigation, content area, and floating account island.

use crate::app::theme;
use crate::app::{Tab, YmdApp};

const TITLE_H: f32 = 40.0;
const BTN_W: f32 = 46.0;

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    egui::Panel::top("title_bar")
        .exact_size(TITLE_H)
        .frame(
            egui::Frame::new()
                .fill(theme::BG_BASIC)
                .inner_margin(egui::Margin::ZERO),
        )
        .show(ui, |ui| {
            let rect = ui.max_rect();

            let drag_rect = egui::Rect::from_min_max(
                rect.min,
                egui::pos2(rect.max.x - BTN_W * 3.0, rect.max.y),
            );
            let drag = ui.interact(
                drag_rect,
                egui::Id::new("win_drag"),
                egui::Sense::click_and_drag(),
            );
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }

            ui.horizontal(|ui| {
                ui.set_height(TITLE_H);
                ui.add_space(12.0);
                ui.label(theme::heading("Я.Музыка Downloader", 17.0));
                ui.add_space(16.0);
                tab_button(ui, app, Tab::Queue, "Загрузки");
                tab_button(ui, app, Tab::Settings, "Настройки");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    win_btn(
                        ui,
                        WinIcon::Close,
                        egui::Color32::from_rgb(0xc4, 0x2b, 0x1c),
                        |ctx| {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        },
                    );
                    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    let restore_icon = if maximized {
                        WinIcon::Restore
                    } else {
                        WinIcon::Maximize
                    };
                    win_btn(
                        ui,
                        restore_icon,
                        egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 0x22),
                        |ctx| {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        },
                    );
                    win_btn(
                        ui,
                        WinIcon::Minimize,
                        egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 0x22),
                        |ctx| {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        },
                    );
                });
            });
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(theme::BG_CONTENT)
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ui, |ui| match app.tab {
            Tab::Queue => super::queue::show(ui, app),
            Tab::Settings => super::settings::show(ui, app),
        });

    super::auth::show_island(ui.ctx(), app);
}

fn tab_button(ui: &mut egui::Ui, app: &mut YmdApp, tab: Tab, label: &str) {
    let selected = app.tab == tab;
    let color = if selected {
        theme::ACCENT
    } else {
        theme::TEXT_MUTED
    };
    let text = egui::RichText::new(label).color(color).size(15.0);
    let text = if selected { text.strong() } else { text };
    if ui
        .add(
            egui::Button::new(text)
                .fill(egui::Color32::TRANSPARENT)
                .frame(false),
        )
        .clicked()
    {
        app.tab = tab;
    }
}

/// Window control button icons.
///
/// Symbols such as `✕`, `□`, `─` are rendered by the font, but the application's custom fonts
/// (YS Text / YSMusic Headline) lack glyphs for them — Skia/egui substitutes a blank placeholder.
/// Icons are therefore drawn manually using lines and rectangles.
#[derive(Clone, Copy)]
enum WinIcon {
    Close,
    Minimize,
    Maximize,
    Restore,
}

/// Window control button: transparent background, `hover_fill` on hover.
fn win_btn(
    ui: &mut egui::Ui,
    icon: WinIcon,
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
    draw_win_icon(ui.painter(), rect.center(), icon, color);
    if response.clicked() {
        on_click(ui.ctx());
    }
}

/// Draws a Windows 11-style window control icon (thin lines, ~10 px).
fn draw_win_icon(painter: &egui::Painter, center: egui::Pos2, icon: WinIcon, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.0, color);
    let s = 4.5;

    match icon {
        WinIcon::Close => {
            painter.line_segment(
                [center + egui::vec2(-s, -s), center + egui::vec2(s, s)],
                stroke,
            );
            painter.line_segment(
                [center + egui::vec2(-s, s), center + egui::vec2(s, -s)],
                stroke,
            );
        }
        WinIcon::Minimize => {
            painter.line_segment(
                [center + egui::vec2(-s, 0.0), center + egui::vec2(s, 0.0)],
                stroke,
            );
        }
        WinIcon::Maximize => {
            let square = egui::Rect::from_center_size(center, egui::vec2(s * 1.8, s * 1.8));
            painter.rect_stroke(square, 0.0, stroke, egui::StrokeKind::Inside);
        }
        WinIcon::Restore => {
            let side = s * 1.5;
            let offset = s * 0.7;
            let back = egui::Rect::from_min_size(
                center + egui::vec2(-s + offset, -s),
                egui::vec2(side, side),
            );
            let front = egui::Rect::from_min_size(
                center + egui::vec2(-s, -s + offset),
                egui::vec2(side, side),
            );
            painter.line_segment([back.left_top(), back.right_top()], stroke);
            painter.line_segment([back.right_top(), back.right_bottom()], stroke);
            painter.line_segment(
                [back.left_top(), egui::pos2(back.left(), front.top())],
                stroke,
            );
            painter.rect_stroke(front, 0.0, stroke, egui::StrokeKind::Inside);
        }
    }
}

//! Auth screen: shown when the user is not signed in.
//!
//! The logo animates from the centre (splash position) to the upper area of the
//! screen, then a login prompt fades in below it.

use std::time::Duration;

use crate::app::ui::auth::{self, AuthStatus};
use crate::app::{Screen, YmdApp, theme};

/// Duration of the logo fly-up animation.
const ANIM_SECS: f32 = 0.55;
/// Logo width at animation start (matches splash screen size).
const LOGO_W_START: f32 = 280.0;
/// Logo width at animation end.
const LOGO_W_END: f32 = 240.0;

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    let pending = app
        .auth_ui
        .token_to_persist
        .lock()
        .ok()
        .and_then(|mut g| g.take());
    if let Some(token) = pending {
        app.settings.auth.token = Some(token);
        let _ = app.settings.save();
    }

    let status = app
        .auth_ui
        .status
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();

    if matches!(status, AuthStatus::SignedIn(_)) {
        app.main_started = std::time::Instant::now();
        app.screen = Screen::Main;
        ui.ctx().request_repaint();
        return;
    }

    let t = (app.auth_started.elapsed().as_secs_f32() / ANIM_SECS).min(1.0);
    let te = ease_out_quart(t);

    let rect = ui.max_rect();
    let cx = rect.center().x;

    let logo_w = lerp(LOGO_W_START, LOGO_W_END, te);
    let logo_cy = lerp(rect.center().y, rect.min.y + 72.0, te);

    if let Some(tex) = &app.logo_texture {
        let tex_size = tex.size_vec2();
        let scale = logo_w / tex_size.x;
        let display = tex_size * scale;
        ui.painter().image(
            tex.id(),
            egui::Rect::from_center_size(egui::pos2(cx, logo_cy), display),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    // Content fades in during the second half of the animation; pinned to the
    // screen centre so it stays put regardless of the logo's final position.
    let fade = ((t - 0.45) / 0.55).clamp(0.0, 1.0);
    if fade > 0.0 {
        let alpha = (fade * 255.0) as u8;

        let heading_color = egui::Color32::from_rgba_unmultiplied(0xe6, 0xe6, 0xe6, alpha);
        let font_heading =
            egui::FontId::new(26.0, egui::FontFamily::Name(theme::HEADING_FAMILY.into()));

        let text_y = rect.center().y - 52.0;

        let g1 = ui.painter().text(
            egui::pos2(cx, text_y),
            egui::Align2::CENTER_TOP,
            "Войдите в аккаунт,",
            font_heading.clone(),
            heading_color,
        );
        ui.painter().text(
            egui::pos2(cx, text_y + g1.height() + 4.0),
            egui::Align2::CENTER_TOP,
            "чтобы открыть приложение",
            font_heading,
            heading_color,
        );

        let btn_cy = text_y + g1.height() * 2.0 + 4.0 + 32.0;

        match &status {
            AuthStatus::RequestingCode | AuthStatus::CheckingToken => {
                let sp_rect = egui::Rect::from_center_size(
                    egui::pos2(cx, btn_cy + 6.0),
                    egui::vec2(28.0, 28.0),
                );
                ui.put(sp_rect, egui::Spinner::new());
            }
            AuthStatus::AwaitingConfirmation(device) => {
                let code_color = egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, alpha);
                let hint_color = egui::Color32::from_rgba_unmultiplied(0x80, 0x80, 0x80, alpha);

                let hint_galley = ui.painter().text(
                    egui::pos2(cx, btn_cy),
                    egui::Align2::CENTER_TOP,
                    "Введите этот код в браузере:",
                    egui::FontId::proportional(13.0),
                    hint_color,
                );

                let code_top = btn_cy + hint_galley.height() + 10.0;
                let code_galley = ui.painter().text(
                    egui::pos2(cx, code_top),
                    egui::Align2::CENTER_TOP,
                    &device.user_code,
                    egui::FontId::monospace(32.0),
                    code_color,
                );

                // Clicking the code copies it; cursor changes to indicate interactivity.
                let code_resp = ui.interact(
                    egui::Rect::from_center_size(
                        egui::pos2(cx, code_top + code_galley.height() * 0.5),
                        egui::vec2(code_galley.width() + 24.0, code_galley.height() + 8.0),
                    ),
                    egui::Id::new("auth_code_copy"),
                    egui::Sense::click(),
                );
                if code_resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if code_resp.clicked() {
                    ui.ctx().copy_text(device.user_code.clone());
                }

                ui.painter().text(
                    egui::pos2(cx, code_top + code_galley.height() + 6.0),
                    egui::Align2::CENTER_TOP,
                    "нажмите, чтобы скопировать",
                    egui::FontId::proportional(11.0),
                    hint_color,
                );
            }
            _ if fade >= 0.6 => {
                let btn_rect = egui::Rect::from_center_size(
                    egui::pos2(cx, btn_cy + 22.0),
                    egui::vec2(200.0, 44.0),
                );
                let btn = egui::Button::new(
                    egui::RichText::new("Войти")
                        .color(theme::ON_ACCENT)
                        .strong()
                        .size(15.0),
                )
                .fill(theme::ACCENT)
                .corner_radius(egui::CornerRadius::same(10));

                if ui.put(btn_rect, btn).clicked() {
                    auth::spawn_device_flow(app);
                }

                if let AuthStatus::Error(ref msg) = status {
                    let err_color = egui::Color32::from_rgba_unmultiplied(
                        theme::ERROR.r(),
                        theme::ERROR.g(),
                        theme::ERROR.b(),
                        alpha,
                    );
                    ui.painter().text(
                        egui::pos2(cx, btn_cy + 22.0 + 30.0),
                        egui::Align2::CENTER_TOP,
                        msg.as_str(),
                        egui::FontId::proportional(12.0),
                        err_color,
                    );
                }
            }
            _ => {}
        }
    }

    if t < 1.0 {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn ease_out_quart(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(4)
}

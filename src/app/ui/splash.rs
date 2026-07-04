//! Splash screen: displays the Yandex Music logo for ~2 s, then routes based on auth status.

use std::time::Duration;

use crate::app::ui::auth::AuthStatus;
use crate::app::{Screen, YmdApp, theme};

const SPLASH_DURATION: Duration = Duration::from_millis(2000);
const LOGO_MAX_WIDTH: f32 = 280.0;

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    if app.logo_texture.is_none() {
        let bytes: &[u8] = include_bytes!("../../../assets/logo.png");
        match image::load_from_memory(bytes) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                app.logo_texture = Some(ui.ctx().load_texture(
                    "ym_logo",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
            Err(err) => tracing::warn!(%err, "failed to decode splash logo"),
        }
    }

    let rect = ui.max_rect();
    let center = rect.center();

    if let Some(tex) = &app.logo_texture {
        let tex_size = tex.size_vec2();
        let scale = (LOGO_MAX_WIDTH / tex_size.x).min(1.0);
        let display = tex_size * scale;
        let img_rect = egui::Rect::from_center_size(center, display);
        ui.painter().image(
            tex.id(),
            img_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    let elapsed = app.splash_start.elapsed();

    if elapsed >= SPLASH_DURATION {
        let status = app
            .auth_ui
            .status
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();

        match status {
            // Keep showing dots while the background token check is still running.
            AuthStatus::CheckingToken => {
                let dot_y = center.y + LOGO_MAX_WIDTH * 0.18 + 32.0;
                draw_dots(ui, center.x, dot_y, elapsed);
                ui.ctx().request_repaint_after(Duration::from_millis(16));
            }
            AuthStatus::SignedIn(_) => {
                app.main_started = std::time::Instant::now();
                app.screen = Screen::Main;
                ui.ctx().request_repaint();
            }
            _ => {
                app.auth_started = std::time::Instant::now();
                app.screen = Screen::Auth;
                ui.ctx().request_repaint();
            }
        }
    } else {
        let dot_y = center.y + LOGO_MAX_WIDTH * 0.18 + 32.0;
        draw_dots(ui, center.x, dot_y, elapsed);
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
}

fn draw_dots(ui: &mut egui::Ui, cx: f32, y: f32, elapsed: Duration) {
    let t = elapsed.as_secs_f32();
    let painter = ui.painter();
    let spacing = 10.0_f32;
    let r = 3.5_f32;

    for i in 0..3 {
        let phase = t * 2.5 - i as f32 * 0.35;
        let alpha = ((phase.sin() * 0.5 + 0.5) * 0.7 + 0.3).clamp(0.3, 1.0);
        let x = cx + (i as f32 - 1.0) * (r * 2.0 + spacing);
        let color = egui::Color32::from_rgba_unmultiplied(
            theme::ACCENT.r(),
            theme::ACCENT.g(),
            theme::ACCENT.b(),
            (alpha * 255.0) as u8,
        );
        painter.circle_filled(egui::pos2(x, y), r, color);
    }
}

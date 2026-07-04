//! Queue screen: link input, download submission, and task list with state indicators.

use crate::app::YmdApp;
use crate::app::theme;
use crate::app::ui::auth::AuthStatus;
use crate::download::queue::ItemState;

const LINK_INPUT_HEIGHT: f32 = 44.0;
const LINK_INPUT_PAD_X: f32 = 14.0;
const LINK_INPUT_ICON_GAP: f32 = 10.0;
const LINK_INPUT_CORNER: f32 = 10.0;

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    ui.add_space(8.0);
    ui.label(theme::heading("Загрузки", 26.0));
    ui.add_space(8.0);

    let signed_in = matches!(
        app.auth_ui
            .status
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default(),
        AuthStatus::SignedIn(_)
    );

    if !signed_in {
        ui.colored_label(
            theme::WARNING,
            "Войдите в аккаунт с активной подпиской (справа), чтобы скачивать треки.",
        );
        ui.add_space(8.0);
    }

    if app.settings.download_dir.is_none() {
        ui.colored_label(
            theme::WARNING,
            "Не выбрана папка для загрузок — задайте её в «Настройках».",
        );
        ui.add_space(8.0);
    }

    ui.horizontal(|ui| {
        let hint = "https://music.yandex.ru/album/… , /track/… или /playlist/…";
        let field_width = ui.available_width() - 160.0;
        let resp = link_input_field(ui, &mut app.link_input, hint, field_width);
        let download_btn = egui::Button::new("Скачать")
            .min_size(egui::vec2(ui.available_width(), LINK_INPUT_HEIGHT));
        let submit = ui.add_enabled(signed_in, download_btn).clicked()
            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && signed_in);
        if submit {
            let input = app.link_input.trim().to_owned();
            if !input.is_empty() {
                app.link_input.clear();
                app.queue.enqueue_link(
                    &app.runtime,
                    app.api_client.clone(),
                    app.settings.clone(),
                    input,
                    app.egui_ctx.clone(),
                );
            }
        }
    });

    ui.add_space(6.0);

    let (resolve_status, resolving, items) = {
        let guard = app
            .queue
            .state()
            .lock()
            .expect("queue mutex is not poisoned");
        (
            guard.resolve_status.clone(),
            guard.resolving,
            guard.items.clone(),
        )
    };

    ui.horizontal(|ui| {
        if resolving {
            ui.spinner();
        }
        if let Some(status) = &resolve_status {
            ui.label(status);
        }
        if !items.is_empty() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Очистить завершённые").clicked() {
                    app.queue.clear_finished();
                }
            });
        }
    });

    ui.add_space(6.0);
    ui.separator();

    let done = items
        .iter()
        .filter(|i| matches!(i.state, ItemState::Done { .. }))
        .count();
    let failed = items
        .iter()
        .filter(|i| matches!(i.state, ItemState::Failed { .. }))
        .count();
    let active = items
        .iter()
        .filter(|i| matches!(i.state, ItemState::Downloading))
        .count();
    if !items.is_empty() {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(format!("Всего: {}", items.len()));
            ui.colored_label(theme::SUCCESS, format!("Готово: {done}"));
            if active > 0 {
                ui.label(format!("Активно: {active}"));
            }
            if failed > 0 {
                ui.colored_label(theme::ERROR, format!("Ошибки: {failed}"));
            }
        });
        let fraction = if items.is_empty() {
            0.0
        } else {
            (done + failed) as f32 / items.len() as f32
        };
        ui.add(
            egui::ProgressBar::new(fraction)
                .desired_height(6.0)
                .fill(theme::ACCENT),
        );
        ui.add_space(4.0);
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for item in &items {
                render_item(ui, item);
            }
        });
}

/// Link input field styled like the original Yandex Music search bar: transparent background
/// with a thin outline at rest, thicker outline with a subtle fill on focus, and a search icon.
fn link_input_field(
    ui: &mut egui::Ui,
    text: &mut String,
    hint: &str,
    width: f32,
) -> egui::Response {
    let id = egui::Id::new("link_input_field");
    let focused = ui.memory(|m| m.has_focus(id));

    let (stroke, fill) = if focused {
        (
            egui::Stroke::new(2.0, egui::Color32::WHITE),
            egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 20),
        )
    } else {
        (
            egui::Stroke::new(1.5, theme::OUTLINE),
            egui::Color32::TRANSPARENT,
        )
    };

    let mut text_response = None;
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(LINK_INPUT_CORNER)
        .inner_margin(egui::Margin::symmetric(LINK_INPUT_PAD_X as i8, 0))
        .show(ui, |ui| {
            ui.set_width(width - LINK_INPUT_PAD_X * 2.0 - stroke.width * 2.0);
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui
                    .allocate_exact_size(egui::vec2(16.0, LINK_INPUT_HEIGHT), egui::Sense::hover());
                draw_search_icon(ui.painter(), icon_rect.center(), theme::TEXT_MUTED);
                ui.add_space(LINK_INPUT_ICON_GAP);
                text_response = Some(
                    ui.add(
                        egui::TextEdit::singleline(text)
                            .id(id)
                            .frame(egui::Frame::NONE)
                            .hint_text(hint)
                            .desired_width(ui.available_width())
                            .vertical_align(egui::Align::Center)
                            .margin(egui::Margin::ZERO),
                    ),
                );
            });
        });
    text_response.expect("text field is always added inside the frame")
}

/// Draws a magnifying-glass icon manually because the app's custom fonts lack that glyph.
fn draw_search_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.3, color);
    let radius = 5.0;
    let glass_center = center + egui::vec2(-1.5, -1.5);
    painter.circle_stroke(glass_center, radius, stroke);
    let handle_dir = egui::vec2(
        std::f32::consts::FRAC_1_SQRT_2,
        std::f32::consts::FRAC_1_SQRT_2,
    );
    let handle_start = glass_center + handle_dir * radius;
    let handle_end = handle_start + handle_dir * 4.5;
    painter.line_segment([handle_start, handle_end], stroke);
}

fn render_item(ui: &mut egui::Ui, item: &crate::download::queue::QueueItem) {
    egui::Frame::new()
        .fill(theme::BG_PLAYER)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(&item.title)
                            .color(theme::TEXT_PRIMARY)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(&item.artist)
                            .color(theme::TEXT_MUTED)
                            .size(12.0),
                    );
                });
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| match &item.state {
                        ItemState::Queued => {
                            ui.label(egui::RichText::new("В очереди").color(theme::TEXT_MUTED));
                        }
                        ItemState::Downloading => {
                            ui.spinner();
                            ui.label(egui::RichText::new("Скачивается").color(theme::ACCENT));
                        }
                        ItemState::Done {
                            codec,
                            bitrate,
                            path,
                        } => {
                            ui.label(
                                egui::RichText::new(format!("✓ {codec} {bitrate} kbps"))
                                    .color(theme::SUCCESS),
                            )
                            .on_hover_text(path);
                        }
                        ItemState::Failed { error } => {
                            ui.label(egui::RichText::new("✗ Ошибка").color(theme::ERROR))
                                .on_hover_text(error);
                        }
                    },
                );
            });
        });
    ui.add_space(6.0);
}

//! Queue screen: link input, download submission, and task list with state indicators.

use egui_material_icons::icons::ICON_SEARCH;

use crate::app::YmdApp;
use crate::app::theme;
use crate::download::pipeline::DownloadConfig;
use crate::download::queue::ItemState;

const LINK_INPUT_HEIGHT: f32 = 44.0;
const LINK_INPUT_PAD_X: f32 = 14.0;
const LINK_INPUT_ICON_GAP: f32 = 10.0;
const LINK_INPUT_CORNER: f32 = 10.0;

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    ui.horizontal(|ui| {
        let hint = "https://music.yandex.ru/album/… , /track/… или /playlist/…";
        let field_width = ui.available_width() - 160.0;
        let resp = link_input_field(ui, &mut app.link_input, hint, field_width);
        let download_btn = egui::Button::new("Скачать")
            .min_size(egui::vec2(ui.available_width(), LINK_INPUT_HEIGHT));

        let submit = ui.add(download_btn).clicked()
            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));

        if submit {
            let input = app.link_input.trim().to_owned();
            if !input.is_empty() {
                submit_link(app, input);
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

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if items.is_empty() {
                for i in 0..10 {
                    render_skeleton_row(ui, i);
                }
            } else {
                for item in &items {
                    render_item(ui, item);
                }
            }
        });
}

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

    let inner_h = LINK_INPUT_HEIGHT - stroke.width * 2.0;
    let mut text_response = None;

    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(LINK_INPUT_CORNER)
        .inner_margin(egui::Margin::symmetric(LINK_INPUT_PAD_X as i8, 0))
        .show(ui, |ui| {
            ui.set_width(width - LINK_INPUT_PAD_X * 2.0 - stroke.width * 2.0);
            ui.horizontal(|ui| {
                let (icon_rect, _) =
                    ui.allocate_exact_size(egui::vec2(18.0, inner_h), egui::Sense::hover());

                ui.painter().text(
                    icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    ICON_SEARCH.codepoint,
                    egui::FontId::new(18.0, ICON_SEARCH.font_family()),
                    theme::TEXT_MUTED,
                );
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

fn render_skeleton_row(ui: &mut egui::Ui, index: usize) {
    let t = ui.ctx().input(|i| i.time) as f32;
    let phase = (t * 1.4 - index as f32 * 0.3).sin() * 0.5 + 0.5;
    let alpha = (0.08 + phase * 0.06) * 255.0;
    let bar = egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, alpha as u8);

    egui::Frame::new()
        .fill(theme::BG_PLAYER)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(180.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 4.0, bar);
                    ui.add_space(4.0);
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(100.0, 11.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 3.0, bar);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(64.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 3.0, bar);
                });
            });
        });
    ui.add_space(6.0);
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(50));
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
                                egui::RichText::new(format!("{codec} {bitrate} kbps"))
                                    .color(theme::SUCCESS),
                            )
                            .on_hover_text(path);
                        }
                        ItemState::Failed { error } => {
                            ui.label(egui::RichText::new("Ошибка").color(theme::ERROR))
                                .on_hover_text(error);
                        }
                    },
                );
            });
        });
    ui.add_space(6.0);
}

fn submit_link(app: &mut YmdApp, input: String) {
    let Some(proj_arc) = app.current_project.clone() else {
        return;
    };

    let Ok(proj) = proj_arc.lock() else { return };
    let config = DownloadConfig {
        quality: proj.settings.quality,
        cover_size: proj.settings.cover_size,
        max_parallel_downloads: app.settings.max_parallel_downloads,
        root: proj.root.clone(),
        smart_library_organization: proj.settings.smart_library_organization,
        album_year_in_folder: proj.settings.album_year_in_folder,
        track_indexing: proj.settings.track_indexing,
        download_album_cover: proj.settings.download_album_cover,
        download_artist_image: proj.settings.download_artist_image,
    };

    drop(proj);
    app.link_input.clear();
    app.queue.enqueue_link(
        &app.runtime,
        app.api_client.clone(),
        config,
        proj_arc,
        input,
        app.egui_ctx.clone(),
    );
}

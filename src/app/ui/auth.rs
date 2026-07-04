//! Authentication screen: Device Code Flow (browser confirmation) and manual token entry.

use std::sync::{Arc, Mutex};

use crate::api::auth::{self, AccountInfo, DeviceCodeResponse};
use crate::api::client::ApiClient;
use crate::app::YmdApp;
use crate::app::theme;

const AVATAR_SIZE: f32 = 72.0;
const RING_GAP: f32 = 2.0;
const RING_WIDTH: f32 = 2.0;
const AVATAR_OUTER_SIZE: f32 = AVATAR_SIZE + (RING_GAP + RING_WIDTH) * 2.0;

/// Current state of the authorization process, shared between the UI thread and background tasks.
#[derive(Debug, Default, Clone)]
pub enum AuthStatus {
    #[default]
    SignedOut,
    RequestingCode,
    AwaitingConfirmation(DeviceCodeResponse),
    CheckingToken,
    SignedIn(AccountInfo),
    Error(String),
}

/// State for the authentication screen.
pub struct AuthUiState {
    pub status: Arc<Mutex<AuthStatus>>,
    pub manual_token_input: String,
    /// Token received in a background task, waiting to be persisted by the UI thread.
    pub token_to_persist: Arc<Mutex<Option<String>>>,
    /// Raw avatar bytes downloaded in background, waiting to be decoded on the UI thread.
    pub avatar_bytes: Arc<Mutex<Option<Vec<u8>>>>,
    /// Decoded avatar texture, created lazily on the UI thread from `avatar_bytes`.
    pub avatar_texture: Option<egui::TextureHandle>,
}

impl Default for AuthUiState {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(AuthStatus::SignedOut)),
            manual_token_input: String::new(),
            token_to_persist: Arc::new(Mutex::new(None)),
            avatar_bytes: Arc::new(Mutex::new(None)),
            avatar_texture: None,
        }
    }
}

fn set_status(status: &Arc<Mutex<AuthStatus>>, ctx: &egui::Context, new_status: AuthStatus) {
    if let Ok(mut guard) = status.lock() {
        *guard = new_status;
    }
    ctx.request_repaint();
}

fn spawn_avatar_fetch(
    avatar_url: String,
    avatar_bytes: Arc<Mutex<Option<Vec<u8>>>>,
    ctx: egui::Context,
) {
    tokio::spawn(async move {
        let http = reqwest::Client::new();
        match http
            .get(&avatar_url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
        {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => {
                    if let Ok(mut guard) = avatar_bytes.lock() {
                        *guard = Some(bytes.to_vec());
                    }
                    ctx.request_repaint();
                }
                Err(err) => tracing::warn!(%err, "failed to read avatar bytes"),
            },
            Err(err) => tracing::warn!(%err, "failed to download avatar"),
        }
    });
}

/// Spawns a background check of the already-saved token (called on application startup).
pub fn spawn_account_check(
    runtime: &tokio::runtime::Runtime,
    api_client: Arc<ApiClient>,
    status: Arc<Mutex<AuthStatus>>,
    avatar_bytes: Arc<Mutex<Option<Vec<u8>>>>,
    ctx: egui::Context,
) {
    set_status(&status, &ctx, AuthStatus::CheckingToken);
    runtime.spawn(async move {
        match auth::fetch_account_info(&api_client).await {
            Ok(info) => {
                if let Some(url) = info.get_avatar_url("islands-75") {
                    spawn_avatar_fetch(url, avatar_bytes, ctx.clone());
                }
                set_status(&status, &ctx, AuthStatus::SignedIn(info));
            }
            Err(err) => {
                tracing::warn!(%err, "failed to verify saved token");
                set_status(&status, &ctx, AuthStatus::Error(err.to_string()));
            }
        }
    });
}

fn spawn_device_flow(app: &YmdApp) {
    let status = app.auth_ui.status.clone();
    let token_to_persist = app.auth_ui.token_to_persist.clone();
    let avatar_bytes = app.auth_ui.avatar_bytes.clone();
    let api_client = app.api_client.clone();
    let ctx = app.egui_ctx.clone();
    set_status(&status, &ctx, AuthStatus::RequestingCode);

    app.runtime.spawn(async move {
        let http = reqwest::Client::new();
        let device = match auth::request_device_code(&http).await {
            Ok(device) => device,
            Err(err) => {
                set_status(&status, &ctx, AuthStatus::Error(err.to_string()));
                return;
            }
        };

        let _ = open::that(&device.verification_url);
        set_status(
            &status,
            &ctx,
            AuthStatus::AwaitingConfirmation(device.clone()),
        );

        match auth::wait_for_device_token(&http, &device).await {
            Ok(token) => {
                api_client.set_token(Some(token.clone()));
                if let Ok(mut guard) = token_to_persist.lock() {
                    *guard = Some(token);
                }
                set_status(&status, &ctx, AuthStatus::CheckingToken);
                match auth::fetch_account_info(&api_client).await {
                    Ok(info) => {
                        if let Some(url) = info.get_avatar_url("islands-75") {
                            spawn_avatar_fetch(url, avatar_bytes, ctx.clone());
                        }
                        set_status(&status, &ctx, AuthStatus::SignedIn(info));
                    }
                    Err(err) => set_status(&status, &ctx, AuthStatus::Error(err.to_string())),
                }
            }
            Err(err) => set_status(&status, &ctx, AuthStatus::Error(err.to_string())),
        }
    });
}

fn spawn_manual_login(app: &YmdApp, token: String) {
    let status = app.auth_ui.status.clone();
    let token_to_persist = app.auth_ui.token_to_persist.clone();
    let avatar_bytes = app.auth_ui.avatar_bytes.clone();
    let api_client = app.api_client.clone();
    let ctx = app.egui_ctx.clone();

    api_client.set_token(Some(token.clone()));
    if let Ok(mut guard) = token_to_persist.lock() {
        *guard = Some(token);
    }
    set_status(&status, &ctx, AuthStatus::CheckingToken);

    app.runtime.spawn(async move {
        match auth::fetch_account_info(&api_client).await {
            Ok(info) => {
                if let Some(url) = info.get_avatar_url("islands-75") {
                    spawn_avatar_fetch(url, avatar_bytes, ctx.clone());
                }
                set_status(&status, &ctx, AuthStatus::SignedIn(info));
            }
            Err(err) => set_status(&status, &ctx, AuthStatus::Error(err.to_string())),
        }
    });
}

pub(crate) fn sign_out(app: &mut YmdApp) {
    app.api_client.set_token(None);
    app.settings.auth.token = None;
    if let Err(err) = app.settings.save() {
        tracing::warn!(%err, "failed to save sign-out");
    }
    if let Ok(mut guard) = app.auth_ui.status.lock() {
        *guard = AuthStatus::SignedOut;
    }
    if let Ok(mut guard) = app.auth_ui.avatar_bytes.lock() {
        *guard = None;
    }
    app.auth_ui.avatar_texture = None;
}

pub fn show(ui: &mut egui::Ui, app: &mut YmdApp) {
    let persisted_token = app
        .auth_ui
        .token_to_persist
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    if let Some(token) = persisted_token {
        app.settings.auth.token = Some(token);
        if let Err(err) = app.settings.save() {
            tracing::warn!(%err, "failed to save authorization token");
        }
    }

    let status = app
        .auth_ui
        .status
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();

    ui.label(theme::heading("Аккаунт", 18.0));
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    match status {
        AuthStatus::SignedOut => show_signed_out(ui, app),
        AuthStatus::RequestingCode => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Запрашиваем код…");
            });
        }
        AuthStatus::AwaitingConfirmation(device) => show_awaiting_confirmation(ui, &device),
        AuthStatus::CheckingToken => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Проверяем токен…");
            });
        }
        AuthStatus::SignedIn(info) => show_signed_in(ui, app, &info),
        AuthStatus::Error(message) => {
            ui.colored_label(theme::ERROR, format!("Ошибка: {message}"));
            ui.add_space(8.0);
            show_signed_out(ui, app);
        }
    }
}

fn show_signed_out(ui: &mut egui::Ui, app: &mut YmdApp) {
    ui.label("Войдите в аккаунт с активной подпиской.");
    ui.add_space(10.0);

    let login_btn = egui::Button::new(
        egui::RichText::new("Войти через браузер")
            .color(theme::ON_ACCENT)
            .strong(),
    )
    .fill(theme::ACCENT);
    if ui
        .add_sized([ui.available_width(), 32.0], login_btn)
        .clicked()
    {
        spawn_device_flow(app);
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label("Или вставьте OAuth-токен:");
    ui.add(
        egui::TextEdit::singleline(&mut app.auth_ui.manual_token_input)
            .password(true)
            .desired_width(ui.available_width())
            .hint_text("y0_AgAAA…"),
    );
    ui.add_space(4.0);
    if ui
        .add_sized(
            [ui.available_width(), 28.0],
            egui::Button::new("Войти по токену"),
        )
        .clicked()
        && !app.auth_ui.manual_token_input.trim().is_empty()
    {
        let token = app.auth_ui.manual_token_input.trim().to_owned();
        app.auth_ui.manual_token_input.clear();
        spawn_manual_login(app, token);
    }
}

fn show_awaiting_confirmation(ui: &mut egui::Ui, device: &DeviceCodeResponse) {
    ui.label("Откройте эту страницу в браузере и введите код подтверждения:");
    ui.add_space(8.0);
    ui.hyperlink(&device.verification_url);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Код:");
        ui.monospace(&device.user_code);
        if ui.small_button("Копировать").clicked() {
            ui.ctx().copy_text(device.user_code.clone());
        }
    });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label("Ожидаем подтверждения…");
    });
}

const PLUS_GRADIENT: &[(f32, egui::Color32)] = &[
    (0.00, egui::Color32::from_rgb(0xff, 0x5c, 0x4d)),
    (0.26, egui::Color32::from_rgb(0xeb, 0x46, 0x9f)),
    (0.75, egui::Color32::from_rgb(0x83, 0x41, 0xef)),
    (1.00, egui::Color32::from_rgb(0x3f, 0x68, 0xf9)),
];

fn lerp_stops(stops: &[(f32, egui::Color32)], t: f32) -> egui::Color32 {
    if t <= stops[0].0 {
        return stops[0].1;
    }
    if t >= stops[stops.len() - 1].0 {
        return stops[stops.len() - 1].1;
    }
    for w in stops.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        if t >= t0 && t <= t1 {
            let f = (t - t0) / (t1 - t0);
            return egui::Color32::from_rgba_unmultiplied(
                (c0.r() as f32 + (c1.r() as f32 - c0.r() as f32) * f) as u8,
                (c0.g() as f32 + (c1.g() as f32 - c0.g() as f32) * f) as u8,
                (c0.b() as f32 + (c1.b() as f32 - c0.b() as f32) * f) as u8,
                255,
            );
        }
    }
    stops[stops.len() - 1].1
}

fn gradient_color(t: f32) -> egui::Color32 {
    lerp_stops(PLUS_GRADIENT, t)
}

/// Draws the Yandex Plus gradient ring around an avatar image.
fn draw_plus_ring(painter: &egui::Painter, center: egui::Pos2, outer_r: f32, inner_r: f32) {
    const SEGMENTS: usize = 256;
    const FEATHER: f32 = 1.0;
    let mut mesh = egui::Mesh::default();

    for i in 0..SEGMENTS {
        let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
        let cos = angle.cos();
        let sin = angle.sin();
        let t = (cos + 1.0) / 2.0;
        let color = gradient_color(t);
        let transparent = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);

        let base = mesh.vertices.len() as u32;
        for &(r, c) in &[
            (outer_r + FEATHER, transparent),
            (outer_r, color),
            (inner_r, color),
            (inner_r - FEATHER, transparent),
        ] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(center.x + r * cos, center.y + r * sin),
                uv: egui::epaint::WHITE_UV,
                color: c,
            });
        }

        let next = if i + 1 < SEGMENTS { base + 4 } else { 0 };
        for strip in 0..3_u32 {
            let (a, b, c, d) = (
                base + strip,
                base + strip + 1,
                next + strip,
                next + strip + 1,
            );
            mesh.indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }

    painter.add(egui::Shape::mesh(mesh));
}

fn show_signed_in(ui: &mut egui::Ui, app: &mut YmdApp, info: &AccountInfo) {
    let name = info
        .public_name
        .clone()
        .or_else(|| info.login.clone())
        .unwrap_or_else(|| info.uid.to_string());

    ui.vertical_centered(|ui| {
        ui.add_space(8.0);

        let has_plus = info.has_active_subscription();
        let total = if has_plus {
            AVATAR_OUTER_SIZE
        } else {
            AVATAR_SIZE
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(total, total), egui::Sense::hover());
        let img_rect =
            egui::Rect::from_center_size(rect.center(), egui::vec2(AVATAR_SIZE, AVATAR_SIZE));

        if has_plus {
            let inner_r = AVATAR_SIZE / 2.0 + RING_GAP;
            let outer_r = inner_r + RING_WIDTH;
            draw_plus_ring(ui.painter(), rect.center(), outer_r, inner_r);
        }

        if let Some(tex) = &app.auth_ui.avatar_texture {
            let sized = egui::load::SizedTexture::new(tex.id(), img_rect.size());
            egui::Image::new(sized)
                .corner_radius(AVATAR_SIZE / 2.0)
                .paint_at(ui, img_rect);
        } else {
            ui.painter().circle_filled(
                rect.center(),
                AVATAR_SIZE / 2.0,
                theme::ACCENT.gamma_multiply(0.25),
            );
            let initial = name
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                initial,
                egui::FontId::proportional(28.0),
                theme::ACCENT,
            );
        }

        ui.add_space(10.0);

        ui.label(
            egui::RichText::new(&name)
                .strong()
                .size(15.0)
                .color(theme::TEXT_PRIMARY),
        );
        if let Some(login) = &info.login {
            if login != &name {
                ui.label(
                    egui::RichText::new(login)
                        .color(theme::TEXT_MUTED)
                        .size(12.0),
                );
            }
        }

        ui.add_space(10.0);

        if info.has_active_subscription() {
            ui.colored_label(theme::SUCCESS, "Яндекс.Плюс активен");
        } else {
            ui.colored_label(theme::WARNING, "Подписка не обнаружена");
        }

        ui.add_space(16.0);
        if ui
            .add_sized([ui.available_width(), 28.0], egui::Button::new("Выйти"))
            .clicked()
        {
            sign_out(app);
        }
    });
}

const ISLAND_AVATAR: f32 = 36.0;
const ISLAND_MARGIN: f32 = 12.0;
const ISLAND_MARGIN_BOTTOM: f32 = 16.0;
const ISLAND_PADDING: f32 = 10.0;
const ISLAND_W: f32 = 210.0;
const POPUP_PADDING: f32 = 12.0;

pub fn show_island(ctx: &egui::Context, app: &mut YmdApp) {
    let pending = app
        .auth_ui
        .avatar_bytes
        .lock()
        .ok()
        .and_then(|mut g| g.take());
    if let Some(bytes) = pending {
        match image::load_from_memory(&bytes) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                app.auth_ui.avatar_texture = Some(ctx.load_texture(
                    "user_avatar",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
            Err(err) => tracing::warn!(%err, "failed to decode avatar image"),
        }
    }

    let status = app
        .auth_ui
        .status
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let screen = ctx.input(|i| i.viewport_rect());

    if app.show_account_popup {
        let popup_pos = egui::pos2(
            ISLAND_MARGIN,
            screen.max.y
                - ISLAND_AVATAR
                - ISLAND_PADDING * 2.0
                - ISLAND_MARGIN_BOTTOM
                - 8.0
                - 130.0,
        );
        egui::Area::new(egui::Id::new("account_popup"))
            .fixed_pos(popup_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::BG_POPOVER)
                    .corner_radius(12.0)
                    .stroke(egui::Stroke::new(1.0, theme::OUTLINE))
                    .inner_margin(egui::Margin::same(POPUP_PADDING as i8))
                    .show(ui, |ui| {
                        ui.set_min_width(ISLAND_W - POPUP_PADDING * 2.0);
                        if let AuthStatus::SignedIn(ref info) = status {
                            let name = info
                                .public_name
                                .clone()
                                .or_else(|| info.login.clone())
                                .unwrap_or_else(|| info.uid.to_string());
                            ui.label(
                                egui::RichText::new(&name)
                                    .strong()
                                    .color(theme::TEXT_PRIMARY),
                            );
                            if let Some(login) = &info.login {
                                if *login != name {
                                    ui.label(
                                        egui::RichText::new(login)
                                            .color(theme::TEXT_MUTED)
                                            .size(12.0),
                                    );
                                }
                            }
                            ui.add_space(8.0);
                        }
                        let sign_out_btn = egui::Button::new(
                            egui::RichText::new("Выйти").color(theme::TEXT_PRIMARY),
                        )
                        .fill(theme::SECONDARY_BG);
                        if ui
                            .add_sized([ui.available_width(), 30.0], sign_out_btn)
                            .clicked()
                        {
                            sign_out(app);
                            app.show_account_popup = false;
                        }
                    });
            });

        if ctx.input(|i| i.pointer.any_click()) {
            let click_pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default();
            let popup_rect =
                egui::Rect::from_min_size(popup_pos, egui::vec2(ISLAND_W, 130.0 + 24.0));
            let island_rect = egui::Rect::from_min_size(
                egui::pos2(
                    ISLAND_MARGIN,
                    screen.max.y - ISLAND_AVATAR - ISLAND_PADDING * 2.0 - ISLAND_MARGIN_BOTTOM,
                ),
                egui::vec2(ISLAND_W, ISLAND_AVATAR + ISLAND_PADDING * 2.0),
            );
            if !popup_rect.contains(click_pos) && !island_rect.contains(click_pos) {
                app.show_account_popup = false;
            }
        }
    }

    let island_y = screen.max.y - ISLAND_AVATAR - ISLAND_PADDING * 2.0 - ISLAND_MARGIN_BOTTOM;
    egui::Area::new(egui::Id::new("account_island"))
        .fixed_pos(egui::pos2(ISLAND_MARGIN, island_y))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let inner = egui::Frame::new()
                .fill(theme::BG_PLAYER)
                .corner_radius(12.0)
                .inner_margin(egui::Margin::symmetric(
                    ISLAND_PADDING as i8,
                    ISLAND_PADDING as i8,
                ))
                .show(ui, |ui| {
                    ui.set_min_width(ISLAND_W - ISLAND_PADDING * 2.0);
                    ui.horizontal(|ui| {
                        draw_small_avatar(ui, app, &status);
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.set_width(
                                ISLAND_W - ISLAND_AVATAR - ISLAND_PADDING * 2.0 - 8.0 - 20.0,
                            );
                            match &status {
                                AuthStatus::SignedIn(info) => {
                                    let name = info
                                        .public_name
                                        .as_deref()
                                        .or(info.login.as_deref())
                                        .unwrap_or("—");
                                    ui.label(
                                        egui::RichText::new(name)
                                            .strong()
                                            .size(13.0)
                                            .color(theme::TEXT_PRIMARY),
                                    );
                                    if info.has_active_subscription() {
                                        plus_badge(ui);
                                    }
                                }
                                AuthStatus::CheckingToken => {
                                    ui.spinner();
                                }
                                _ => {
                                    ui.label(
                                        egui::RichText::new("Войти")
                                            .size(13.0)
                                            .color(theme::TEXT_MUTED),
                                    );
                                }
                            }
                        });
                    });
                });

            let click = ui.interact(
                inner.response.rect,
                egui::Id::new("island_click"),
                egui::Sense::click(),
            );
            if click.clicked() {
                app.show_account_popup = !app.show_account_popup;
            }
            if click.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        });
}

fn draw_small_avatar(ui: &mut egui::Ui, app: &YmdApp, status: &AuthStatus) {
    let r = ISLAND_AVATAR / 2.0;
    let has_plus = matches!(status, AuthStatus::SignedIn(info) if info.has_active_subscription());

    let outer = if has_plus {
        r + RING_GAP + RING_WIDTH
    } else {
        r
    };
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(outer * 2.0, outer * 2.0), egui::Sense::hover());

    if has_plus {
        draw_plus_ring(ui.painter(), rect.center(), outer, r + RING_GAP);
    }

    let img_rect =
        egui::Rect::from_center_size(rect.center(), egui::vec2(ISLAND_AVATAR, ISLAND_AVATAR));
    if let Some(tex) = &app.auth_ui.avatar_texture {
        let sized = egui::load::SizedTexture::new(tex.id(), img_rect.size());
        egui::Image::new(sized)
            .corner_radius(r)
            .paint_at(ui, img_rect);
    } else {
        ui.painter()
            .circle_filled(rect.center(), r, theme::ACCENT.gamma_multiply(0.25));
        let initial = match status {
            AuthStatus::SignedIn(info) => info
                .public_name
                .as_deref()
                .or(info.login.as_deref())
                .and_then(|s| s.chars().next())
                .unwrap_or('?')
                .to_uppercase()
                .next()
                .unwrap_or('?'),
            _ => '?',
        };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            initial,
            egui::FontId::proportional(16.0),
            theme::ACCENT,
        );
    }
}

/// Small Yandex Plus badge rendered as a gradient pill.
fn plus_badge(ui: &mut egui::Ui) {
    let badge_size = egui::vec2(46.0, 16.0);
    let (rect, _) = ui.allocate_exact_size(badge_size, egui::Sense::hover());
    badge_gradient(ui.painter(), rect);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "+ Плюс",
        egui::FontId::proportional(10.5),
        egui::Color32::WHITE,
    );
}

const BADGE_STOPS: &[(f32, egui::Color32)] = &[
    (0.00, egui::Color32::from_rgb(0xff, 0x5c, 0x4d)),
    (0.25, egui::Color32::from_rgb(0xeb, 0x46, 0x9f)),
    (0.72, egui::Color32::from_rgb(0x83, 0x41, 0xef)),
    (1.00, egui::Color32::from_rgb(0x3f, 0x68, 0xf9)),
];

fn badge_gradient(painter: &egui::Painter, rect: egui::Rect) {
    let cr = rect.height() / 2.0;
    let w = rect.width();

    const N: usize = 10;
    let mut mesh = egui::Mesh::default();
    for i in 0..=N {
        let t = i as f32 / N as f32;
        let x = rect.min.x + t * w;
        let c = lerp_stops(BADGE_STOPS, t);
        mesh.vertices.push(egui::epaint::Vertex {
            pos: egui::pos2(x, rect.min.y),
            uv: egui::epaint::WHITE_UV,
            color: c,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: egui::pos2(x, rect.max.y),
            uv: egui::epaint::WHITE_UV,
            color: c,
        });
    }
    for i in 0..N as u32 {
        let b = i * 2;
        mesh.indices
            .extend_from_slice(&[b, b + 1, b + 2, b + 1, b + 3, b + 2]);
    }
    painter.add(egui::Shape::mesh(mesh));

    let bg = theme::BG_PLAYER;
    let c_l = lerp_stops(BADGE_STOPS, cr / w);
    let c_r = lerp_stops(BADGE_STOPS, (w - cr) / w);

    let corners: [(egui::Rect, egui::Pos2, egui::Color32); 4] = [
        (
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + cr, rect.min.y + cr)),
            egui::pos2(rect.min.x + cr, rect.min.y + cr),
            c_l,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(rect.max.x - cr, rect.min.y),
                egui::pos2(rect.max.x, rect.min.y + cr),
            ),
            egui::pos2(rect.max.x - cr, rect.min.y + cr),
            c_r,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.max.y - cr),
                egui::pos2(rect.min.x + cr, rect.max.y),
            ),
            egui::pos2(rect.min.x + cr, rect.max.y - cr),
            c_l,
        ),
        (
            egui::Rect::from_min_max(egui::pos2(rect.max.x - cr, rect.max.y - cr), rect.max),
            egui::pos2(rect.max.x - cr, rect.max.y - cr),
            c_r,
        ),
    ];
    for (sq, center, color) in corners {
        painter.rect_filled(sq, 0.0, bg);
        painter.circle_filled(center, cr, color);
    }
}

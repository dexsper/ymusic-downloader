//! Root application type for eframe and screen routing.

pub mod theme;
mod ui;

use std::sync::Arc;

use crate::api::client::ApiClient;
use crate::config::Settings;

/// Active application screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Splash,
    /// Shown when the user is not authenticated.
    Auth,
    Main,
}

/// Root application state, holding settings, the API client, and UI state.
pub struct YmdApp {
    pub settings: Settings,
    pub screen: Screen,
    pub show_settings: bool,
    pub api_client: Arc<ApiClient>,
    pub runtime: tokio::runtime::Runtime,
    pub egui_ctx: egui::Context,
    pub auth_ui: ui::auth::AuthUiState,
    pub queue: crate::download::queue::DownloadQueue,
    pub link_input: String,
    pub show_account_popup: bool,
    pub smart_org_expanded: bool,
    pub splash_start: std::time::Instant,
    pub logo_texture: Option<egui::TextureHandle>,
    /// Set to `Instant::now()` when transitioning to `Screen::Auth`; drives the logo animation.
    pub auth_started: std::time::Instant,
    /// Set to `Instant::now()` when transitioning to `Screen::Main`; drives the sidebar logo animation.
    pub main_started: std::time::Instant,
}

impl YmdApp {
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        egui_material_icons::initialize(&cc.egui_ctx);

        let mut settings = Settings::load().unwrap_or_else(|err| {
            tracing::warn!(%err, "failed to load settings, using defaults");
            Settings::default()
        });

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to create Tokio runtime");

        let api_client = ApiClient::new(&settings.auth).expect("failed to create HTTP client");
        if settings.auth.device_uuid.is_none() || settings.auth.device_id.is_none() {
            settings.auth.device_uuid = Some(api_client.device_uuid().to_owned());
            settings.auth.device_id = Some(api_client.device_id().to_owned());
            if let Err(err) = settings.save() {
                tracing::warn!(%err, "failed to save device_id/uuid");
            }
        }

        let api_client = Arc::new(api_client);
        let auth_ui = ui::auth::AuthUiState::default();
        if settings.auth.token.is_some() {
            ui::auth::spawn_account_check(
                &runtime,
                api_client.clone(),
                auth_ui.status.clone(),
                auth_ui.avatar_bytes.clone(),
                cc.egui_ctx.clone(),
            );
        }

        Self {
            settings,
            screen: Screen::Splash,
            show_settings: false,
            api_client,
            runtime,
            egui_ctx: cc.egui_ctx.clone(),
            auth_ui,
            queue: crate::download::queue::DownloadQueue::default(),
            link_input: String::new(),
            show_account_popup: false,
            smart_org_expanded: false,
            splash_start: std::time::Instant::now(),
            logo_texture: None,
            auth_started: std::time::Instant::now(),
            main_started: std::time::Instant::now(),
        }
    }
}

impl eframe::App for YmdApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, theme::BG_CONTENT);

        egui::Panel::top("title_bar")
            .exact_size(ui::widgets::TITLE_H)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_CONTENT)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let drag_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.max.x - ui::widgets::BTN_W * 3.0, rect.max.y),
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
            });

        ui::widgets::show_win_controls(ui.ctx());
        match self.screen {
            Screen::Splash => ui::splash::show(ui, self),
            Screen::Auth => ui::auth_screen::show(ui, self),
            Screen::Main => ui::main_screen::show(ui, self),
        }
    }
}

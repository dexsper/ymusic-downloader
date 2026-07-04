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
    Main,
}

/// Main screen tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Queue,
    Settings,
}

/// Root application state, holding settings, the API client, and UI state.
pub struct YmdApp {
    pub settings: Settings,
    pub screen: Screen,
    pub tab: Tab,
    /// Yandex Music API client shared between the UI thread and background tasks
    /// (`Arc` makes cloning cheap).
    pub api_client: Arc<ApiClient>,
    /// Dedicated Tokio runtime for network operations; keeps the egui UI thread responsive.
    pub runtime: tokio::runtime::Runtime,
    /// egui context cloned into background tasks to request repaints on completion.
    pub egui_ctx: egui::Context,
    pub auth_ui: ui::auth::AuthUiState,
    /// Download queue shared with background Tokio tasks.
    pub queue: crate::download::queue::DownloadQueue,
    /// Link input field text on the queue screen.
    pub link_input: String,
    /// Whether the account popup is open.
    pub show_account_popup: bool,
    /// Instant of application startup, used to time the splash screen.
    pub splash_start: std::time::Instant,
    /// Logo texture, loaded lazily on the first splash frame.
    pub logo_texture: Option<egui::TextureHandle>,
    /// Screen to transition to after the splash finishes.
    pub post_splash_screen: Screen,
}

impl YmdApp {
    /// Creates the application, loading persisted settings and starting the Tokio runtime.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

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

        let post_splash_screen = Screen::Main;

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
            post_splash_screen,
            tab: Tab::Queue,
            api_client,
            runtime,
            egui_ctx: cc.egui_ctx.clone(),
            auth_ui,
            queue: crate::download::queue::DownloadQueue::default(),
            link_input: String::new(),
            show_account_popup: false,
            splash_start: std::time::Instant::now(),
            logo_texture: None,
        }
    }
}

impl eframe::App for YmdApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, theme::BG_CONTENT);
        match self.screen {
            Screen::Splash => ui::splash::show(ui, self),
            Screen::Main => ui::main_screen::show(ui, self),
        }
    }
}

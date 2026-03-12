//! volt-app: macOS application shell.
//!
//! Thin native shell built directly on AppKit via `objc2-app-kit` (NOT winit —
//! Ghostty abandoned it because macOS-only apps need native tabs, IME, fullscreen,
//! and Services integration that leak through winit's abstractions).

mod app;
mod config;
mod event;
mod view;
mod window;

fn main() {
    // Initialize tracing (respects RUST_LOG env var)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Volt v{} starting", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = config::load_config();
    tracing::debug!("config: {config:?}");

    // Run the app (does not return)
    app::run_app(config);
}

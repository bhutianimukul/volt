//! volt-app: macOS application shell.
//!
//! Thin native shell built directly on AppKit via `objc2-app-kit` (NOT winit —
//! Ghostty abandoned it because macOS-only apps need native tabs, IME, fullscreen,
//! and Services integration that leak through winit's abstractions).

mod app;
mod window;
mod view;
mod event;
mod config;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Volt v{} starting", env!("CARGO_PKG_VERSION"));

    // TODO: Load config, create NSApplication, open window, start PTY, enter run loop
}

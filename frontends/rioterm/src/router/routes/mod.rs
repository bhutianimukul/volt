pub mod assistant;
pub mod bookmarks_viewer;
pub mod connections_viewer;
pub mod dialog;
pub mod env_viewer;
pub mod help;
pub mod history;
pub mod settings;
pub mod tmux_picker;
pub mod welcome;

#[derive(PartialEq)]
pub enum RoutePath {
    Assistant,
    Terminal,
    Welcome,
    ConfirmQuit,
    Settings,
    Help,
    History,
    TmuxPicker,
    EnvViewer,
    Bookmarks,
    Connections,
}

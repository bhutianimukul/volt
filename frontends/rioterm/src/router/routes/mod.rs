pub mod assistant;
pub mod dialog;
pub mod help;
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
    TmuxPicker,
}

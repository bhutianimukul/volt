pub mod assistant;
pub mod dialog;
pub mod settings;
pub mod welcome;

#[derive(PartialEq)]
pub enum RoutePath {
    Assistant,
    Terminal,
    Welcome,
    ConfirmQuit,
    Settings,
}

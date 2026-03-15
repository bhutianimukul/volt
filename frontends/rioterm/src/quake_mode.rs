//! Quake/dropdown terminal mode — global hotkey to toggle terminal visibility.
//!
//! When activated, the terminal slides down from the top of the screen.
//! Press the hotkey again to hide it.

#[derive(Debug, Clone)]
pub struct QuakeConfig {
    /// Whether quake mode is enabled
    pub enabled: bool,
    /// Height as fraction of screen (0.0 - 1.0)
    pub height: f32,
    /// Animation duration in milliseconds
    pub animation_ms: u64,
    /// Whether to show on all Spaces
    pub all_spaces: bool,
    /// Whether to float above other windows
    pub always_on_top: bool,
}

impl Default for QuakeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            height: 0.4, // 40% of screen height
            animation_ms: 200,
            all_spaces: true,
            always_on_top: true,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum QuakeState {
    Hidden,
    Showing,
    Visible,
    Hiding,
}

pub struct QuakeMode {
    pub config: QuakeConfig,
    pub state: QuakeState,
    animation_start: Option<std::time::Instant>,
}

impl QuakeMode {
    pub fn new(config: QuakeConfig) -> Self {
        Self {
            config,
            state: QuakeState::Hidden,
            animation_start: None,
        }
    }

    /// Toggle between visible and hidden
    pub fn toggle(&mut self) {
        match self.state {
            QuakeState::Hidden => {
                self.state = QuakeState::Showing;
                self.animation_start = Some(std::time::Instant::now());
            }
            QuakeState::Visible => {
                self.state = QuakeState::Hiding;
                self.animation_start = Some(std::time::Instant::now());
            }
            // If already animating, reverse direction
            QuakeState::Showing => {
                self.state = QuakeState::Hiding;
                self.animation_start = Some(std::time::Instant::now());
            }
            QuakeState::Hiding => {
                self.state = QuakeState::Showing;
                self.animation_start = Some(std::time::Instant::now());
            }
        }
    }

    /// Get the current animation progress (0.0 = hidden, 1.0 = fully visible)
    pub fn progress(&self) -> f32 {
        let Some(start) = self.animation_start else {
            return match self.state {
                QuakeState::Hidden | QuakeState::Hiding => 0.0,
                QuakeState::Visible | QuakeState::Showing => 1.0,
            };
        };

        let elapsed = start.elapsed().as_millis() as f32;
        let duration = self.config.animation_ms as f32;
        let raw = (elapsed / duration).min(1.0);

        // Ease-out cubic
        let eased = 1.0 - (1.0 - raw).powi(3);

        match self.state {
            QuakeState::Showing => eased,
            QuakeState::Hiding => 1.0 - eased,
            QuakeState::Visible => 1.0,
            QuakeState::Hidden => 0.0,
        }
    }

    /// Update state — call each frame during animation
    pub fn update(&mut self) {
        if let Some(start) = self.animation_start {
            let elapsed = start.elapsed().as_millis() as u64;
            if elapsed >= self.config.animation_ms {
                match self.state {
                    QuakeState::Showing => self.state = QuakeState::Visible,
                    QuakeState::Hiding => self.state = QuakeState::Hidden,
                    _ => {}
                }
                self.animation_start = None;
            }
        }
    }

    /// Get the window Y position based on animation progress
    pub fn window_y(&self, screen_height: f32) -> f32 {
        let target_height = screen_height * self.config.height;
        let progress = self.progress();
        -target_height + (target_height * progress)
    }

    /// Get the window height
    pub fn window_height(&self, screen_height: f32) -> f32 {
        screen_height * self.config.height
    }

    pub fn is_visible(&self) -> bool {
        !matches!(self.state, QuakeState::Hidden)
    }

    pub fn is_animating(&self) -> bool {
        matches!(self.state, QuakeState::Showing | QuakeState::Hiding)
    }
}

impl Default for QuakeMode {
    fn default() -> Self {
        Self::new(QuakeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle() {
        let mut quake = QuakeMode::default();
        assert_eq!(quake.state, QuakeState::Hidden);
        quake.toggle();
        assert_eq!(quake.state, QuakeState::Showing);
        // Simulate animation complete
        quake.state = QuakeState::Visible;
        quake.toggle();
        assert_eq!(quake.state, QuakeState::Hiding);
    }

    #[test]
    fn test_progress_hidden() {
        let quake = QuakeMode::default();
        assert_eq!(quake.progress(), 0.0);
    }

    #[test]
    fn test_window_height() {
        let quake = QuakeMode::default();
        let h = quake.window_height(1000.0);
        assert_eq!(h, 400.0); // 40% of 1000
    }

    #[test]
    fn test_is_visible() {
        let mut quake = QuakeMode::default();
        assert!(!quake.is_visible());
        quake.state = QuakeState::Showing;
        assert!(quake.is_visible());
        quake.state = QuakeState::Visible;
        assert!(quake.is_visible());
    }
}

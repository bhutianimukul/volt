//! Shell startup profiler — measures how long the shell takes to become ready.

use std::time::{Duration, Instant};

pub struct ShellProfiler {
    shell_start: Option<Instant>,
    prompt_ready: Option<Instant>,
}

impl ShellProfiler {
    pub fn new() -> Self {
        Self {
            shell_start: Some(Instant::now()),
            prompt_ready: None,
        }
    }

    /// Called when the first prompt appears (OSC 133;A)
    pub fn on_first_prompt(&mut self) {
        if self.prompt_ready.is_none() {
            self.prompt_ready = Some(Instant::now());
            if let (Some(start), Some(ready)) = (self.shell_start, self.prompt_ready) {
                let duration = ready.duration_since(start);
                if duration > Duration::from_millis(500) {
                    tracing::info!(
                        "Shell startup took {:.2}s — consider profiling with 'zsh -xv' or 'zprof'",
                        duration.as_secs_f64()
                    );
                }
            }
        }
    }

    pub fn startup_duration(&self) -> Option<Duration> {
        match (self.shell_start, self.prompt_ready) {
            (Some(s), Some(r)) => Some(r.duration_since(s)),
            _ => None,
        }
    }

    pub fn format_startup(&self) -> String {
        match self.startup_duration() {
            Some(d) if d.as_millis() < 100 => {
                format!("{}ms (fast)", d.as_millis())
            }
            Some(d) if d.as_millis() < 500 => {
                format!("{}ms (ok)", d.as_millis())
            }
            Some(d) if d.as_millis() < 2000 => {
                format!("{:.1}s (slow)", d.as_secs_f64())
            }
            Some(d) => format!(
                "{:.1}s (very slow — profile your shell config)",
                d.as_secs_f64()
            ),
            None => "measuring...".to_string(),
        }
    }
}

impl Default for ShellProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler() {
        let mut p = ShellProfiler::new();
        assert!(p.startup_duration().is_none());
        p.on_first_prompt();
        assert!(p.startup_duration().is_some());
        let s = p.format_startup();
        assert!(s.contains("ms") || s.contains("s"));
    }
}

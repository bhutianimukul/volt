//! Triggers — watch terminal output for patterns and fire actions.
//! Configure in ~/.config/volt/config.toml under [triggers]

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRule {
    pub name: String,
    pub pattern: String,
    pub action: TriggerAction,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerAction {
    #[serde(rename = "notify")]
    Notify { message: Option<String> },
    #[serde(rename = "highlight")]
    Highlight { color: Option<String> },
    #[serde(rename = "run")]
    RunCommand { command: String },
    #[serde(rename = "bell")]
    Bell,
    #[serde(rename = "log")]
    Log { level: Option<String> },
}

/// Compiled trigger with pre-compiled regex
pub struct CompiledTrigger {
    pub rule: TriggerRule,
    pub regex: Regex,
}

/// Trigger engine — watches output and fires actions
pub struct TriggerEngine {
    triggers: Vec<CompiledTrigger>,
    matches: std::collections::VecDeque<TriggerMatch>,
    max_matches: usize,
}

#[derive(Debug, Clone)]
pub struct TriggerMatch {
    pub trigger_name: String,
    pub matched_text: String,
    pub action: TriggerAction,
    pub line_number: usize,
}

impl TriggerEngine {
    pub fn new() -> Self {
        Self {
            triggers: Vec::new(),
            matches: std::collections::VecDeque::new(),
            max_matches: 1000,
        }
    }

    /// Load triggers from config
    pub fn load_rules(&mut self, rules: Vec<TriggerRule>) {
        self.triggers.clear();
        for rule in rules {
            if !rule.enabled {
                continue;
            }
            match Regex::new(&rule.pattern) {
                Ok(regex) => {
                    self.triggers.push(CompiledTrigger { rule, regex });
                }
                Err(e) => {
                    tracing::warn!("Invalid trigger pattern '{}': {}", rule.pattern, e);
                }
            }
        }
    }

    /// Check a line of output against all triggers
    pub fn check_line(&mut self, line: &str, line_number: usize) -> Vec<TriggerMatch> {
        let mut fired = Vec::new();

        for trigger in &self.triggers {
            if trigger.regex.is_match(line) {
                let matched = TriggerMatch {
                    trigger_name: trigger.rule.name.clone(),
                    matched_text: line.to_string(),
                    action: trigger.rule.action.clone(),
                    line_number,
                };
                fired.push(matched.clone());
                self.matches.push_back(matched);
            }
        }

        // Trim old matches (O(1) with VecDeque)
        while self.matches.len() > self.max_matches {
            self.matches.pop_front();
        }

        fired
    }

    /// Get all matches
    pub fn all_matches(&self) -> Vec<&TriggerMatch> {
        self.matches.iter().collect()
    }

    /// Get recent matches
    pub fn recent_matches(&self, count: usize) -> Vec<&TriggerMatch> {
        self.matches.iter().rev().take(count).collect()
    }

    /// Number of loaded triggers
    pub fn trigger_count(&self) -> usize {
        self.triggers.len()
    }

    /// Clear match history
    pub fn clear_matches(&mut self) {
        self.matches.clear();
    }
}

impl Default for TriggerEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in trigger rules that are always available
pub fn builtin_triggers() -> Vec<TriggerRule> {
    vec![
        TriggerRule {
            name: "error-detected".into(),
            pattern: r"(?i)\b(error|fatal|panic|exception)\b".into(),
            action: TriggerAction::Highlight {
                color: Some("#ff5555".into()),
            },
            enabled: true,
        },
        TriggerRule {
            name: "warning-detected".into(),
            pattern: r"(?i)\bwarning\b".into(),
            action: TriggerAction::Highlight {
                color: Some("#ffb86c".into()),
            },
            enabled: true,
        },
        TriggerRule {
            name: "test-failure".into(),
            pattern: r"(?i)(FAIL|FAILED|test.*failed)".into(),
            action: TriggerAction::Notify {
                message: Some("Test failure detected".into()),
            },
            enabled: true,
        },
        TriggerRule {
            name: "build-success".into(),
            pattern: r"(?i)(build succeeded|compilation successful|Finished)".into(),
            action: TriggerAction::Bell,
            enabled: true,
        },
        TriggerRule {
            name: "ip-address".into(),
            pattern: r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b".into(),
            action: TriggerAction::Highlight {
                color: Some("#8be9fd".into()),
            },
            enabled: false, // Disabled by default (noisy)
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_trigger() {
        let mut engine = TriggerEngine::new();
        engine.load_rules(vec![TriggerRule {
            name: "error".into(),
            pattern: r"ERROR".into(),
            action: TriggerAction::Highlight { color: None },
            enabled: true,
        }]);

        let matches = engine.check_line("Something ERROR happened", 1);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].trigger_name, "error");
    }

    #[test]
    fn test_no_match() {
        let mut engine = TriggerEngine::new();
        engine.load_rules(vec![TriggerRule {
            name: "error".into(),
            pattern: r"ERROR".into(),
            action: TriggerAction::Bell,
            enabled: true,
        }]);

        let matches = engine.check_line("All good here", 1);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_disabled_trigger() {
        let mut engine = TriggerEngine::new();
        engine.load_rules(vec![TriggerRule {
            name: "disabled".into(),
            pattern: r"test".into(),
            action: TriggerAction::Bell,
            enabled: false,
        }]);

        assert_eq!(engine.trigger_count(), 0);
    }

    #[test]
    fn test_multiple_triggers() {
        let mut engine = TriggerEngine::new();
        engine.load_rules(vec![
            TriggerRule {
                name: "error".into(),
                pattern: r"(?i)error".into(),
                action: TriggerAction::Highlight { color: None },
                enabled: true,
            },
            TriggerRule {
                name: "warning".into(),
                pattern: r"(?i)warning".into(),
                action: TriggerAction::Highlight { color: None },
                enabled: true,
            },
        ]);

        let m1 = engine.check_line("Error: something", 1);
        assert_eq!(m1.len(), 1);

        let m2 = engine.check_line("Warning: something", 2);
        assert_eq!(m2.len(), 1);

        assert_eq!(engine.all_matches().len(), 2);
    }

    #[test]
    fn test_builtin_triggers() {
        let builtins = builtin_triggers();
        assert!(builtins.len() >= 4);
        assert!(builtins.iter().any(|t| t.name == "error-detected"));
    }

    #[test]
    fn test_invalid_regex() {
        let mut engine = TriggerEngine::new();
        engine.load_rules(vec![TriggerRule {
            name: "bad".into(),
            pattern: r"[invalid".into(), // Invalid regex
            action: TriggerAction::Bell,
            enabled: true,
        }]);
        // Should not crash, just skip the bad rule
        assert_eq!(engine.trigger_count(), 0);
    }
}

/// Volt slash command system
/// When user types / at prompt start, Volt intercepts and runs built-in commands

#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub usage: &'static str,
    pub category: CommandCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandCategory {
    Navigation,
    Appearance,
    Tools,
    Session,
    Debug,
}

impl CommandCategory {
    pub fn name(&self) -> &str {
        match self {
            CommandCategory::Navigation => "Navigation",
            CommandCategory::Appearance => "Appearance",
            CommandCategory::Tools => "Tools",
            CommandCategory::Session => "Session",
            CommandCategory::Debug => "Debug",
        }
    }
}

/// All built-in slash commands
pub fn all_commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand {
            name: "split",
            description: "Split current pane horizontally or vertically",
            usage: "/split [right|down]",
            category: CommandCategory::Navigation,
        },
        SlashCommand {
            name: "zoom",
            description: "Toggle zoom on current pane",
            usage: "/zoom",
            category: CommandCategory::Navigation,
        },
        SlashCommand {
            name: "tab",
            description: "Create a new tab",
            usage: "/tab [name]",
            category: CommandCategory::Navigation,
        },
        SlashCommand {
            name: "close",
            description: "Close current pane or tab",
            usage: "/close",
            category: CommandCategory::Navigation,
        },
        SlashCommand {
            name: "theme",
            description: "Switch color theme",
            usage: "/theme [name]",
            category: CommandCategory::Appearance,
        },
        SlashCommand {
            name: "font",
            description: "Change font or font size",
            usage: "/font [size N | family NAME]",
            category: CommandCategory::Appearance,
        },
        SlashCommand {
            name: "opacity",
            description: "Set window opacity",
            usage: "/opacity [0.0-1.0]",
            category: CommandCategory::Appearance,
        },
        SlashCommand {
            name: "settings",
            description: "Open settings panel",
            usage: "/settings",
            category: CommandCategory::Appearance,
        },
        SlashCommand {
            name: "undo",
            description: "Undo last command's filesystem changes",
            usage: "/undo [list|N]",
            category: CommandCategory::Tools,
        },
        SlashCommand {
            name: "pipe",
            description: "Pipe previous command output through a filter",
            usage: "/pipe <filter command>",
            category: CommandCategory::Tools,
        },
        SlashCommand {
            name: "test",
            description: "Auto-detect and run project tests",
            usage: "/test [--failed|--affected]",
            category: CommandCategory::Tools,
        },
        SlashCommand {
            name: "debug",
            description: "Start debugger for current project",
            usage: "/debug [file:line]",
            category: CommandCategory::Tools,
        },
        SlashCommand {
            name: "search",
            description: "Search command history and output",
            usage: "/search <query>",
            category: CommandCategory::Session,
        },
        SlashCommand {
            name: "history",
            description: "Show session timeline",
            usage: "/history [--failed|--recent N]",
            category: CommandCategory::Session,
        },
        SlashCommand {
            name: "bookmark",
            description: "Bookmark current command block",
            usage: "/bookmark [name]",
            category: CommandCategory::Session,
        },
        SlashCommand {
            name: "share",
            description: "Share terminal session or block",
            usage: "/share [session|block]",
            category: CommandCategory::Session,
        },
        SlashCommand {
            name: "notify",
            description: "Send notification when current command finishes",
            usage: "/notify [message]",
            category: CommandCategory::Session,
        },
        SlashCommand {
            name: "sandbox",
            description: "Run next command in sandboxed environment",
            usage: "/sandbox <command>",
            category: CommandCategory::Debug,
        },
        SlashCommand {
            name: "ai",
            description: "Open AI assistant panel",
            usage: "/ai [question]",
            category: CommandCategory::Debug,
        },
        SlashCommand {
            name: "layout",
            description: "Save or load pane layout",
            usage: "/layout [save NAME|load NAME|list]",
            category: CommandCategory::Navigation,
        },
    ]
}

/// Fuzzy match slash commands against a query
pub fn fuzzy_match(query: &str) -> Vec<&'static SlashCommand> {
    static COMMANDS: std::sync::OnceLock<Vec<SlashCommand>> = std::sync::OnceLock::new();
    let commands = COMMANDS.get_or_init(all_commands);

    if query.is_empty() {
        return commands.iter().collect();
    }

    let q = query.to_lowercase();
    let mut scored: Vec<(usize, &SlashCommand)> = commands
        .iter()
        .filter_map(|cmd| {
            let name = cmd.name.to_lowercase();
            let desc = cmd.description.to_lowercase();

            // Exact prefix match scores highest
            if name.starts_with(&q) {
                return Some((100 - name.len(), cmd));
            }
            // Contains match
            if name.contains(&q) || desc.contains(&q) {
                return Some((50 - name.len(), cmd));
            }
            // Fuzzy: all query chars appear in order
            let mut qi = q.chars().peekable();
            for c in name.chars() {
                if qi.peek() == Some(&c) {
                    qi.next();
                }
            }
            if qi.peek().is_none() {
                return Some((25 - name.len(), cmd));
            }
            None
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, cmd)| cmd).collect()
}

/// Check if input looks like a slash command (starts with / at prompt start)
pub fn is_slash_command(input: &str) -> bool {
    let trimmed = input.trim();
    trimmed.starts_with('/')
        && !trimmed.contains("//") // Not a URL path
        && !trimmed.starts_with("/usr")
        && !trimmed.starts_with("/bin")
        && !trimmed.starts_with("/etc")
        && !trimmed.starts_with("/var")
        && !trimmed.starts_with("/tmp")
        && !trimmed.starts_with("/home")
        && !trimmed.starts_with("/opt")
        && !trimmed.starts_with("/dev")
}

/// Parse a slash command input into (command_name, args)
pub fn parse_slash_command(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    if !is_slash_command(trimmed) {
        return None;
    }
    let without_slash = &trimmed[1..];
    let (cmd, args) = match without_slash.find(' ') {
        Some(pos) => (&without_slash[..pos], without_slash[pos + 1..].trim()),
        None => (without_slash, ""),
    };
    Some((cmd, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_slash_command() {
        assert!(is_slash_command("/split"));
        assert!(is_slash_command("/theme dark"));
        assert!(!is_slash_command("/usr/bin/python"));
        assert!(!is_slash_command("/home/user"));
        assert!(!is_slash_command("https://example.com"));
    }

    #[test]
    fn test_parse_slash_command() {
        assert_eq!(
            parse_slash_command("/split right"),
            Some(("split", "right"))
        );
        assert_eq!(parse_slash_command("/zoom"), Some(("zoom", "")));
        assert_eq!(
            parse_slash_command("/theme tokyo-night"),
            Some(("theme", "tokyo-night"))
        );
        assert_eq!(parse_slash_command("/usr/bin/python"), None);
    }

    #[test]
    fn test_fuzzy_match() {
        let results = fuzzy_match("sp");
        assert!(results.iter().any(|c| c.name == "split"));

        let results = fuzzy_match("und");
        assert!(results.iter().any(|c| c.name == "undo"));

        let results = fuzzy_match("");
        assert!(results.len() > 10); // All commands returned
    }
}

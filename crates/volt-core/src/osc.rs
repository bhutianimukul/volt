//! OSC (Operating System Command) handling.
//!
//! Handles: OSC 0/1/2 (window/icon titles), OSC 7 (current working directory),
//! OSC 8 (hyperlinks), OSC 10/11/12 (default colors), OSC 52 (clipboard),
//! OSC 133 (shell integration — prompt/command/output markers for block model).

/// Parsed OSC command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscCommand {
    /// OSC 0: Set icon name and window title.
    SetTitle(String),
    /// OSC 1: Set icon name.
    SetIconName(String),
    /// OSC 2: Set window title.
    SetWindowTitle(String),
    /// OSC 7: Set current working directory.
    SetWorkingDirectory(String),
    /// OSC 8: Set/clear hyperlink. `params` is the URI parameters, `uri` is the link.
    SetHyperlink {
        params: String,
        uri: Option<String>,
    },
    /// OSC 10: Query/set default foreground color.
    DefaultForeground(Option<String>),
    /// OSC 11: Query/set default background color.
    DefaultBackground(Option<String>),
    /// OSC 12: Query/set cursor color.
    CursorColor(Option<String>),
    /// OSC 52: Clipboard operation. `clipboard` is which clipboard (c, p, s, etc.),
    /// `data` is base64-encoded content or "?" to query.
    Clipboard {
        clipboard: String,
        data: String,
    },
    /// OSC 133: Shell integration marker.
    ShellIntegration(ShellIntegrationMark),
    /// Unrecognized OSC.
    Unknown(Vec<Vec<u8>>),
}

/// OSC 133 shell integration markers (iTerm2/Ghostty convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellIntegrationMark {
    /// 133;A — Prompt start.
    PromptStart,
    /// 133;B — Command input start (after prompt, user is typing).
    CommandStart,
    /// 133;C — Command output start (command is executing).
    OutputStart,
    /// 133;D[;exit_code] — Command finished.
    CommandEnd(Option<i32>),
}

/// Parse raw OSC parameters (as byte slices) into an `OscCommand`.
///
/// The `vte` crate gives us OSC data as `&[&[u8]]` — each segment separated by `;`.
pub fn parse_osc(params: &[&[u8]]) -> OscCommand {
    if params.is_empty() {
        return OscCommand::Unknown(params.iter().map(|p| p.to_vec()).collect());
    }

    let command = std::str::from_utf8(params[0]).unwrap_or("");

    match command {
        "0" => {
            let title = get_param_str(params, 1).unwrap_or_default();
            OscCommand::SetTitle(title)
        }
        "1" => {
            let name = get_param_str(params, 1).unwrap_or_default();
            OscCommand::SetIconName(name)
        }
        "2" => {
            let title = get_param_str(params, 1).unwrap_or_default();
            OscCommand::SetWindowTitle(title)
        }
        "7" => {
            let dir = get_param_str(params, 1).unwrap_or_default();
            OscCommand::SetWorkingDirectory(dir)
        }
        "8" => {
            let osc_params = get_param_str(params, 1).unwrap_or_default();
            let uri = get_param_str(params, 2);
            OscCommand::SetHyperlink {
                params: osc_params,
                uri: if uri.as_deref() == Some("") { None } else { uri },
            }
        }
        "10" => OscCommand::DefaultForeground(get_param_str(params, 1)),
        "11" => OscCommand::DefaultBackground(get_param_str(params, 1)),
        "12" => OscCommand::CursorColor(get_param_str(params, 1)),
        "52" => {
            let clipboard = get_param_str(params, 1).unwrap_or_default();
            let data = get_param_str(params, 2).unwrap_or_default();
            OscCommand::Clipboard { clipboard, data }
        }
        "133" => parse_shell_integration(params),
        _ => OscCommand::Unknown(params.iter().map(|p| p.to_vec()).collect()),
    }
}

fn parse_shell_integration(params: &[&[u8]]) -> OscCommand {
    let mark = get_param_str(params, 1).unwrap_or_default();
    match mark.as_str() {
        "A" => OscCommand::ShellIntegration(ShellIntegrationMark::PromptStart),
        "B" => OscCommand::ShellIntegration(ShellIntegrationMark::CommandStart),
        "C" => OscCommand::ShellIntegration(ShellIntegrationMark::OutputStart),
        d if d.starts_with('D') => {
            // "D" or "D;0" or "D;1" etc.
            let exit_code = if params.len() > 2 {
                get_param_str(params, 2).and_then(|s| s.parse().ok())
            } else if d.len() > 1 {
                d[1..].trim_start_matches(';').parse().ok()
            } else {
                None
            };
            OscCommand::ShellIntegration(ShellIntegrationMark::CommandEnd(exit_code))
        }
        _ => OscCommand::Unknown(params.iter().map(|p| p.to_vec()).collect()),
    }
}

/// Extract a parameter at the given index as a String.
fn get_param_str(params: &[&[u8]], index: usize) -> Option<String> {
    params
        .get(index)
        .and_then(|p| std::str::from_utf8(p).ok())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_set_title() {
        let params: &[&[u8]] = &[b"0", b"My Terminal"];
        assert_eq!(
            parse_osc(params),
            OscCommand::SetTitle("My Terminal".into())
        );
    }

    #[test]
    fn parse_cwd() {
        let params: &[&[u8]] = &[b"7", b"file:///Users/mukul/code"];
        assert_eq!(
            parse_osc(params),
            OscCommand::SetWorkingDirectory("file:///Users/mukul/code".into())
        );
    }

    #[test]
    fn parse_hyperlink_set() {
        let params: &[&[u8]] = &[b"8", b"id=123", b"https://example.com"];
        assert_eq!(
            parse_osc(params),
            OscCommand::SetHyperlink {
                params: "id=123".into(),
                uri: Some("https://example.com".into()),
            }
        );
    }

    #[test]
    fn parse_hyperlink_clear() {
        let params: &[&[u8]] = &[b"8", b"", b""];
        assert_eq!(
            parse_osc(params),
            OscCommand::SetHyperlink {
                params: String::new(),
                uri: None,
            }
        );
    }

    #[test]
    fn parse_shell_integration_markers() {
        assert_eq!(
            parse_osc(&[b"133", b"A"]),
            OscCommand::ShellIntegration(ShellIntegrationMark::PromptStart)
        );
        assert_eq!(
            parse_osc(&[b"133", b"C"]),
            OscCommand::ShellIntegration(ShellIntegrationMark::OutputStart)
        );
    }

    #[test]
    fn parse_shell_integration_command_end() {
        assert_eq!(
            parse_osc(&[b"133", b"D", b"0"]),
            OscCommand::ShellIntegration(ShellIntegrationMark::CommandEnd(Some(0)))
        );
        assert_eq!(
            parse_osc(&[b"133", b"D"]),
            OscCommand::ShellIntegration(ShellIntegrationMark::CommandEnd(None))
        );
    }

    #[test]
    fn parse_clipboard() {
        let params: &[&[u8]] = &[b"52", b"c", b"SGVsbG8="];
        assert_eq!(
            parse_osc(params),
            OscCommand::Clipboard {
                clipboard: "c".into(),
                data: "SGVsbG8=".into(),
            }
        );
    }
}

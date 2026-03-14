use std::io::Write;
use std::process::{Command, Stdio};

/// Pipe previously captured output through a new command.
///
/// Example: If `kubectl get pods` output was captured, the user can later
/// pipe it through `grep Running` without re-running kubectl.
pub fn pipe_through(
    captured_output: &[u8],
    filter_command: &str,
) -> Result<PipeResult, PipeError> {
    if filter_command.trim().is_empty() {
        return Err(PipeError::EmptyCommand);
    }

    // Parse the filter command — support pipes within the filter itself
    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    };
    let shell_flag = if cfg!(target_os = "windows") {
        "/C"
    } else {
        "-c"
    };

    let mut child = Command::new(shell)
        .arg(shell_flag)
        .arg(filter_command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| PipeError::SpawnFailed(e.to_string()))?;

    // Write captured output to the filter's stdin
    if let Some(mut stdin) = child.stdin.take() {
        // Write in a thread to avoid deadlock with large outputs
        let data = captured_output.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
            // stdin is dropped here, closing the pipe
        });
    }

    let output = child
        .wait_with_output()
        .map_err(|e| PipeError::WaitFailed(e.to_string()))?;

    Ok(PipeResult {
        stdout: output.stdout,
        stderr: output.stderr,
        exit_code: output.status.code().unwrap_or(-1),
    })
}

#[derive(Debug)]
pub struct PipeResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}

#[derive(Debug)]
pub enum PipeError {
    EmptyCommand,
    SpawnFailed(String),
    WaitFailed(String),
}

impl std::fmt::Display for PipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipeError::EmptyCommand => write!(f, "Filter command is empty"),
            PipeError::SpawnFailed(e) => write!(f, "Failed to spawn filter: {}", e),
            PipeError::WaitFailed(e) => write!(f, "Filter process error: {}", e),
        }
    }
}

/// Suggest common filter commands based on the output content
pub fn suggest_filters(output: &str) -> Vec<&'static str> {
    let mut suggestions = vec!["grep ", "head -20", "tail -20", "wc -l", "sort"];

    // If output looks like JSON, suggest jq
    let trimmed = output.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        suggestions.insert(0, "jq '.'");
        suggestions.insert(1, "jq '.[]'");
        suggestions.insert(2, "jq 'keys'");
    }

    // If output has columns, suggest awk
    if output
        .lines()
        .take(5)
        .any(|l| l.split_whitespace().count() > 3)
    {
        suggestions.push("awk '{print $1}'");
        suggestions.push("awk '{print $NF}'");
    }

    // If output has many lines, suggest filtering
    if output.lines().count() > 20 {
        suggestions.push("head -10");
        suggestions.push("grep -i ");
        suggestions.push("grep -c ''");
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipe_through_grep() {
        let output = b"hello world\nfoo bar\nhello rust\n";
        let result = pipe_through(output, "grep hello").unwrap();
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(stdout.contains("hello world"));
        assert!(stdout.contains("hello rust"));
        assert!(!stdout.contains("foo bar"));
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_pipe_through_wc() {
        let output = b"line1\nline2\nline3\n";
        let result = pipe_through(output, "wc -l").unwrap();
        let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
        assert_eq!(stdout, "3");
    }

    #[test]
    fn test_pipe_empty_command() {
        let result = pipe_through(b"test", "");
        assert!(matches!(result, Err(PipeError::EmptyCommand)));
    }

    #[test]
    fn test_suggest_filters_json() {
        let suggestions = suggest_filters(r#"{"key": "value"}"#);
        assert!(suggestions.iter().any(|s| s.contains("jq")));
    }

    #[test]
    fn test_suggest_filters_tabular() {
        let output = "NAME    STATUS    AGE    RESTARTS\npod1    Running   5d     0\npod2    Pending   1h     3\n";
        let suggestions = suggest_filters(output);
        assert!(suggestions.iter().any(|s| s.contains("awk")));
    }
}

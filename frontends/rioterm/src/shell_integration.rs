//! Shell integration scripts — auto-inject OSC 133 sequences for block tracking.
//! Supports zsh, bash, and fish.

/// Get the shell integration script for the given shell
pub fn integration_script(shell: &str) -> Option<&'static str> {
    let base = shell.rsplit('/').next().unwrap_or(shell);
    match base {
        "zsh" => Some(ZSH_INTEGRATION),
        "bash" => Some(BASH_INTEGRATION),
        "fish" => Some(FISH_INTEGRATION),
        _ => None,
    }
}

/// Get the environment variable name to source the integration
pub fn integration_env_var(shell: &str) -> Option<(&'static str, &'static str)> {
    let base = shell.rsplit('/').next().unwrap_or(shell);
    match base {
        "zsh" => Some(("ZDOTDIR", "")), // We'll use precmd/preexec hooks
        "bash" => Some(("PROMPT_COMMAND", "")),
        _ => None,
    }
}

/// Path where we write the integration script
pub fn integration_script_path(shell: &str) -> Option<std::path::PathBuf> {
    let base = shell.rsplit('/').next().unwrap_or(shell);
    let dir = rio_backend::config::config_dir_path().join("shell");
    let _ = std::fs::create_dir_all(&dir);

    match base {
        "zsh" => Some(dir.join("volt-integration.zsh")),
        "bash" => Some(dir.join("volt-integration.bash")),
        "fish" => Some(dir.join("volt-integration.fish")),
        _ => None,
    }
}

/// Write integration scripts to disk so shells can source them
pub fn install_integration_scripts() {
    let shells = ["zsh", "bash", "fish"];
    for shell in &shells {
        if let (Some(path), Some(script)) =
            (integration_script_path(shell), integration_script(shell))
        {
            if let Err(e) = std::fs::write(&path, script) {
                tracing::warn!("Failed to write {} integration script: {}", shell, e);
            } else {
                tracing::info!("Installed {} integration at {}", shell, path.display());
            }
        }
    }
}

// OSC 133 sequences:
// \x1b]133;A\x07  = prompt start
// \x1b]133;B\x07  = command input start (after prompt, before command runs)
// \x1b]133;C\x07  = command output start (command is executing)
// \x1b]133;D;$?\x07 = command finished with exit code

const ZSH_INTEGRATION: &str = r#"# Volt Terminal — ZSH Integration
# Emits OSC 133 sequences for shell integration (block model)
# Source this file in your .zshrc or it will be auto-sourced by Volt

__volt_prompt_start() {
    printf '\e]133;A\a'
}

__volt_command_start() {
    printf '\e]133;B\a'
}

__volt_preexec() {
    printf '\e]133;C\a'
}

__volt_precmd() {
    local exit_code=$?
    printf '\e]133;D;%d\a' "$exit_code"
    __volt_prompt_start
}

# Install hooks if not already installed
if [[ -z "$__VOLT_INTEGRATION_INSTALLED" ]]; then
    export __VOLT_INTEGRATION_INSTALLED=1

    # precmd — runs before each prompt
    autoload -Uz add-zsh-hook
    add-zsh-hook precmd __volt_precmd

    # preexec — runs before each command executes
    add-zsh-hook preexec __volt_preexec

    # Mark command input start after prompt is drawn
    # This uses zle-line-init to detect when the user starts typing
    __volt_zle_line_init() {
        __volt_command_start
    }
    zle -N zle-line-init __volt_zle_line_init

    # Initial prompt start
    __volt_prompt_start
fi
"#;

const BASH_INTEGRATION: &str = r#"# Volt Terminal — Bash Integration
# Emits OSC 133 sequences for shell integration (block model)
# Source this file in your .bashrc or it will be auto-sourced by Volt

if [[ -z "$__VOLT_INTEGRATION_INSTALLED" ]]; then
    export __VOLT_INTEGRATION_INSTALLED=1

    __volt_prompt_start() {
        printf '\e]133;A\a'
    }

    __volt_command_start() {
        printf '\e]133;B\a'
    }

    __volt_preexec() {
        printf '\e]133;C\a'
    }

    __volt_precmd() {
        local exit_code=$?
        printf '\e]133;D;%d\a' "$exit_code"
        __volt_prompt_start
        __volt_command_start
    }

    # Use DEBUG trap for preexec
    __volt_debug_trap() {
        # Only fire on actual commands, not PROMPT_COMMAND
        if [[ "$BASH_COMMAND" != "$PROMPT_COMMAND" ]]; then
            __volt_preexec
        fi
    }

    trap '__volt_debug_trap' DEBUG

    # Prepend to PROMPT_COMMAND
    PROMPT_COMMAND="__volt_precmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}"

    # Initial prompt start
    __volt_prompt_start
fi
"#;

const FISH_INTEGRATION: &str = r#"# Volt Terminal — Fish Integration
# Emits OSC 133 sequences for shell integration (block model)
# Source this file in your config.fish or it will be auto-sourced by Volt

if not set -q __VOLT_INTEGRATION_INSTALLED
    set -g __VOLT_INTEGRATION_INSTALLED 1

    function __volt_prompt_start --on-event fish_prompt
        printf '\e]133;A\a'
        printf '\e]133;B\a'
    end

    function __volt_preexec --on-event fish_preexec
        printf '\e]133;C\a'
    end

    function __volt_postexec --on-event fish_postexec
        printf '\e]133;D;%d\a' $status
    end

    # Initial prompt start
    printf '\e]133;A\a'
end
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_script_zsh() {
        let script = integration_script("zsh").unwrap();
        assert!(script.contains("133;A"));
        assert!(script.contains("133;B"));
        assert!(script.contains("133;C"));
        assert!(script.contains("133;D"));
        assert!(script.contains("precmd"));
        assert!(script.contains("preexec"));
    }

    #[test]
    fn test_integration_script_bash() {
        let script = integration_script("bash").unwrap();
        assert!(script.contains("PROMPT_COMMAND"));
        assert!(script.contains("DEBUG"));
    }

    #[test]
    fn test_integration_script_fish() {
        let script = integration_script("fish").unwrap();
        assert!(script.contains("fish_preexec"));
        assert!(script.contains("fish_postexec"));
    }

    #[test]
    fn test_integration_script_unknown() {
        assert!(integration_script("powershell").is_none());
    }

    #[test]
    fn test_full_path_shell() {
        let script = integration_script("/bin/zsh").unwrap();
        assert!(script.contains("133;A"));
    }

    #[test]
    fn test_script_path() {
        let path = integration_script_path("zsh").unwrap();
        assert!(path.to_string_lossy().contains("volt-integration.zsh"));
    }
}

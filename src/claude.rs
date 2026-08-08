//! Shells out to the configured AI CLI in one-shot, non-interactive mode.
//!
//! Confirmed via `claude --help` on this machine: `-p`/`--print` runs a
//! single turn, prints the response, and exits — exactly the "prompt in,
//! text out" shape this needs, no `--output-format json` parsing
//! required since the default text output is already just the answer.

use std::process::Command;

/// Runs `<cli> -p "<prompt>"` and returns trimmed stdout as the answer.
/// `cli` comes from `og_config::Config::default_ai_cli`, falling back to
/// `"claude"` if that field is empty (matches og-search's own fallback
/// behavior for the same field).
pub fn ask(cli: &str, prompt: &str) -> Result<String, String> {
    let cli = if cli.trim().is_empty() { "claude" } else { cli.trim() };
    let output = Command::new(cli)
        .arg("-p")
        .arg(prompt)
        .output()
        .map_err(|e| format!("failed to run `{cli}`: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("`{cli}` exited with {}", output.status)
        } else {
            stderr
        });
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        Err(format!("`{cli}` returned no output"))
    } else {
        Ok(text)
    }
}

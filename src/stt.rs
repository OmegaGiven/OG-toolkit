//! HTTP client for the local faster-whisper server (`stt-server/`), plus
//! best-effort auto-start of that server if it isn't already listening.
//!
//! `ureq` (blocking, sync) is used instead of an async client because
//! every call here already happens on its own dedicated background
//! thread (`audio.rs`'s capture thread) — pulling in an async HTTP stack
//! just to `.await` from a thread with nothing else to do would be pure
//! overhead.

use serde::Deserialize;
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Deserialize)]
struct TranscribeResponse {
    text: String,
}

/// POSTs one WAV chunk, returns the transcribed text (possibly empty for
/// silence). A network/parse error is surfaced as `Err` so the caller can
/// decide whether to drop the chunk and keep going (a single dropped
/// chunk isn't fatal to the running transcript) or surface it to the UI.
pub fn transcribe(server_url: &str, wav_bytes: &[u8]) -> Result<String, String> {
    let url = format!("{}/transcribe", server_url.trim_end_matches('/'));
    let resp = ureq::post(&url)
        .set("Content-Type", "audio/wav")
        .timeout(Duration::from_secs(10))
        .send_bytes(wav_bytes)
        .map_err(|e| e.to_string())?;
    let parsed: TranscribeResponse = resp.into_json().map_err(|e| e.to_string())?;
    Ok(parsed.text)
}

fn is_up(server_url: &str) -> bool {
    ureq::get(&format!("{}/health", server_url.trim_end_matches('/')))
        .timeout(Duration::from_millis(400))
        .call()
        .is_ok()
}

/// If the STT server isn't answering `/health` yet, spawns it in the
/// background (venv Python, detached — stdout/stderr dropped rather than
/// inherited, since this runs from a background thread with no terminal
/// of its own) and polls briefly for it to come up. Returns `true` once
/// the server is reachable (already running, or came up within the
/// timeout), `false` if it never answered — the caller falls back to a
/// clear "STT server unavailable" message rather than hanging forever.
pub fn ensure_running(server_url: &str) -> bool {
    if is_up(server_url) {
        return true;
    }

    let home = og_config::dirs_home();
    let venv_python = home.join(".local/share/og-voice/stt-venv/bin/python3");
    let script = home.join(".local/share/og-voice/stt_server.py");

    if venv_python.exists() && script.exists() {
        let _ = Command::new(venv_python)
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }

    for _ in 0..30 {
        if is_up(server_url) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

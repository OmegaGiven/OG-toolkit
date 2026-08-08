//! The whole recording -> transcribing -> asking-Claude pipeline, run
//! entirely on background threads so the iced event loop (main thread)
//! never blocks. `App` polls `Shared` on a timer (see `app.rs`'s `Tick`)
//! rather than receiving messages through a channel — a plain
//! `Arc<Mutex<Shared>>` is simpler here than threading a non-`Send`
//! `mpsc::Receiver` through iced's `Task`/subscription machinery for
//! what's fundamentally just "latest known state", not a message log.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{claude, stt, wav};

const SAMPLE_RATE: u32 = 16_000;
const CHANNELS: u16 = 1;
const BITS: u16 = 16;
/// ~1.5s chunks, per the brief's chunked-not-true-streaming tradeoff.
const CHUNK_BYTES: usize = (SAMPLE_RATE as usize) * (CHANNELS as usize) * (BITS as usize / 8) * 3 / 2;

#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    /// Waiting on the STT server to come up (only visible if it wasn't
    /// already running when the hotkey was pressed).
    StartingStt,
    Recording,
    Processing,
    Done,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Shared {
    pub stage: Stage,
    pub transcript: String,
    pub answer: String,
}

impl Default for Shared {
    fn default() -> Self {
        Self { stage: Stage::StartingStt, transcript: String::new(), answer: String::new() }
    }
}

pub type SharedState = Arc<Mutex<Shared>>;

/// Spawns the capture -> transcribe loop, then (once `stop_flag` is set)
/// the Claude call, updating `shared` throughout. Returns immediately;
/// everything happens on a detached thread.
pub fn start(stt_url: String, ai_cli: String, stop_flag: Arc<AtomicBool>, shared: SharedState) {
    std::thread::spawn(move || {
        if !stt::ensure_running(&stt_url) {
            let mut s = shared.lock().unwrap();
            s.stage = Stage::Failed("STT server unavailable — see stt-server/README or run scripts/install-stt.sh".to_string());
            return;
        }
        {
            let mut s = shared.lock().unwrap();
            s.stage = Stage::Recording;
        }

        let Some(mut child) = spawn_parec() else {
            let mut s = shared.lock().unwrap();
            s.stage = Stage::Failed("failed to start `parec` — is PulseAudio/PipeWire running?".to_string());
            return;
        };

        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut buf = vec![0u8; 4096];
        let mut chunk: Vec<u8> = Vec::with_capacity(CHUNK_BYTES);

        loop {
            let stop = stop_flag.load(Ordering::Relaxed);

            match stdout.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => chunk.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }

            if chunk.len() >= CHUNK_BYTES || (stop && !chunk.is_empty()) {
                let wav_bytes = wav::pcm_to_wav(&chunk, SAMPLE_RATE, CHANNELS, BITS);
                chunk.clear();
                if let Ok(text) = stt::transcribe(&stt_url, &wav_bytes) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        let mut s = shared.lock().unwrap();
                        if !s.transcript.is_empty() {
                            s.transcript.push(' ');
                        }
                        s.transcript.push_str(trimmed);
                    }
                }
            }

            if stop {
                break;
            }
        }

        let _ = child.kill();
        let _ = child.wait();

        let (transcript, cli) = {
            let mut s = shared.lock().unwrap();
            s.stage = Stage::Processing;
            (s.transcript.clone(), ai_cli.clone())
        };

        if transcript.trim().is_empty() {
            let mut s = shared.lock().unwrap();
            s.stage = Stage::Failed("no speech transcribed".to_string());
            return;
        }

        match claude::ask(&cli, &transcript) {
            Ok(answer) => {
                let mut s = shared.lock().unwrap();
                s.answer = answer;
                s.stage = Stage::Done;
            }
            Err(err) => {
                let mut s = shared.lock().unwrap();
                s.stage = Stage::Failed(err);
            }
        }
    });
}

fn spawn_parec() -> Option<Child> {
    // Same invocation shape as sway-control's audio_meter.rs (the
    // suite's other `parec` consumer): `--latency-msec=20` avoids
    // PulseAudio buffering ~2s server-side before delivering anything,
    // which would otherwise make "live" transcription anything but.
    Command::new("parec")
        .args([
            "--raw",
            "--format=s16le",
            &format!("--rate={SAMPLE_RATE}"),
            &format!("--channels={CHANNELS}"),
            "--latency-msec=20",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

/// Small helper so `app.rs` doesn't need to import `Duration` just for
/// the poll interval constant.
pub const POLL_INTERVAL: Duration = Duration::from_millis(150);

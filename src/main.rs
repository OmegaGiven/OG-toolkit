mod app;
mod claude;
mod config;
mod fonts;
mod lock;
mod pipeline;
mod placement;
mod stt;
mod wav;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use app::App;
use config::{Config, VoiceConfig};

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // `--stop` (release edge of the push-to-talk bind): signal the
    // already-running instance and exit immediately. Never starts a
    // second instance/window — see lock.rs and the README for why
    // SIGUSR1 was chosen over a socket.
    if args.iter().any(|a| a == "--stop") {
        if lock::send_stop() {
            eprintln!("og-voice: sent stop signal to running instance");
        } else {
            eprintln!("og-voice: no running instance to stop");
        }
        return Ok(());
    }

    // Press edge: refuse to start a second recording session if one is
    // already in flight. `og-voice-launch` also guards against this, but
    // guarding here too means launching `og-voice` directly (bypassing
    // the wrapper) is still safe.
    if lock::running_pid().is_some() {
        eprintln!("og-voice: already running, not starting a second instance");
        return Ok(());
    }
    lock::acquire();

    let cfg = Config::load();
    let voice_cfg = VoiceConfig::load();
    let ai_cli = cfg.default_ai_cli.clone();

    // SIGUSR1 is the stop signal from `og-voice --stop`. `signal_hook`'s
    // `flag::register` just flips an `AtomicBool` from the signal
    // handler — no async-signal-safety concerns beyond what the crate
    // already guarantees, and the capture loop in `pipeline.rs` polls it
    // between reads rather than doing anything signal-handler-unsafe
    // directly. Chosen over a Unix socket for this: a signal plus a PID
    // file needs no listener bookkeeping/accept loop at all, and the
    // kernel already guarantees exactly-once, race-free delivery to a
    // specific PID — a socket would have to reinvent both of those for
    // no benefit here, since there's never more than one (sender,
    // receiver) pair at a time.
    let stop_flag = Arc::new(AtomicBool::new(false));
    if signal_hook::flag::register(signal_hook::consts::SIGUSR1, Arc::clone(&stop_flag)).is_err() {
        eprintln!("og-voice: warning: failed to install SIGUSR1 handler; --stop will not work");
    }

    let shared = Arc::new(Mutex::new(pipeline::Shared::default()));
    pipeline::start(voice_cfg.stt_server_url.clone(), ai_cli, Arc::clone(&stop_flag), Arc::clone(&shared));

    let result = run_ui(shared, stop_flag);
    lock::release();
    result
}

fn run_ui(
    shared: pipeline::SharedState,
    stop_flag: Arc<AtomicBool>,
) -> iced::Result {
    let build = iced::application("og-voice", App::update, App::view)
        .subscription(App::subscription)
        .style(|_state, _theme| iced::application::Appearance {
            background_color: iced::Color::TRANSPARENT,
            text_color: iced::Color::WHITE,
        });

    let build = match fonts::load_default_font() {
        Some((family, bytes)) => build.font(bytes).default_font(iced::Font::with_name(family)),
        None => build,
    };

    build
        .window(iced::window::Settings {
            size: iced::Size::new(app::SIZE.0, app::SIZE.1),
            decorations: false,
            transparent: true,
            visible: false,
            position: placement::hint(app::SIZE),
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: "og-voice".to_string(),
                ..Default::default()
            },
            ..Default::default()
        })
        .run_with(move || App::new(shared.clone(), stop_flag.clone()))
}

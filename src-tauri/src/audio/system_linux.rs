use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use libpulse_binding::context::{Context, State};
use libpulse_binding::context::flags as ctx_flags;
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;
use tokio::sync::mpsc;

use super::system_capture::send_system_chunk;
use super::AudioChunk;

const RATE: u32 = 48_000;
const CHANNELS: u16 = 2;

/// Record the default sink's monitor through PulseAudio.
/// PipeWire exposes the same client API, so this is the Linux system-audio path.
pub fn run(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
    ready: &std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    let monitor = resolve_monitor(device_name.as_deref())?;
    log::info!("Linux system capture monitor: {}", monitor);

    let spec = Spec {
        format: Format::S16NE,
        channels: CHANNELS as u8,
        rate: RATE,
    };
    if !spec.is_valid() {
        return Err("Pulse sample spec is invalid".to_string());
    }

    let simple = Simple::new(
        None,
        "zaiqoM",
        Direction::Record,
        Some(&monitor),
        "system",
        &spec,
        None,
        None,
    )
    .map_err(|e| format!("Pulse monitor open failed for '{monitor}': {e}"))?;

    let _ = ready.try_send(Ok(()));
    let frame_samples = (RATE / 10) as usize * CHANNELS as usize;
    let mut bytes = vec![0u8; frame_samples * 2];

    while !stop_flag.load(Ordering::Relaxed) {
        if simple.read(&mut bytes).is_err() {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            return Err("Pulse monitor read failed".to_string());
        }
        let pcm: Vec<i16> = bytes
            .chunks_exact(2)
            .map(|c| i16::from_ne_bytes([c[0], c[1]]))
            .collect();
        send_system_chunk(&pcm, RATE, CHANNELS, &tx);
    }

    log::info!("Linux system capture stopped");
    Ok(())
}

fn resolve_monitor(device_name: Option<&str>) -> Result<String, String> {
    if let Some(name) = device_name {
        if name.ends_with(".monitor") {
            return Ok(name.to_string());
        }
        if name != "default" {
            return Ok(format!("{name}.monitor"));
        }
    }

    match default_sink_monitor() {
        Ok(name) => Ok(name),
        Err(e) => {
            log::warn!("Pulse server info failed ({e}); using @DEFAULT_MONITOR@");
            Ok("@DEFAULT_MONITOR@".to_string())
        }
    }
}

fn default_sink_monitor() -> Result<String, String> {
    let mut mainloop = Mainloop::new().ok_or_else(|| "Pulse mainloop failed".to_string())?;
    let mut context = Context::new(&mainloop, "zaiqoM").ok_or_else(|| "Pulse context failed".to_string())?;
    context
        .connect(None, ctx_flags::NOFLAGS, None)
        .map_err(|e| format!("Pulse connect failed: {e}"))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if std::time::Instant::now() > deadline {
            return Err("Pulse context timed out".to_string());
        }
        match mainloop.iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                return Err("Pulse mainloop stopped".to_string());
            }
            IterateResult::Success(_) => {}
        }
        match context.get_state() {
            State::Ready => break,
            State::Failed | State::Terminated => return Err("Pulse context failed".to_string()),
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let op = context.introspect().get_server_info(move |info| {
        if let Some(name) = info.default_sink_name.as_deref() {
            let _ = tx.send(name.to_string());
        }
    });

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let sink = loop {
        if let Ok(name) = rx.try_recv() {
            break name;
        }
        if std::time::Instant::now() > deadline {
            return Err("Pulse server info timed out".to_string());
        }
        match mainloop.iterate(true) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                return Err("Pulse mainloop stopped while reading the default sink".to_string());
            }
            IterateResult::Success(_) => {}
        }
        if op.get_state() == libpulse_binding::operation::State::Done {
            if let Ok(name) = rx.try_recv() {
                break name;
            }
            return Err("Pulse reported no default sink".to_string());
        }
    };
    Ok(format!("{sink}.monitor"))
}

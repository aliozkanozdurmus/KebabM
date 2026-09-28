// System audio capture via cpal loopback.
// Captures audio from a specific output device by building an input stream
// on it — cpal's WASAPI backend handles the loopback flag automatically.
// This approach works with Bluetooth, USB, and virtual audio devices.

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::resampler::resample;
use super::{AudioChunk, AudioSource};

const TARGET_SAMPLE_RATE: u32 = 16000;

/// Start system audio loopback capture using cpal.
/// Uses the default output device.
pub fn start_system_capture(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
) -> Result<std::thread::JoinHandle<()>, String> {
    start_system_capture_device(tx, stop_flag, None)
}

/// Start system audio loopback capture on a specific output device.
/// If device_name is None, uses the default output device.
///
/// Works by calling build_input_stream() on an output device —
/// cpal's WASAPI backend automatically sets the loopback capture flag.
pub fn start_system_capture_device(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
) -> Result<std::thread::JoinHandle<()>, String> {
    let label = device_name.as_deref().unwrap_or("default").to_string();
    log::info!("Starting system audio capture on: {}", label);

    // Everything runs inside the spawned thread because cpal::Stream is !Send.
    // The stream must be created and kept alive on the same thread.
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let stop_on_failure = Arc::clone(&stop_flag);
    let handle = std::thread::Builder::new()
        .name("system-audio-capture".into())
        .spawn(move || {
            let result = run_platform(tx, stop_flag, device_name, &ready_tx);
            if let Err(e) = result {
                let _ = ready_tx.try_send(Err(e.clone()));
                log::error!("System audio capture failed: {}", e);
            }
        })
        .map_err(|e| format!("Failed to spawn system capture thread: {}", e))?;

    match ready_rx.recv_timeout(std::time::Duration::from_secs(15)) {
        Ok(Ok(())) => Ok(handle),
        outcome => {
            stop_on_failure.store(true, Ordering::SeqCst);
            // Do not join an OS permission call that may still be waiting on the user.
            Err(match outcome {
                Ok(Err(error)) => error,
                _ => "System audio did not become ready within 15 seconds. Check capture permissions and retry.".into(),
            })
        }
    }
}

fn run_platform(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
    ready: &std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        return run_cpal_loopback(tx, stop_flag, device_name, ready);
    }
    #[cfg(target_os = "linux")]
    {
        return super::system_linux::run(tx, stop_flag, device_name, ready);
    }
    #[cfg(target_os = "macos")]
    {
        return super::system_macos::run(tx, stop_flag, device_name, ready);
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        let _ = (tx, stop_flag, device_name, ready);
        Err("System audio capture is not available on this operating system".to_string())
    }
}

#[cfg(target_os = "windows")]
fn run_cpal_loopback(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
    ready: &std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    let host = cpal::default_host();

    let device = if let Some(ref name) = device_name {
        let mut found = None;
        if let Ok(devices) = host.output_devices() {
            for d in devices {
                if let Ok(d_name) = d.description().map(|description| description.name().to_owned()) {
                    log::info!("  Output device: {}", d_name);
                    if d_name == *name {
                        found = Some(d);
                        break;
                    }
                }
            }
        }
        match found {
            Some(d) => d,
            None => {
                log::warn!("Output device '{}' not found, using default", name);
                host.default_output_device()
                    .ok_or_else(|| "No default output device".to_string())?
            }
        }
    } else {
        host.default_output_device()
            .ok_or_else(|| "No default output device".to_string())?
    };

    let actual_name = device.description().map(|description| description.name().to_owned()).unwrap_or_else(|_| "unknown".into());
    log::info!("System loopback device: {}", actual_name);

    let config = device
        .default_output_config()
        .map_err(|e| format!("No output config for '{}': {}", actual_name, e))?;

    let sample_rate = config.sample_rate();
    let channels = config.channels();
    let sample_format = config.sample_format();
    log::info!("System capture: {}Hz, {}ch, {:?}", sample_rate, channels, sample_format);

    let stop = stop_flag.clone();
    // Building an input stream on a WASAPI output device enables loopback.
    let stream = super::mic_capture::build_pcm_stream(&device, config, move |pcm| {
        if !stop.load(Ordering::Relaxed) {
            send_system_chunk(pcm, sample_rate, channels, &tx);
        }
    })?;

    stream.play().map_err(|e| format!("Failed to play loopback stream: {}", e))?;
    let _ = ready.try_send(Ok(()));
    log::info!("System audio loopback ACTIVE on '{}'", actual_name);

    // Keep stream alive until stop flag
    while !stop_flag.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    drop(stream);
    log::info!("System audio loopback stopped");
    Ok(())
}

pub(crate) fn send_system_chunk(
    i16_data: &[i16],
    sample_rate: u32,
    channels: u16,
    tx: &mpsc::Sender<AudioChunk>,
) {
    if i16_data.is_empty() {
        return;
    }

    let pcm_data = resample(i16_data, sample_rate, TARGET_SAMPLE_RATE, channels);

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let chunk = AudioChunk {
        pcm_data,
        source: AudioSource::System,
        timestamp_ms,
        is_speech: false,
    };

    if tx.try_send(chunk).is_err() {
        // Channel full — drop chunk silently
    }
}

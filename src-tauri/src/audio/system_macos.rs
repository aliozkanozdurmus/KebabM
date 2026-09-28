use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use screencapturekit::prelude::*;
use tokio::sync::mpsc;

use super::system_capture::send_system_chunk;
use super::AudioChunk;

const RATE: u32 = 48_000;

struct AudioHandler {
    sender: std::sync::mpsc::SyncSender<Vec<i16>>,
}

impl SCStreamOutputTrait for AudioHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, output_type: SCStreamOutputType) {
        if output_type != SCStreamOutputType::Audio {
            return;
        }
        let Ok(list) = sample.audio_buffer_list() else {
            return;
        };
        let Some(buffer) = list.iter().next() else {
            return;
        };
        let raw = buffer.data();
        if raw.len() < 4 {
            return;
        }
        // CoreAudio byte buffers need not be aligned as Rust f32 slices.
        let pcm: Vec<i16> = raw.chunks_exact(4)
            .map(|bytes| f32::from_ne_bytes(bytes.try_into().unwrap()))
            .map(|value| (value.clamp(-1.0, 1.0) * 32768.0) as i16)
            .collect();
        let _ = self.sender.try_send(pcm);
    }
}

/// Capture system audio with ScreenCaptureKit. macOS asks for screen-recording permission.
pub fn run(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    _device_name: Option<String>,
    ready: &std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    let content = SCShareableContent::get().map_err(|e| {
        format!("ScreenCaptureKit needs screen recording permission: {e}")
    })?;
    let display = content
        .displays()
        .into_iter()
        .next()
        .ok_or_else(|| "No display available for system audio".to_string())?;

    let filter = SCContentFilter::create()
        .with_display(&display)
        .with_excluding_windows(&[])
        .build().map_err(|e| format!("ScreenCaptureKit filter: {e}"))?;
    let config = SCStreamConfiguration::new()
        .with_width(2)
        .with_height(2)
        .with_captures_audio(true)
        .with_excludes_current_process_audio(true)
        .with_sample_rate(RATE as i32)
        .with_channel_count(1);

    let (pcm_tx, pcm_rx) = std::sync::mpsc::sync_channel(32);
    let mut stream = SCStream::new(&filter, &config).map_err(|e| format!("ScreenCaptureKit stream: {e}"))?;
    stream.add_output_handler(AudioHandler { sender: pcm_tx }, SCStreamOutputType::Audio).map_err(|e| format!("ScreenCaptureKit audio handler: {e}"))?;
    stream
        .start_capture()
        .map_err(|e| format!("ScreenCaptureKit start failed: {e}"))?;
    let _ = ready.try_send(Ok(()));
    log::info!("macOS system audio capture active");

    while !stop_flag.load(Ordering::Relaxed) {
        match pcm_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(pcm) => send_system_chunk(&pcm, RATE, 1, &tx),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = stream.stop_capture();
    log::info!("macOS system audio capture stopped");
    Ok(())
}

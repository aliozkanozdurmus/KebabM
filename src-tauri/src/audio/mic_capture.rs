// Sub-PRD 3: Microphone capture via cpal
// 16kHz, 16-bit mono PCM, sends via tokio::sync::mpsc

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use super::input_stream::InputStream;
use super::resampler::resample;
use super::{AudioChunk, AudioSource};
use std::sync::atomic::Ordering;

/// Target sample rate for all audio output
const TARGET_SAMPLE_RATE: u32 = 16000;

/// Start capturing audio from the specified input device.
/// Returns an owner-thread handle — dropping it requests stream shutdown.
///
/// Audio is resampled to 16kHz mono 16-bit PCM and sent as AudioChunk
/// through the provided mpsc channel. The `source` tag determines whether
/// chunks are labeled as Mic or System (for input devices used as "Them").
pub fn start_mic_capture(
    device_id: &str,
    tx: mpsc::Sender<AudioChunk>,
    source: AudioSource,
) -> Result<InputStream, String> {
    let device_id = device_id.to_string();
    InputStream::open(move |active| {
        let device = super::device_manager::find_input_device(&device_id)?;
        let config = device
            .default_input_config()
            .map_err(|e| format!("Input configuration: {e}"))?;
        let rate = config.sample_rate();
        let channels = config.channels();
        let callback_active = active.clone();
        let stream = build_pcm_stream(&device, config, move |data| {
            if !callback_active.load(Ordering::SeqCst) {
                return;
            }
            handle_mic_data_i16(data, rate, channels, &tx, &source);
        })?;
        if !active.load(Ordering::SeqCst) {
            return Err("Microphone opening cancelled".into());
        }
        stream
            .play()
            .map_err(|e| format!("Start microphone: {e}"))?;
        Ok(stream)
    })
}

/// One hardware stream when both roles share an input device.
pub fn start_mic_capture_dual(
    device_id: &str,
    tx: mpsc::Sender<AudioChunk>,
) -> Result<InputStream, String> {
    let device_id = device_id.to_string();
    InputStream::open(move |active| {
        let device = super::device_manager::find_input_device(&device_id)?;
        let config = device
            .default_input_config()
            .map_err(|e| format!("Input configuration: {e}"))?;
        let rate = config.sample_rate();
        let channels = config.channels();
        let callback_active = active.clone();
        let stream = build_pcm_stream(&device, config, move |data| {
            if !callback_active.load(Ordering::SeqCst) {
                return;
            }
            if data.is_empty() {
                return;
            }
            let pcm_data = resample(data, rate, TARGET_SAMPLE_RATE, channels);
            let timestamp_ms = current_timestamp_ms();
            for source in [AudioSource::Mic, AudioSource::System] {
                let _ = tx.try_send(AudioChunk {
                    pcm_data: pcm_data.clone(),
                    source,
                    timestamp_ms,
                    is_speech: false,
                });
            }
        })?;
        if !active.load(Ordering::SeqCst) {
            return Err("Microphone opening cancelled".into());
        }
        stream
            .play()
            .map_err(|e| format!("Start shared microphone: {e}"))?;
        Ok(stream)
    })
}

/// CPAL may select integer or floating PCM formats as the device default.
/// Share conversion between microphone and Windows output-device loopback.
pub(crate) fn build_pcm_stream(
    device: &cpal::Device,
    config: cpal::SupportedStreamConfig,
    mut on_data: impl FnMut(&[i16]) + Send + 'static,
) -> Result<Stream, String> {
    macro_rules! capture {
        ($sample:ty) => {
            device.build_input_stream(
                config.into(),
                move |data: &[$sample], _: &cpal::InputCallbackInfo| {
                    let pcm = to_pcm16(data);
                    on_data(&pcm);
                },
                |error| log::error!("Audio stream failed: {error}"),
                None,
            )
        };
    }
    let stream = match config.sample_format() {
        SampleFormat::I8 => capture!(i8),
        SampleFormat::I16 => capture!(i16),
        SampleFormat::I24 => capture!(cpal::I24),
        SampleFormat::I32 => capture!(i32),
        SampleFormat::I64 => capture!(i64),
        SampleFormat::U8 => capture!(u8),
        SampleFormat::U16 => capture!(u16),
        SampleFormat::U24 => capture!(cpal::U24),
        SampleFormat::U32 => capture!(u32),
        SampleFormat::U64 => capture!(u64),
        SampleFormat::F32 => capture!(f32),
        SampleFormat::F64 => capture!(f64),
        format => return Err(format!("Unsupported audio format: {format:?}")),
    };
    stream.map_err(|e| format!("Build audio stream: {e}"))
}

fn to_pcm16<T: cpal::Sample>(data: &[T]) -> Vec<i16>
where
    f64: cpal::FromSample<T>,
{
    data.iter()
        .map(|sample| {
            let value = sample.to_sample::<f64>();
            // Saturate out-of-range floating input; NaN becomes silence.
            if value.is_nan() {
                0
            } else {
                (value.clamp(-1.0, 1.0) * 32768.0) as i16
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_formats_preserve_silence_polarity_and_saturation() {
        assert_eq!(
            to_pcm16(&[i16::MIN, -1, 0, 1, i16::MAX]),
            vec![i16::MIN, -1, 0, 1, i16::MAX]
        );
        assert_eq!(
            to_pcm16(&[0_u16, 32768, 65535]),
            vec![i16::MIN, 0, i16::MAX]
        );
        assert_eq!(
            to_pcm16(&[i32::MIN, 0, i32::MAX]),
            vec![i16::MIN, 0, i16::MAX]
        );
        assert_eq!(
            to_pcm16(&[-2.0_f32, 0.0, 0.5, 2.0, f32::NAN]),
            vec![i16::MIN, 0, 16384, i16::MAX, 0]
        );
        assert_eq!(
            to_pcm16(&[
                cpal::I24::new(-8388608).unwrap(),
                cpal::I24::new(0).unwrap(),
                cpal::I24::new(8388607).unwrap()
            ]),
            vec![i16::MIN, 0, i16::MAX]
        );
    }
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn handle_mic_data_i16(
    data: &[i16],
    sample_rate: u32,
    channels: u16,
    tx: &mpsc::Sender<AudioChunk>,
    source: &AudioSource,
) {
    if data.is_empty() {
        return;
    }

    let pcm_data = resample(data, sample_rate, TARGET_SAMPLE_RATE, channels);

    let chunk = AudioChunk {
        pcm_data,
        source: source.clone(),
        timestamp_ms: current_timestamp_ms(),
        is_speech: false,
    };

    // Non-blocking send — drop chunk if channel is full
    if tx.try_send(chunk).is_err() {
        log::trace!("Mic audio channel full, dropping chunk");
    }
}

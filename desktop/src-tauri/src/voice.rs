//! Native microphone capture for speech-to-text.
//!
//! A cpal input `Stream` is `!Send`, so capture runs on a dedicated OS thread.
//! Samples are downmixed to mono `f32` while recording; on stop they are encoded
//! to 16-bit PCM WAV in memory and handed to the transcription endpoint.

use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};

/// Mono PCM audio captured from the input device.
pub struct RecordedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

/// Handle to an in-progress recording. Set `stop` to true, then `recv` the
/// captured audio (or an error string) from the capture thread.
pub struct RecordingHandle {
    pub stop: Arc<AtomicBool>,
    pub done: Receiver<Result<RecordedAudio, String>>,
}

/// Spawn the capture thread and return its control handle.
pub fn start_capture() -> Result<RecordingHandle, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, done) = std::sync::mpsc::channel();

    let stop_thread = stop.clone();
    std::thread::Builder::new()
        .name("audio-capture".into())
        .spawn(move || {
            let _ = tx.send(capture_loop(stop_thread));
        })
        .map_err(|e| format!("failed to start capture thread: {e}"))?;

    Ok(RecordingHandle { stop, done })
}

fn capture_loop(stop: Arc<AtomicBool>) -> Result<RecordedAudio, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("no microphone / input device found")?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("no default input config: {e}"))?;

    let sample_rate = supported.sample_rate();
    let channels = supported.channels() as usize;
    let config: cpal::StreamConfig = supported.clone().into();

    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let err_fn = |e| eprintln!("audio input stream error: {e}");

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            build_stream::<f32>(&device, &config, samples.clone(), channels, err_fn)
        }
        cpal::SampleFormat::I16 => {
            build_stream::<i16>(&device, &config, samples.clone(), channels, err_fn)
        }
        cpal::SampleFormat::U16 => {
            build_stream::<u16>(&device, &config, samples.clone(), channels, err_fn)
        }
        other => Err(format!("unsupported sample format: {other:?}")),
    }?;

    stream.play().map_err(|e| format!("failed to start stream: {e}"))?;
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(stream);

    let captured = samples.lock().unwrap().clone();
    Ok(RecordedAudio {
        samples: captured,
        sample_rate,
    })
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    channels: usize,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut buf = samples.lock().unwrap();
                for frame in data.chunks(channels.max(1)) {
                    let sum: f32 = frame.iter().map(|&s| f32::from_sample(s)).sum();
                    buf.push(sum / frame.len() as f32);
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("failed to build input stream: {e}"))
}

/// Encode mono `f32` samples to 16-bit PCM WAV bytes.
pub fn encode_wav(audio: &RecordedAudio) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::<u8>::new());
    {
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).map_err(|e| format!("wav init: {e}"))?;
        for &s in &audio.samples {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer
                .write_sample(v)
                .map_err(|e| format!("wav write: {e}"))?;
        }
        writer.finalize().map_err(|e| format!("wav finalize: {e}"))?;
    }
    Ok(cursor.into_inner())
}

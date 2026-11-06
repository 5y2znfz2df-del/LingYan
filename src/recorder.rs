//! Microphone recorder with simple energy VAD.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

pub fn record_until_silence() -> Result<Vec<i16>, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "没有找到麦克风".to_owned())?;
    let supported = device
        .default_input_config()
        .map_err(|error| format!("读取麦克风配置失败：{error}"))?;
    let sample_format = supported.sample_format();
    let config = supported.config();
    let channels = config.channels.max(1) as usize;
    let sample_rate = config.sample_rate.0;

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let error_fn = |_error| {};

    let stream = match sample_format {
        SampleFormat::F32 => {
            let buffer = buffer.clone();
            device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    let mut sink = buffer.lock().unwrap();
                    for frame in data.chunks(channels) {
                        let sum: f32 = frame.iter().copied().sum();
                        sink.push(sum / channels as f32);
                    }
                },
                error_fn,
                None,
            )
        }
        SampleFormat::I16 => {
            let buffer = buffer.clone();
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let mut sink = buffer.lock().unwrap();
                    for frame in data.chunks(channels) {
                        let sum: f32 = frame.iter().map(|s| *s as f32 / 32768.0).sum();
                        sink.push(sum / channels as f32);
                    }
                },
                error_fn,
                None,
            )
        }
        SampleFormat::U16 => {
            let buffer = buffer.clone();
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    let mut sink = buffer.lock().unwrap();
                    for frame in data.chunks(channels) {
                        let sum: f32 = frame
                            .iter()
                            .map(|s| (*s as f32 - 32768.0) / 32768.0)
                            .sum();
                        sink.push(sum / channels as f32);
                    }
                },
                error_fn,
                None,
            )
        }
        SampleFormat::I32 => {
            let buffer = buffer.clone();
            device.build_input_stream(
                &config,
                move |data: &[i32], _| {
                    let mut sink = buffer.lock().unwrap();
                    for frame in data.chunks(channels) {
                        let sum: f32 = frame
                            .iter()
                            .map(|s| *s as f32 / 2_147_483_648.0)
                            .sum();
                        sink.push(sum / channels as f32);
                    }
                },
                error_fn,
                None,
            )
        }
        other => return Err(format!("不支持的麦克风格式：{other:?}")),
    }
    .map_err(|error| format!("打开麦克风失败：{error}"))?;

    stream
        .play()
        .map_err(|error| format!("启动录音失败：{error}"))?;

    let start = Instant::now();
    let mut speech = false;
    let mut last_voice = Instant::now();
    let mut noise = 0.01f32;
    let mut cursor = 0usize;

    loop {
        std::thread::sleep(Duration::from_millis(50));
        let (rms, _len) = {
            let sink = buffer.lock().unwrap();
            if sink.len() <= cursor {
                continue;
            }
            let slice = &sink[cursor..];
            cursor = sink.len();
            let energy: f32 = slice.iter().map(|s| s * s).sum();
            ((energy / slice.len().max(1) as f32).sqrt(), sink.len())
        };

        if start.elapsed() < Duration::from_millis(350) {
            noise = noise * 0.8 + rms * 0.2;
            continue;
        }
        let threshold = (noise * 3.0).max(0.012);
        if rms > threshold {
            speech = true;
            last_voice = Instant::now();
        }
        if speech && last_voice.elapsed() > Duration::from_millis(1_500) {
            break;
        }
        if start.elapsed() > Duration::from_secs(15) {
            break;
        }
        if !speech && start.elapsed() > Duration::from_secs(8) {
            break;
        }
    }

    drop(stream);
    let samples = buffer.lock().unwrap().clone();
    if samples.len() < (sample_rate as usize / 2) {
        return Err("没有录到声音".to_owned());
    }

    let mut resampled = resample(&samples, sample_rate, 16_000);
    let peak = resampled
        .iter()
        .fold(0.0f32, |acc, value| acc.max(value.abs()));
    if peak > 0.001 && peak < 0.12 {
        let gain = (0.35 / peak).clamp(1.0, 12.0);
        for sample in &mut resampled {
            *sample *= gain;
        }
    }
    Ok(resampled
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * 32_767.0) as i16)
        .collect())
}

fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let out_len = (input.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(out_len);
    for index in 0..out_len {
        let position = index as f64 * ratio;
        let base = position.floor() as usize;
        let frac = (position - base as f64) as f32;
        let a = input.get(base).copied().unwrap_or(0.0);
        let b = input.get(base + 1).copied().unwrap_or(a);
        output.push(a + (b - a) * frac);
    }
    output
}

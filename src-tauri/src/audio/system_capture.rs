// System audio capture via Windows WASAPI loopback.
//
// Windows render endpoints (speakers, wired headsets, Bluetooth headsets and
// USB audio devices) are opened as shared-mode loopback capture clients. The
// selected device is resolved by friendly name for backwards compatibility;
// a Windows endpoint ID is also accepted when a newer config provides one.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

#[cfg(not(target_os = "windows"))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::resampler::resample;
use super::{AudioChunk, AudioSource};

const TARGET_SAMPLE_RATE: u32 = 16000;

/// Start system audio loopback capture.
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
/// Windows uses a native WASAPI shared loopback client; other platforms use
/// the cpal loopback implementation.
pub fn start_system_capture_device(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
) -> Result<std::thread::JoinHandle<()>, String> {
    start_system_capture_device_with_error(tx, stop_flag, device_name, None)
}

/// Start loopback capture and flip `error_flag` when the cpal stream reports a
/// terminal error. The meeting recovery loop uses this to rebuild the stream.
pub fn start_system_capture_device_with_error(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
    error_flag: Option<Arc<AtomicBool>>,
) -> Result<std::thread::JoinHandle<()>, String> {
    let label = device_name.as_deref().unwrap_or("default").to_string();
    log::info!("Starting system audio capture on: {}", label);

    // Report initialization synchronously. Previously the caller was told that
    // capture had started even when the worker failed immediately, which made
    // headset failures look like a silent transcription problem.
    let (startup_tx, startup_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let startup_stop_flag = stop_flag.clone();

    // Everything runs inside the spawned thread because the native audio
    // client must be kept alive on the same thread.
    let handle = std::thread::Builder::new()
        .name("system-audio-capture".into())
        .spawn(move || {
            let thread_error_flag = error_flag.clone();
            if let Err(e) = run_loopback(tx, stop_flag, device_name, error_flag, startup_tx.clone())
            {
                let _ = startup_tx.send(Err(e.clone()));
                if let Some(flag) = thread_error_flag {
                    flag.store(true, Ordering::SeqCst);
                }
                log::error!("System audio capture failed: {}", e);
            }
        })
        .map_err(|e| format!("Failed to spawn system capture thread: {}", e))?;

    match startup_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(handle),
        Ok(Err(error)) => {
            startup_stop_flag.store(true, Ordering::SeqCst);
            let _ = handle.join();
            Err(error)
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            startup_stop_flag.store(true, Ordering::SeqCst);
            let _ = handle.join();
            Err(format!("Timed out opening system audio output '{}'", label))
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            startup_stop_flag.store(true, Ordering::SeqCst);
            let _ = handle.join();
            Err(format!(
                "System audio output '{}' stopped during startup",
                label
            ))
        }
    }
}

#[cfg(target_os = "windows")]
fn run_loopback(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
    error_flag: Option<Arc<AtomicBool>>,
    startup_tx: std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    run_wasapi_loopback(tx, stop_flag, device_name, error_flag, startup_tx)
}

#[cfg(not(target_os = "windows"))]
fn run_loopback(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
    error_flag: Option<Arc<AtomicBool>>,
    startup_tx: std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    run_cpal_loopback(tx, stop_flag, device_name, error_flag, startup_tx)
}

#[cfg(not(target_os = "windows"))]
fn run_cpal_loopback(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
    error_flag: Option<Arc<AtomicBool>>,
    startup_tx: std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    let host = cpal::default_host();

    let device = if let Some(ref name) = device_name {
        let mut found = None;
        if let Ok(devices) = host.output_devices() {
            for d in devices {
                if let Ok(d_name) = d.name() {
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
            None => return Err(format!("Output device '{}' not found", name)),
        }
    } else {
        host.default_output_device()
            .ok_or_else(|| "No default output device".to_string())?
    };

    let actual_name = device.name().unwrap_or_else(|_| "unknown".into());
    log::info!("System loopback device: {}", actual_name);

    let config = device
        .default_output_config()
        .map_err(|e| format!("No output config for '{}': {}", actual_name, e))?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    let sample_format = config.sample_format();
    log::info!(
        "System capture: {}Hz, {}ch, {:?}",
        sample_rate,
        channels,
        sample_format
    );

    let err_flag = error_flag.clone();
    let err_fn = move |err: cpal::StreamError| {
        if let Some(flag) = &err_flag {
            flag.store(true, Ordering::SeqCst);
        }
        log::error!("System capture error: {}", err);
    };

    let stop = stop_flag.clone();
    let tx2 = tx.clone();

    // Build INPUT stream on OUTPUT device = loopback capture
    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                let pcm: Vec<i16> = data
                    .iter()
                    .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                    .collect();
                send_system_chunk(&pcm, sample_rate, channels, &tx2);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                send_system_chunk(data, sample_rate, channels, &tx2);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                let pcm: Vec<i16> = data.iter().map(|&s| (s as i32 - 32768) as i16).collect();
                send_system_chunk(&pcm, sample_rate, channels, &tx2);
            },
            err_fn,
            None,
        ),
        _ => return Err(format!("Unsupported format: {:?}", sample_format)),
    }
    .map_err(|e| {
        format!(
            "Failed to build loopback stream on '{}': {}",
            actual_name, e
        )
    })?;

    stream
        .play()
        .map_err(|e| format!("Failed to play loopback stream: {}", e))?;
    log::info!("System audio loopback ACTIVE on '{}'", actual_name);
    let _ = startup_tx.send(Ok(()));

    // Keep stream alive until stop flag
    while !stop_flag.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    drop(stream);
    log::info!("System audio loopback stopped");
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_wasapi_loopback(
    tx: mpsc::Sender<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
    device_name: Option<String>,
    _error_flag: Option<Arc<AtomicBool>>,
    startup_tx: std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    let com_hr = wasapi::initialize_mta();
    let should_deinitialize = com_hr.0 == 0 || com_hr.0 == 1;
    if com_hr.is_err() && com_hr.0 as u32 != 0x8001_0106 {
        return Err(format!(
            "WASAPI COM initialization failed: 0x{:08X}",
            com_hr.0
        ));
    }

    let result = (|| -> Result<(), String> {
        use wasapi::{DeviceEnumerator, Direction, StreamMode};

        let enumerator = DeviceEnumerator::new()
            .map_err(|e| format!("WASAPI device enumerator failed: {}", e))?;

        let device = match device_name
            .as_deref()
            .filter(|name| !name.is_empty() && *name != "default")
        {
            None => enumerator
                .get_default_device(&Direction::Render)
                .map_err(|e| format!("No default output device: {}", e))?,
            Some(requested) => {
                // Newer configurations may contain the stable MMDevice endpoint
                // ID; older configurations contain the friendly name.
                match enumerator.get_device(requested) {
                    Ok(device) => device,
                    Err(id_error) => enumerator
                        .get_device_collection(&Direction::Render)
                        .and_then(|devices| devices.get_device_with_name(requested))
                        .map_err(|name_error| {
                            format!(
                                "Output device '{}' not found by endpoint ID ({}) or name ({})",
                                requested, id_error, name_error
                            )
                        })?,
                }
            }
        };

        let actual_name = device
            .get_friendlyname()
            .map_err(|e| format!("Could not read selected output name: {}", e))?;
        let endpoint_id = device
            .get_id()
            .map_err(|e| format!("Could not read selected output endpoint ID: {}", e))?;
        let mut audio_client = device
            .get_iaudioclient()
            .map_err(|e| format!("Could not open output endpoint '{}': {}", actual_name, e))?;
        let format = audio_client
            .get_mixformat()
            .map_err(|e| format!("Could not read output format for '{}': {}", actual_name, e))?;
        let sample_rate = format.get_samplespersec();
        let channels = format.get_nchannels();
        let block_align = format.get_blockalign() as usize;
        let bits_per_sample = format.get_bitspersample();
        let valid_bits = format.get_validbitspersample();
        let sample_type = format
            .get_subformat()
            .map_err(|e| format!("Unsupported output format for '{}': {}", actual_name, e))?;

        if sample_rate == 0 || channels == 0 || block_align == 0 {
            return Err(format!(
                "Output device '{}' returned an invalid audio format",
                actual_name
            ));
        }

        log::info!(
            "WASAPI loopback target: '{}' (endpoint='{}', {}Hz, {}ch, {} bits/{}, {:?})",
            actual_name,
            endpoint_id,
            sample_rate,
            channels,
            bits_per_sample,
            valid_bits,
            sample_type
        );

        let mode = StreamMode::PollingShared {
            autoconvert: true,
            buffer_duration_hns: 200_000,
        };
        audio_client
            .initialize_client(&format, &Direction::Capture, &mode)
            .map_err(|e| format!("Could not initialize loopback on '{}': {}", actual_name, e))?;
        let capture = audio_client.get_audiocaptureclient().map_err(|e| {
            format!(
                "Could not create loopback capture client for '{}': {}",
                actual_name, e
            )
        })?;
        audio_client
            .start_stream()
            .map_err(|e| format!("Could not start loopback on '{}': {}", actual_name, e))?;

        log::info!("WASAPI loopback ACTIVE on '{}'", actual_name);
        let _ = startup_tx.send(Ok(()));

        while !stop_flag.load(Ordering::Relaxed) {
            let mut read_any = false;
            loop {
                let frames = capture
                    .get_next_packet_size()
                    .map_err(|e| {
                        format!("Loopback packet query failed on '{}': {}", actual_name, e)
                    })?
                    .unwrap_or(0);
                if frames == 0 {
                    break;
                }

                let mut bytes = vec![0u8; frames as usize * block_align];
                let (read_frames, buffer_info) =
                    capture.read_from_device(&mut bytes).map_err(|e| {
                        format!("Loopback buffer read failed on '{}': {}", actual_name, e)
                    })?;
                if read_frames == 0 {
                    continue;
                }

                let pcm = if buffer_info.flags.silent {
                    vec![0i16; read_frames as usize * channels as usize]
                } else {
                    decode_wasapi_samples(
                        &bytes[..read_frames as usize * block_align],
                        channels,
                        block_align,
                        sample_type,
                        bits_per_sample,
                        valid_bits,
                    )?
                };
                send_system_chunk(&pcm, sample_rate, channels, &tx);
                read_any = true;
            }

            if !read_any {
                std::thread::sleep(std::time::Duration::from_millis(4));
            }
        }

        let _ = audio_client.stop_stream();
        log::info!("WASAPI loopback stopped for '{}'", actual_name);
        Ok(())
    })();

    if should_deinitialize {
        wasapi::deinitialize();
    }
    result
}

#[cfg(target_os = "windows")]
fn decode_wasapi_samples(
    bytes: &[u8],
    channels: u16,
    block_align: usize,
    sample_type: wasapi::SampleType,
    bits_per_sample: u16,
    valid_bits: u16,
) -> Result<Vec<i16>, String> {
    if channels == 0 || block_align == 0 || bytes.len() % block_align != 0 {
        return Err("WASAPI returned an invalid interleaved audio buffer".to_string());
    }

    let channels = channels as usize;
    let bytes_per_sample = block_align / channels;
    if bytes_per_sample == 0 {
        return Err("WASAPI returned zero bytes per sample".to_string());
    }

    let frames = bytes.len() / block_align;
    let mut mono = Vec::with_capacity(frames);
    for frame in bytes.chunks_exact(block_align) {
        let mut sum = 0i64;
        for sample in frame.chunks_exact(bytes_per_sample) {
            let value = match sample_type {
                wasapi::SampleType::Float => match bytes_per_sample {
                    4 => {
                        f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]])
                            .clamp(-1.0, 1.0)
                            * i16::MAX as f32
                    }
                    8 => {
                        f64::from_le_bytes([
                            sample[0], sample[1], sample[2], sample[3], sample[4], sample[5],
                            sample[6], sample[7],
                        ])
                        .clamp(-1.0, 1.0) as f32
                            * i16::MAX as f32
                    }
                    _ => {
                        return Err(format!(
                            "Unsupported WASAPI float sample width: {} bytes",
                            bytes_per_sample
                        ));
                    }
                },
                wasapi::SampleType::Int => match bytes_per_sample {
                    1 => (sample[0] as i16 - 128) as f32 * 256.0,
                    2 => i16::from_le_bytes([sample[0], sample[1]]) as f32,
                    3 => {
                        let raw = (sample[0] as i32)
                            | ((sample[1] as i32) << 8)
                            | ((sample[2] as i32) << 16);
                        let signed = if raw & 0x0080_0000 != 0 {
                            raw | !0x00FF_FFFF
                        } else {
                            raw
                        };
                        (signed >> 8) as f32
                    }
                    4 => {
                        let raw = i32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]);
                        let shift = valid_bits
                            .saturating_sub(16)
                            .max(bits_per_sample.saturating_sub(16));
                        (raw >> shift.min(31)) as f32
                    }
                    _ => {
                        return Err(format!(
                            "Unsupported WASAPI integer sample width: {} bytes",
                            bytes_per_sample
                        ));
                    }
                },
            };
            sum += value.round() as i64;
        }
        mono.push((sum / channels as i64).clamp(i16::MIN as i64, i16::MAX as i64) as i16);
    }
    Ok(mono)
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::decode_wasapi_samples;

    #[test]
    fn downmixes_float_loopback_frames_to_mono() {
        let bytes = [
            0.5f32.to_le_bytes(),
            (-0.5f32).to_le_bytes(),
            1.0f32.to_le_bytes(),
            1.0f32.to_le_bytes(),
        ]
        .concat();

        let decoded = decode_wasapi_samples(&bytes, 2, 8, wasapi::SampleType::Float, 32, 32)
            .expect("float loopback should decode");

        assert_eq!(decoded, vec![0, i16::MAX]);
    }
}

fn send_system_chunk(
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

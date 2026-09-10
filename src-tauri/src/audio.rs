use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

pub struct AudioInput {
    pub stream: Stream,
    pub level: Arc<Mutex<f32>>,
}

#[derive(Default)]
pub struct AudioState {
    pub stream: Mutex<Option<AudioInput>>,
}

pub fn start_microphone() -> Result<AudioInput, String> {
    let host = cpal::default_host();

    let device = host
        .default_input_device()
        .ok_or_else(|| "No microphone input device found".to_string())?;

    let supported_config = device
        .default_input_config()
        .map_err(|error| {
            format!("Failed to get microphone config: {error}")
        })?;

    let config: StreamConfig = supported_config.clone().into();

    let level = Arc::new(Mutex::new(0.0_f32));
    let level_for_callback = Arc::clone(&level);

    let err_fn = |error| {
        eprintln!("[JARVIS AUDIO] stream error: {error}");
    };

    let stream = match supported_config.sample_format() {
        SampleFormat::F32 => device
            .build_input_stream(
                &config,
                move |data: &[f32], _| {
                    update_level(data, &level_for_callback);
                },
                err_fn,
                None,
            )
            .map_err(|error| {
                format!("Failed to build F32 input stream: {error}")
            })?,

        SampleFormat::I16 => {
            let level_for_callback = Arc::clone(&level);

            device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let samples: Vec<f32> = data
                            .iter()
                            .map(|sample| {
                                *sample as f32 / i16::MAX as f32
                            })
                            .collect();

                        update_level(
                            &samples,
                            &level_for_callback,
                        );
                    },
                    err_fn,
                    None,
                )
                .map_err(|error| {
                    format!(
                        "Failed to build I16 input stream: {error}"
                    )
                })?
        }

        SampleFormat::U16 => {
            let level_for_callback = Arc::clone(&level);

            device
                .build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        let samples: Vec<f32> = data
                            .iter()
                            .map(|sample| {
                                (*sample as f32 - 32768.0)
                                    / 32768.0
                            })
                            .collect();

                        update_level(
                            &samples,
                            &level_for_callback,
                        );
                    },
                    err_fn,
                    None,
                )
                .map_err(|error| {
                    format!(
                        "Failed to build U16 input stream: {error}"
                    )
                })?
        }

        unsupported => {
            return Err(format!(
                "Unsupported microphone sample format: {unsupported:?}"
            ));
        }
    };

    stream
        .play()
        .map_err(|error| {
            format!(
                "Failed to start microphone stream: {error}"
            )
        })?;

    Ok(AudioInput { stream, level })
}

pub fn get_level(input: &AudioInput) -> Result<f32, String> {
    input
        .level
        .lock()
        .map(|level| *level)
        .map_err(|_| {
            "Microphone level lock failed".to_string()
        })
}

fn update_level(samples: &[f32], level: &Arc<Mutex<f32>>) {
    if samples.is_empty() {
        return;
    }

    let sum = samples
        .iter()
        .map(|sample| sample * sample)
        .sum::<f32>();

    let rms = (sum / samples.len() as f32).sqrt();

    if let Ok(mut current_level) = level.lock() {
        *current_level = rms;
    }
}
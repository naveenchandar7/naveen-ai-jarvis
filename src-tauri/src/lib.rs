#![cfg_attr(mobile, tauri::mobile_entry_point)]

use std::time::Duration;

use tauri::{Emitter, Manager, State};

mod auth;
mod audio;
mod capabilities;
mod core_supervisor;
mod device_gateway;
mod ipc;
mod security;

const SYSTEM_TELEMETRY_EVENT: &str = "host://telemetry/system";
const VOICE_TELEMETRY_EVENT: &str = "host://voice/telemetry";

#[tauri::command]
fn get_system_info() -> device_gateway::SystemInfo {
    device_gateway::snapshot_system_info()
}

#[tauri::command]
fn submit_text(
    text: String,
    supervisor: State<'_, core_supervisor::CoreSupervisor>,
) -> Result<String, String> {
    supervisor.submit_text(text)
}

#[tauri::command]
fn start_voice(audio_state: State<'_, audio::AudioState>) -> Result<(), String> {
    let mut stream = audio_state
        .stream
        .lock()
        .map_err(|_| "Microphone state lock failed".to_string())?;

    if stream.is_some() {
        return Ok(());
    }

    *stream = Some(audio::start_microphone()?);
    Ok(())
}

#[tauri::command]
fn stop_voice(audio_state: State<'_, audio::AudioState>) -> Result<(), String> {
    let mut stream = audio_state
        .stream
        .lock()
        .map_err(|_| "Microphone state lock failed".to_string())?;
    *stream = None;
    Ok(())
}

#[tauri::command]
fn get_voice_level(audio_state: State<'_, audio::AudioState>) -> Result<f32, String> {
    let stream = audio_state
        .stream
        .lock()
        .map_err(|_| "Microphone state lock failed".to_string())?;
    match stream.as_ref() {
        Some(input) => audio::get_level(input),
        None => Ok(0.0),
    }
}

pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            app.manage(audio::AudioState::default());

            #[cfg(windows)]
            {
                match core_supervisor::CoreLaunchConfig::from_app(app.handle()) {
                    Ok(config) => {
                        app.manage(core_supervisor::CoreSupervisor::start(
                            config,
                            app.handle().clone(),
                        ));
                    }
                    Err(_) => {
                        log::warn!(
                            "NAVEEN Core unavailable: local Core resource not configured"
                        );
                        app.manage(core_supervisor::CoreSupervisor::disabled());
                    }
                }
            }

            #[cfg(not(windows))]
            {
                app.manage(core_supervisor::CoreSupervisor::disabled());
            }

            let telemetry_handle = app.handle().clone();
            let audio_handle = app.handle().clone();

            std::thread::spawn(move || loop {
                let system_info = device_gateway::snapshot_system_info();

                if telemetry_handle
                    .emit(SYSTEM_TELEMETRY_EVENT, system_info)
                    .is_err()
                {
                    break;
                }

                let level = audio_handle
                    .try_state::<audio::AudioState>()
                    .and_then(|state| state.stream.lock().ok())
                    .and_then(|stream| stream.as_ref().map(audio::get_level));
                let mic_level = match level {
                    Some(Ok(value)) => value,
                    _ => 0.0,
                };

                if audio_handle
                    .emit(VOICE_TELEMETRY_EVENT, mic_level)
                    .is_err()
                {
                    break;
                }

                std::thread::sleep(Duration::from_millis(100));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            submit_text,
            start_voice,
            stop_voice,
            get_voice_level,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            let _ = app_handle.state::<audio::AudioState>().stream.lock().map(|mut s| {
                *s = None;
            });
            app_handle
                .state::<core_supervisor::CoreSupervisor>()
                .shutdown_and_join();
        }
    });
}

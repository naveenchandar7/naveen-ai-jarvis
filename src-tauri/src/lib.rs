#![cfg_attr(mobile, tauri::mobile_entry_point)]

use std::time::Duration;

use tauri::{Emitter, Manager, State};

#[expect(
    dead_code,
    clippy::too_many_arguments,
    clippy::wrong_self_convention,
    reason = "Authentication exposes stable foundational APIs that are intentionally consumed incrementally as the host/runtime layers expand."
)]
mod auth;
#[expect(
    dead_code,
    reason = "Audio owns the native stream lifetime; some provider-facing pieces are intentionally dormant until voice providers are integrated."
)]
mod audio;
mod capabilities;
#[expect(
    dead_code,
    clippy::enum_variant_names,
    reason = "Core supervision keeps explicit authentication control-message names to preserve the existing wire contract."
)]
mod core_supervisor;
mod device_gateway;
#[expect(
    dead_code,
    reason = "IPC retains replaceable transport/audit abstractions whose public surface is exercised incrementally by the live runtime."
)]
mod ipc;
#[expect(
    clippy::manual_contains,
    reason = "Network allowlist compatibility code is intentionally kept stable while the gateway is integrated through the host capability boundary."
)]
mod network_gateway;
#[expect(
    dead_code,
    reason = "Security policy types expose the stable enforcement contract used as capabilities are integrated incrementally."
)]
mod security;

const SYSTEM_TELEMETRY_EVENT: &str = "host://telemetry/system";
const VOICE_TELEMETRY_EVENT: &str = "host://voice/telemetry";

#[tauri::command]
fn get_system_info() -> device_gateway::SystemInfo {
    device_gateway::snapshot_system_info()
}

#[tauri::command]
fn get_core_status(
    supervisor: State<'_, core_supervisor::CoreSupervisor>,
) -> &'static str {
    if supervisor.is_connected() {
        "connected"
    } else {
        "connecting"
    }
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

                let mic_level = audio_handle
                    .try_state::<audio::AudioState>()
                    .and_then(|state| {
                        let stream = state.stream.lock().ok()?;
                        stream.as_ref().map(audio::get_level)
                    })
                    .and_then(Result::ok)
                    .unwrap_or(0.0);

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
            get_core_status,
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

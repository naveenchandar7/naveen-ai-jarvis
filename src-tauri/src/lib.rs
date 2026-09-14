#![cfg_attr(mobile, tauri::mobile_entry_point)]

use std::time::Duration;

use tauri::{Emitter, Manager, State};

#[expect(
    dead_code,
    reason = "Audio owns the native stream lifetime; some provider-facing pieces are intentionally dormant until voice providers are integrated."
)]
mod audio;
#[expect(
    dead_code,
    clippy::too_many_arguments,
    clippy::wrong_self_convention,
    reason = "Authentication exposes stable foundational APIs that are intentionally consumed incrementally as the host/runtime layers expand."
)]
mod auth;
mod capabilities;
#[expect(
    dead_code,
    clippy::enum_variant_names,
    clippy::too_many_arguments,
    reason = "Core supervision keeps explicit authentication control-message names to preserve the existing wire contract."
)]
mod core_supervisor;
mod device_gateway;
mod device_security;
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
fn get_core_status(supervisor: State<'_, core_supervisor::CoreSupervisor>) -> &'static str {
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

pub fn run() {
    let builder = tauri::Builder::default()
        .manage(audio::AudioState::default())
        .manage(core_supervisor::CoreSupervisor::new())
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_core_status,
            submit_text,
            start_voice,
            stop_voice,
        ]);

    let app = builder
        .setup(|app| {
            let handle = app.handle().clone();
            let supervisor = app.state::<core_supervisor::CoreSupervisor>().inner().clone();
            supervisor.start(handle.clone());

            let telemetry_handle = handle.clone();
            std::thread::spawn(move || loop {
                let info = device_gateway::snapshot_system_info();
                if telemetry_handle
                    .emit(SYSTEM_TELEMETRY_EVENT, &info)
                    .is_err()
                {
                    break;
                }
                std::thread::sleep(Duration::from_secs(2));
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running NAVEEN AI");
}

#![cfg_attr(mobile, tauri::mobile_entry_point)]

mod audio;

use serde::Serialize;
use sysinfo::System;
use tauri::State;

use audio::{get_level, AudioState};

#[derive(Serialize)]
struct SystemInfo {
    cpu_usage: f32,
    memory_used: u64,
    memory_total: u64,
    os_name: String,
    os_version: String,
    uptime: u64,
}

#[tauri::command]
fn jarvis_ping() -> String {
    "CORE_ONLINE".to_string()
}

#[tauri::command]
fn get_system_info() -> SystemInfo {
    let mut system = System::new_all();

    std::thread::sleep(
        sysinfo::MINIMUM_CPU_UPDATE_INTERVAL
    );

    system.refresh_cpu_usage();
    system.refresh_memory();

    SystemInfo {
        cpu_usage: system.global_cpu_usage(),
        memory_used: system.used_memory(),
        memory_total: system.total_memory(),
        os_name: System::name()
            .unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::os_version()
            .unwrap_or_else(|| "Unknown".to_string()),
        uptime: System::uptime(),
    }
}

#[tauri::command]
fn start_microphone(
    state: State<'_, AudioState>,
) -> Result<String, String> {
    let mut stream_guard = state
        .stream
        .lock()
        .map_err(|_| {
            "Microphone state lock failed".to_string()
        })?;

    if stream_guard.is_some() {
        return Ok(
            "MICROPHONE_ALREADY_RUNNING".to_string()
        );
    }

    let input = audio::start_microphone()?;

    *stream_guard = Some(input);

    Ok("MICROPHONE_STARTED".to_string())
}

#[tauri::command]
fn stop_microphone(
    state: State<'_, AudioState>,
) -> Result<String, String> {
    let mut stream_guard = state
        .stream
        .lock()
        .map_err(|_| {
            "Microphone state lock failed".to_string()
        })?;

    if stream_guard.is_none() {
        return Ok(
            "MICROPHONE_ALREADY_STOPPED".to_string()
        );
    }

    *stream_guard = None;

    Ok("MICROPHONE_STOPPED".to_string())
}

#[tauri::command]
fn get_microphone_level(
    state: State<'_, AudioState>,
) -> Result<f32, String> {
    let stream_guard = state
        .stream
        .lock()
        .map_err(|_| {
            "Microphone state lock failed".to_string()
        })?;

    match stream_guard.as_ref() {
        Some(input) => get_level(input),
        None => Ok(0.0),
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(AudioState::default())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            jarvis_ping,
            get_system_info,
            start_microphone,
            stop_microphone,
            get_microphone_level
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
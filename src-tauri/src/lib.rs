#![cfg_attr(mobile, tauri::mobile_entry_point)]

use serde::Serialize;
use std::time::Duration;
use sysinfo::System;
use tauri::Emitter;
use tauri::Manager;

mod auth;
mod core_supervisor;
mod ipc;
mod security;

const SYSTEM_TELEMETRY_EVENT: &str = "host://telemetry/system";

#[derive(Clone, Serialize)]
struct SystemInfo {
    cpu_usage: f32,
    memory_used: u64,
    memory_total: u64,
    os_name: String,
    os_version: String,
    uptime: u64,
}

#[tauri::command]
fn get_system_info() -> SystemInfo {
    let mut system = System::new_all();

    system.refresh_cpu_usage();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_cpu_usage();
    system.refresh_memory();

    system_info_from(&system)
}

fn system_info_from(system: &System) -> SystemInfo {
    SystemInfo {
        cpu_usage: system.global_cpu_usage(),
        memory_used: system.used_memory(),
        memory_total: system.total_memory(),
        os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        uptime: System::uptime(),
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

            #[cfg(windows)]
            {
                match core_supervisor::CoreLaunchConfig::from_app(app.handle()) {
                    Ok(config) => {
                        app.manage(core_supervisor::CoreSupervisor::start(config));
                    }
                    Err(_) => {
                        log::warn!("NAVEEN Core unavailable: local Core resource not configured");
                        app.manage(core_supervisor::CoreSupervisor::disabled());
                    }
                }
            }

            #[cfg(not(windows))]
            {
                app.manage(core_supervisor::CoreSupervisor::disabled());
            }

            let app_handle = app.handle().clone();

            std::thread::spawn(move || {
                let mut system = System::new_all();

                loop {
                    system.refresh_cpu_usage();
                    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
                    system.refresh_cpu_usage();
                    system.refresh_memory();

                    if app_handle
                        .emit(SYSTEM_TELEMETRY_EVENT, system_info_from(&system))
                        .is_err()
                    {
                        break;
                    }

                    std::thread::sleep(Duration::from_secs(2));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_system_info])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            app_handle
                .state::<core_supervisor::CoreSupervisor>()
                .shutdown_and_join();
        }
    });
}

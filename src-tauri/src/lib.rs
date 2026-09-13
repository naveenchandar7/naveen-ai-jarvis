#![cfg_attr(mobile, tauri::mobile_entry_point)]

use std::time::Duration;

use tauri::{Emitter, Manager, State};

mod auth;
mod capabilities;
mod core_supervisor;
mod device_gateway;
mod ipc;
mod security;

const SYSTEM_TELEMETRY_EVENT: &str = "host://telemetry/system";

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

            let app_handle = app.handle().clone();

            std::thread::spawn(move || loop {
                let system_info = device_gateway::snapshot_system_info();

                if app_handle
                    .emit(SYSTEM_TELEMETRY_EVENT, system_info)
                    .is_err()
                {
                    break;
                }

                std::thread::sleep(Duration::from_secs(2));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_system_info, submit_text])
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

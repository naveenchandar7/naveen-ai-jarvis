use serde::Serialize;
use sysinfo::System;

#[derive(Clone, Serialize)]
pub struct SystemInfo {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub os_name: String,
    pub os_version: String,
    pub uptime: u64,
}

pub fn snapshot_system_info() -> SystemInfo {
    let mut system = System::new_all();

    system.refresh_cpu_usage();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_cpu_usage();
    system.refresh_memory();

    SystemInfo {
        cpu_usage: system.global_cpu_usage(),
        memory_used: system.used_memory(),
        memory_total: system.total_memory(),
        os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        uptime: System::uptime(),
    }
}

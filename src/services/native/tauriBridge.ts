import { invoke } from "@tauri-apps/api/core";

export async function pingJarvisCore(): Promise<string> {
  return await invoke<string>("jarvis_ping");
}

export interface SystemInfo {
  cpu_usage: number;
  memory_used: number;
  memory_total: number;
  os_name: string;
  os_version: string;
  uptime: number;
}

export async function getSystemInfo(): Promise<SystemInfo> {
  return await invoke<SystemInfo>(
    "get_system_info",
  );
}

export async function startMicrophone(): Promise<string> {
  return await invoke<string>(
    "start_microphone",
  );
}

export async function stopMicrophone(): Promise<string> {
  return await invoke<string>(
    "stop_microphone",
  );
}

export async function getMicrophoneLevel(): Promise<number> {
  return await invoke<number>(
    "get_microphone_level",
  );
}
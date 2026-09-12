import { invoke } from "@tauri-apps/api/core";
import {
  listen,
  type UnlistenFn,
} from "@tauri-apps/api/event";

const SYSTEM_TELEMETRY_EVENT =
  "host://telemetry/system";


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

export async function subscribeToSystemTelemetry(
  onSystemInfo: (
    systemInfo: SystemInfo,
  ) => void,
): Promise<UnlistenFn> {
  return await listen<SystemInfo>(
    SYSTEM_TELEMETRY_EVENT,
    (event) => {
      onSystemInfo(event.payload);
    },
  );
}

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const SYSTEM_TELEMETRY_EVENT = "host://telemetry/system";
const CORE_EVENT = "host://core/event";

export interface SystemInfo {
  cpu_usage: number;
  memory_used: number;
  memory_total: number;
  os_name: string;
  os_version: string;
  uptime: number;
}

export interface CoreEvent {
  event_type: string;
  event_id: string;
  payload: Record<string, unknown>;
}

export async function getSystemInfo(): Promise<SystemInfo> {
  return await invoke<SystemInfo>("get_system_info");
}

export async function getCoreStatus(): Promise<"connected" | "connecting"> {
  return await invoke<"connected" | "connecting">("get_core_status");
}

export async function submitText(text: string): Promise<string> {
  return await invoke<string>("submit_text", { text });
}

export async function subscribeToSystemTelemetry(
  onSystemInfo: (systemInfo: SystemInfo) => void,
): Promise<UnlistenFn> {
  return await listen<SystemInfo>(SYSTEM_TELEMETRY_EVENT, (event) => {
    onSystemInfo(event.payload);
  });
}

export async function subscribeToCoreEvents(
  onCoreEvent: (event: CoreEvent) => void,
): Promise<UnlistenFn> {
  return await listen<CoreEvent>(CORE_EVENT, (event) => {
    onCoreEvent(event.payload);
  });
}

use crate::device_gateway::snapshot_system_info;
use serde_json::Value;

pub const SYSTEM_TELEMETRY_READ: &str = "system.telemetry.read";

pub struct HostCapabilityRegistry;

impl HostCapabilityRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, capability_id: &str, input: Value) -> Result<Value, String> {
        match capability_id {
            SYSTEM_TELEMETRY_READ => {
                if !input.is_object() {
                    return Err("invalid capability input".to_string());
                }
                serde_json::to_value(snapshot_system_info())
                    .map_err(|_| "capability output serialization failed".to_string())
            }
            _ => Err("capability is not registered".to_string()),
        }
    }
}

impl Default for HostCapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

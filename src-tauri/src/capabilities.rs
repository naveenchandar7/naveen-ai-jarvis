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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_capability_is_denied() {
        let registry = HostCapabilityRegistry::new();
        assert!(registry.execute("unknown.capability", json!({})).is_err());
    }

    #[test]
    fn telemetry_requires_object_input() {
        let registry = HostCapabilityRegistry::new();
        assert!(registry
            .execute(SYSTEM_TELEMETRY_READ, Value::Null)
            .is_err());
    }
}

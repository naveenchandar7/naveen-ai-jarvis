use crate::device_gateway::snapshot_system_info;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

pub const SYSTEM_TELEMETRY_READ: &str = "system.telemetry.read";
pub const FILESYSTEM_READ_TEXT: &str = "filesystem.read_text";
const DEFAULT_MAX_READ_BYTES: u64 = 512 * 1024;

pub struct HostCapabilityRegistry {
    workspace_root: Option<PathBuf>,
}

impl HostCapabilityRegistry {
    pub fn new(workspace_root: Option<PathBuf>) -> Self {
        Self { workspace_root }
    }

    pub fn execute(&self, capability_id: &str, input: Value) -> Result<Value, String> {
        match capability_id {
            SYSTEM_TELEMETRY_READ => self.read_system_telemetry(input),
            FILESYSTEM_READ_TEXT => self.read_workspace_text(input),
            _ => Err("capability is not registered".to_string()),
        }
    }

    fn read_system_telemetry(&self, input: Value) -> Result<Value, String> {
        if !input.is_object() {
            return Err("invalid capability input".to_string());
        }
        serde_json::to_value(snapshot_system_info())
            .map_err(|_| "capability output serialization failed".to_string())
    }

    fn read_workspace_text(&self, input: Value) -> Result<Value, String> {
        let Some(root) = &self.workspace_root else {
            return Err("workspace is not configured".to_string());
        };

        let object = input
            .as_object()
            .ok_or_else(|| "invalid capability input".to_string())?;
        let requested_path = object
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing file path".to_string())?;
        let max_bytes = object
            .get("max_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_MAX_READ_BYTES)
            .min(DEFAULT_MAX_READ_BYTES);

        let canonical_root = root
            .canonicalize()
            .map_err(|_| "workspace is unavailable".to_string())?;
        let candidate = canonical_root.join(requested_path);
        let canonical_candidate = candidate
            .canonicalize()
            .map_err(|_| "file is unavailable".to_string())?;

        if !canonical_candidate.starts_with(&canonical_root) {
            return Err("path is outside the configured workspace".to_string());
        }

        let metadata = fs::metadata(&canonical_candidate)
            .map_err(|_| "file metadata is unavailable".to_string())?;
        if !metadata.is_file() {
            return Err("path is not a file".to_string());
        }
        if metadata.len() > max_bytes {
            return Err("file exceeds the configured read limit".to_string());
        }

        let bytes = fs::read(&canonical_candidate)
            .map_err(|_| "file could not be read".to_string())?;
        let content = String::from_utf8(bytes)
            .map_err(|_| "file is not valid UTF-8 text".to_string())?;
        let relative = canonical_candidate
            .strip_prefix(&canonical_root)
            .map_err(|_| "file path normalization failed".to_string())?
            .to_string_lossy()
            .replace('\\', "/");

        Ok(json!({
            "path": relative,
            "size_bytes": metadata.len(),
            "content": content,
        }))
    }
}

impl Default for HostCapabilityRegistry {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn unknown_capability_is_denied() {
        let registry = HostCapabilityRegistry::new(None);
        assert!(registry.execute("unknown.capability", json!({})).is_err());
    }

    #[test]
    fn telemetry_requires_object_input() {
        let registry = HostCapabilityRegistry::new(None);
        assert!(registry
            .execute(SYSTEM_TELEMETRY_READ, Value::Null)
            .is_err());
    }

    #[test]
    fn file_reads_require_workspace_configuration() {
        let registry = HostCapabilityRegistry::new(None);
        assert!(registry
            .execute(FILESYSTEM_READ_TEXT, json!({"path": "note.txt"}))
            .is_err());
    }

    #[test]
    fn file_reads_are_confined_to_workspace() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("naveen-ai-{unique}"));
        let workspace = root.join("workspace");
        let outside = root.join("outside.txt");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(workspace.join("note.txt"), "hello from NAVEEN").expect("note");
        fs::write(&outside, "secret").expect("outside");

        let registry = HostCapabilityRegistry::new(Some(workspace));
        let value = registry
            .execute(FILESYSTEM_READ_TEXT, json!({"path": "note.txt"}))
            .expect("read");
        assert_eq!(value["content"], "hello from NAVEEN");

        assert!(registry
            .execute(FILESYSTEM_READ_TEXT, json!({"path": "../outside.txt"}))
            .is_err());
        let _ = fs::remove_dir_all(root);
    }
}

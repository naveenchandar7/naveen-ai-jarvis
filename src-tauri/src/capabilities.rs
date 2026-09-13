use crate::device_gateway::snapshot_system_info;
use crate::network_gateway::{NetworkGateway, MODEL_COMPLETE, NETWORK_FETCH_TEXT};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub const SYSTEM_TELEMETRY_READ: &str = "system.telemetry.read";
pub const FILESYSTEM_READ_TEXT: &str = "filesystem.read_text";
pub const FILESYSTEM_WRITE_TEXT: &str = "filesystem.write_text";
pub const OPEN_URL: &str = "os.url.open";
const DEFAULT_MAX_READ_BYTES: u64 = 512 * 1024;
const MAX_WRITE_BYTES: usize = 256 * 1024;

pub struct HostCapabilityRegistry {
    workspace_root: Option<PathBuf>,
    network: NetworkGateway,
}

impl HostCapabilityRegistry {
    pub fn new(workspace_root: Option<PathBuf>, network: NetworkGateway) -> Self {
        Self {
            workspace_root,
            network,
        }
    }

    pub fn execute(&self, capability_id: &str, input: Value) -> Result<Value, String> {
        match capability_id {
            SYSTEM_TELEMETRY_READ => self.read_system_telemetry(input),
            FILESYSTEM_READ_TEXT => self.read_workspace_text(input),
            FILESYSTEM_WRITE_TEXT => self.write_workspace_text(input),
            OPEN_URL => self.open_url(input),
            MODEL_COMPLETE => self.complete_model(input),
            NETWORK_FETCH_TEXT => self.fetch_network_text(input),
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
        let canonical_candidate = self.resolve_existing_workspace_path(input)?;
        let metadata = fs::metadata(&canonical_candidate)
            .map_err(|_| "file metadata is unavailable".to_string())?;
        if !metadata.is_file() {
            return Err("path is not a file".to_string());
        }
        if metadata.len() > DEFAULT_MAX_READ_BYTES {
            return Err("file exceeds the configured read limit".to_string());
        }

        let bytes = fs::read(&canonical_candidate)
            .map_err(|_| "file could not be read".to_string())?;
        let content = String::from_utf8(bytes)
            .map_err(|_| "file is not valid UTF-8 text".to_string())?;
        let relative = self.relative_workspace_path(&canonical_candidate)?;

        Ok(json!({
            "path": relative,
            "size_bytes": metadata.len(),
            "content": content,
        }))
    }

    fn write_workspace_text(&self, input: Value) -> Result<Value, String> {
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
        let content = object
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing file content".to_string())?;

        if content.len() > MAX_WRITE_BYTES {
            return Err("file content exceeds the write limit".to_string());
        }

        let canonical_root = root
            .canonicalize()
            .map_err(|_| "workspace is unavailable".to_string())?;
        let candidate = canonical_root.join(requested_path);
        let parent = candidate
            .parent()
            .ok_or_else(|| "invalid file path".to_string())?
            .canonicalize()
            .map_err(|_| "file parent is unavailable".to_string())?;

        if !parent.starts_with(&canonical_root) {
            return Err("path is outside the configured workspace".to_string());
        }

        if candidate.exists() {
            let symlink = fs::symlink_metadata(&candidate)
                .map_err(|_| "file metadata is unavailable".to_string())?
                .file_type()
                .is_symlink();
            if symlink {
                return Err("symlink targets are not writable".to_string());
            }

            let canonical_existing = candidate
                .canonicalize()
                .map_err(|_| "existing file is unavailable".to_string())?;
            if !canonical_existing.starts_with(&canonical_root) {
                return Err("path is outside the configured workspace".to_string());
            }
        }

        fs::write(&candidate, content).map_err(|_| "file could not be written".to_string())?;
        let canonical_written = candidate
            .canonicalize()
            .map_err(|_| "written file path normalization failed".to_string())?;
        Ok(json!({
            "path": self.relative_workspace_path(&canonical_written)?,
            "size_bytes": content.len(),
        }))
    }

    fn open_url(&self, input: Value) -> Result<Value, String> {
        let url = input
            .as_object()
            .and_then(|object| object.get("url"))
            .and_then(Value::as_str)
            .ok_or_else(|| "missing URL".to_string())?
            .trim();
        let lower = url.to_ascii_lowercase();
        if !(lower.starts_with("https://") || lower.starts_with("http://"))
            || url.bytes().any(|byte| byte.is_ascii_whitespace() || byte == b'\\')
        {
            return Err("only http(s) URLs are allowed".to_string());
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let system_root = std::env::var_os("SystemRoot")
                .ok_or_else(|| "Windows system root is unavailable".to_string())?;
            let explorer = PathBuf::from(system_root).join("explorer.exe");
            Command::new(explorer)
                .arg(url)
                .spawn()
                .map_err(|_| "URL could not be opened".to_string())?;
            Ok(json!({"url": url, "opened": true}))
        }

        #[cfg(not(windows))]
        {
            let _ = url;
            Err("URL opening is only enabled on Windows in this build".to_string())
        }
    }

    fn resolve_existing_workspace_path(&self, input: Value) -> Result<PathBuf, String> {
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
        let canonical_root = root
            .canonicalize()
            .map_err(|_| "workspace is unavailable".to_string())?;
        let canonical_candidate = canonical_root
            .join(requested_path)
            .canonicalize()
            .map_err(|_| "file is unavailable".to_string())?;
        if !canonical_candidate.starts_with(&canonical_root) {
            return Err("path is outside the configured workspace".to_string());
        }
        Ok(canonical_candidate)
    }

    fn relative_workspace_path(&self, path: &Path) -> Result<String, String> {
        let Some(root) = &self.workspace_root else {
            return Err("workspace is not configured".to_string());
        };
        let canonical_root = root
            .canonicalize()
            .map_err(|_| "workspace is unavailable".to_string())?;
        path.strip_prefix(canonical_root)
            .map_err(|_| "file path normalization failed".to_string())
            .map(|value| value.to_string_lossy().replace('\\', "/"))
    }

    fn complete_model(&self, input: Value) -> Result<Value, String> {
        let prompt = input
            .as_object()
            .and_then(|object| object.get("prompt"))
            .and_then(Value::as_str)
            .ok_or_else(|| "missing model prompt".to_string())?;
        let text = self.network.model_complete(prompt)?;
        Ok(json!({"content": text}))
    }

    fn fetch_network_text(&self, input: Value) -> Result<Value, String> {
        let url = input
            .as_object()
            .and_then(|object| object.get("url"))
            .and_then(Value::as_str)
            .ok_or_else(|| "missing network URL".to_string())?;
        let text = self.network.fetch_text(url)?;
        Ok(json!({"url": url, "content": text}))
    }
}

impl Default for HostCapabilityRegistry {
    fn default() -> Self {
        Self::new(None, NetworkGateway::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace(prefix: &str) -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{prefix}-{unique}"));
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        (root, workspace)
    }

    #[test]
    fn unknown_capability_is_denied() {
        let registry = HostCapabilityRegistry::default();
        assert!(registry.execute("unknown.capability", json!({})).is_err());
    }

    #[test]
    fn telemetry_requires_object_input() {
        let registry = HostCapabilityRegistry::default();
        assert!(registry
            .execute(SYSTEM_TELEMETRY_READ, Value::Null)
            .is_err());
    }

    #[test]
    fn file_reads_require_workspace_configuration() {
        let registry = HostCapabilityRegistry::default();
        assert!(registry
            .execute(FILESYSTEM_READ_TEXT, json!({"path": "note.txt"}))
            .is_err());
    }

    #[test]
    fn file_reads_are_confined_to_workspace() {
        let (root, workspace) = temp_workspace("naveen-read");
        let outside = root.join("outside.txt");
        fs::write(workspace.join("note.txt"), "hello from NAVEEN").expect("note");
        fs::write(&outside, "secret").expect("outside");
        let registry = HostCapabilityRegistry::new(Some(workspace), NetworkGateway::default());

        let value = registry
            .execute(FILESYSTEM_READ_TEXT, json!({"path": "note.txt"}))
            .expect("read");
        assert_eq!(value["content"], "hello from NAVEEN");
        assert!(registry
            .execute(FILESYSTEM_READ_TEXT, json!({"path": "../outside.txt"}))
            .is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn workspace_writes_are_bounded_and_confined() {
        let (root, workspace) = temp_workspace("naveen-write");
        let registry = HostCapabilityRegistry::new(Some(workspace.clone()), NetworkGateway::default());

        let value = registry
            .execute(
                FILESYSTEM_WRITE_TEXT,
                json!({"path": "note.txt", "content": "hello"}),
            )
            .expect("write");
        assert_eq!(value["size_bytes"], 5);
        assert_eq!(
            fs::read_to_string(workspace.join("note.txt")).expect("read"),
            "hello"
        );
        assert!(registry
            .execute(
                FILESYSTEM_WRITE_TEXT,
                json!({"path": "../outside.txt", "content": "no"}),
            )
            .is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn oversized_write_is_rejected() {
        let (root, workspace) = temp_workspace("naveen-write-size");
        let registry = HostCapabilityRegistry::new(Some(workspace), NetworkGateway::default());
        let oversized = "x".repeat(MAX_WRITE_BYTES + 1);
        assert!(registry
            .execute(
                FILESYSTEM_WRITE_TEXT,
                json!({"path": "large.txt", "content": oversized}),
            )
            .is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn url_capability_rejects_unsafe_schemes() {
        let registry = HostCapabilityRegistry::default();
        assert!(registry
            .execute(OPEN_URL, json!({"url": "file:///C:/secret.txt"}))
            .is_err());
        assert!(registry
            .execute(OPEN_URL, json!({"url": "javascript:alert(1)"}))
            .is_err());
    }
}
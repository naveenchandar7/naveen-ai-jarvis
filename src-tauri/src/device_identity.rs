use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

const NODE_ID_FILE: &str = "node_id.txt";
const NODE_ID_PREFIX: &str = "node-";

#[derive(Debug, PartialEq, Eq)]
pub enum DeviceIdentityError {
    AppDataDirectory,
    DirectoryCreate,
    Read,
    Write,
    InvalidStoredIdentity,
}

/// Load the host node identity from Tauri's app-local data directory, creating
/// it exactly once when this installation has no identity yet.
///
/// The node identifier is an opaque persistent identifier, not a credential.
/// Cryptographic device credentials belong to the later enrollment/auth layer.
pub fn load_or_create_node_id(app: &AppHandle) -> Result<String, DeviceIdentityError> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|_| DeviceIdentityError::AppDataDirectory)?;
    fs::create_dir_all(&root).map_err(|_| DeviceIdentityError::DirectoryCreate)?;

    let path = root.join(NODE_ID_FILE);
    match fs::read_to_string(&path) {
        Ok(value) => return validate_node_id(value.trim().to_string()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return Err(DeviceIdentityError::Read),
    }

    let candidate = generate_node_id();
    match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(candidate.as_bytes())
                .map_err(|_| DeviceIdentityError::Write)?;
            file.sync_all().map_err(|_| DeviceIdentityError::Write)?;
            Ok(candidate)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let value = fs::read_to_string(&path).map_err(|_| DeviceIdentityError::Read)?;
            validate_node_id(value.trim().to_string())
        }
        Err(_) => Err(DeviceIdentityError::Write),
    }
}

fn generate_node_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let process = std::process::id() as u128;
    format!("{NODE_ID_PREFIX}{timestamp:032x}-{process:08x}")
}

fn validate_node_id(node_id: String) -> Result<String, DeviceIdentityError> {
    if node_id.starts_with(NODE_ID_PREFIX)
        && node_id.len() <= 128
        && node_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        Ok(node_id)
    } else {
        Err(DeviceIdentityError::InvalidStoredIdentity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_identity_has_expected_shape() {
        let node_id = generate_node_id();
        assert!(node_id.starts_with(NODE_ID_PREFIX));
        assert!(validate_node_id(node_id).is_ok());
    }

    #[test]
    fn invalid_identity_is_rejected() {
        assert_eq!(
            validate_node_id("not-a-node-id".to_string()),
            Err(DeviceIdentityError::InvalidStoredIdentity)
        );
        assert_eq!(
            validate_node_id("node-unsafe id".to_string()),
            Err(DeviceIdentityError::InvalidStoredIdentity)
        );
    }
}
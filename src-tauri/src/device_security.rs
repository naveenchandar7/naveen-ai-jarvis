use std::collections::{HashMap, HashSet};

use crate::security::Permission;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTrust {
    Pending,
    Trusted,
    Quarantined,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRecord {
    pub device_id: String,
    pub trust: DeviceTrust,
    allowed_permissions: HashSet<Permission>,
}

impl DeviceRecord {
    pub fn new(
        device_id: impl Into<String>,
        trust: DeviceTrust,
        allowed_permissions: impl IntoIterator<Item = Permission>,
    ) -> Result<Self, DeviceSecurityError> {
        let device_id = device_id.into();
        if device_id.trim().is_empty() {
            return Err(DeviceSecurityError::InvalidDeviceId);
        }

        Ok(Self {
            device_id,
            trust,
            allowed_permissions: allowed_permissions.into_iter().collect(),
        })
    }

    fn allows_all(&self, permissions: &[Permission]) -> bool {
        permissions
            .iter()
            .all(|permission| self.allowed_permissions.contains(permission))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSecurityError {
    InvalidDeviceId,
    DeviceAlreadyRegistered,
    DeviceNotRegistered,
    DeviceNotTrusted,
    PermissionNotGranted,
}

/// Host-owned registry for per-device trust and least-privilege permission grants.
///
/// Registration does not imply trust. A device must be explicitly promoted to
/// `Trusted` and only receives the permissions listed in its record.
pub struct DeviceRegistry {
    devices: HashMap<String, DeviceRecord>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub fn register(&mut self, device: DeviceRecord) -> Result<(), DeviceSecurityError> {
        if self.devices.contains_key(&device.device_id) {
            return Err(DeviceSecurityError::DeviceAlreadyRegistered);
        }
        self.devices.insert(device.device_id.clone(), device);
        Ok(())
    }

    pub fn set_trust(
        &mut self,
        device_id: &str,
        trust: DeviceTrust,
    ) -> Result<(), DeviceSecurityError> {
        let device = self
            .devices
            .get_mut(device_id)
            .ok_or(DeviceSecurityError::DeviceNotRegistered)?;
        device.trust = trust;
        Ok(())
    }

    pub fn authorize(
        &self,
        device_id: &str,
        required_permissions: &[Permission],
    ) -> Result<(), DeviceSecurityError> {
        let device = self
            .devices
            .get(device_id)
            .ok_or(DeviceSecurityError::DeviceNotRegistered)?;

        if device.trust != DeviceTrust::Trusted {
            return Err(DeviceSecurityError::DeviceNotTrusted);
        }

        if !device.allows_all(required_permissions) {
            return Err(DeviceSecurityError::PermissionNotGranted);
        }

        Ok(())
    }

    pub fn get(&self, device_id: &str) -> Option<&DeviceRecord> {
        self.devices.get(device_id)
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_does_not_implicitly_trust_a_device() {
        let mut registry = DeviceRegistry::new();
        registry
            .register(
                DeviceRecord::new(
                    "phone-01",
                    DeviceTrust::Pending,
                    [Permission::NetworkAccess],
                )
                .expect("valid device"),
            )
            .expect("registration succeeds");

        assert_eq!(
            registry.authorize("phone-01", &[Permission::NetworkAccess]),
            Err(DeviceSecurityError::DeviceNotTrusted)
        );
    }

    #[test]
    fn trusted_device_is_limited_to_granted_permissions() {
        let mut registry = DeviceRegistry::new();
        registry
            .register(
                DeviceRecord::new(
                    "pi-01",
                    DeviceTrust::Trusted,
                    [Permission::SystemTelemetryRead],
                )
                .expect("valid device"),
            )
            .expect("registration succeeds");

        assert_eq!(
            registry.authorize("pi-01", &[Permission::SystemTelemetryRead]),
            Ok(())
        );
        assert_eq!(
            registry.authorize("pi-01", &[Permission::FilesystemWrite]),
            Err(DeviceSecurityError::PermissionNotGranted)
        );
    }

    #[test]
    fn revoked_or_quarantined_devices_are_blocked() {
        let mut registry = DeviceRegistry::new();
        registry
            .register(
                DeviceRecord::new(
                    "robot-01",
                    DeviceTrust::Trusted,
                    [Permission::AudioCapture],
                )
                .expect("valid device"),
            )
            .expect("registration succeeds");

        registry
            .set_trust("robot-01", DeviceTrust::Quarantined)
            .expect("device exists");
        assert_eq!(
            registry.authorize("robot-01", &[Permission::AudioCapture]),
            Err(DeviceSecurityError::DeviceNotTrusted)
        );

        registry
            .set_trust("robot-01", DeviceTrust::Revoked)
            .expect("device exists");
        assert_eq!(
            registry.authorize("robot-01", &[Permission::AudioCapture]),
            Err(DeviceSecurityError::DeviceNotTrusted)
        );
    }

    #[test]
    fn unknown_devices_are_denied() {
        let registry = DeviceRegistry::new();

        assert_eq!(
            registry.authorize("unknown-device", &[Permission::NetworkAccess]),
            Err(DeviceSecurityError::DeviceNotRegistered)
        );
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut registry = DeviceRegistry::new();
        let device = DeviceRecord::new(
            "laptop-01",
            DeviceTrust::Trusted,
            [Permission::SystemTelemetryRead],
        )
        .expect("valid device");

        registry.register(device.clone()).expect("first registration");
        assert_eq!(
            registry.register(device),
            Err(DeviceSecurityError::DeviceAlreadyRegistered)
        );
    }

    #[test]
    fn empty_device_ids_are_rejected() {
        assert_eq!(
            DeviceRecord::new(
                "   ",
                DeviceTrust::Pending,
                std::iter::empty::<Permission>(),
            ),
            Err(DeviceSecurityError::InvalidDeviceId)
        );
    }
}

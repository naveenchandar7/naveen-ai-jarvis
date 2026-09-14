use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::device_security::{DeviceRecord, DeviceRegistry, DeviceSecurityError, DeviceTrust};
use crate::security::Permission;

/// Stable metadata for a NAVEEN node. Hardware type and transport are deliberately
/// represented as descriptive data instead of hardcoded Rust enums.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDescriptor {
    pub node_id: String,
    pub node_type: String,
    pub software_version: String,
}

impl NodeDescriptor {
    pub fn new(
        node_id: impl Into<String>,
        node_type: impl Into<String>,
        software_version: impl Into<String>,
    ) -> Result<Self, DeviceFabricError> {
        let node_id = node_id.into();
        let node_type = node_type.into();
        let software_version = software_version.into();

        if node_id.trim().is_empty() {
            return Err(DeviceFabricError::InvalidNodeId);
        }
        if node_type.trim().is_empty() {
            return Err(DeviceFabricError::InvalidNodeType);
        }
        if software_version.trim().is_empty() {
            return Err(DeviceFabricError::InvalidSoftwareVersion);
        }

        Ok(Self {
            node_id,
            node_type,
            software_version,
        })
    }
}

/// Runtime health state is intentionally transport/provider independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeHealth {
    Unknown,
    Healthy,
    Degraded,
    Offline,
}

/// Host-owned Device Fabric boundary.
///
/// The fabric owns node metadata and delegates trust/permission enforcement to
/// the existing DeviceRegistry. Concrete hardware adapters remain outside this
/// abstraction.
pub struct DeviceFabric {
    descriptor: NodeDescriptor,
    health: NodeHealth,
    capabilities: HashSet<String>,
    devices: DeviceRegistry,
}

impl DeviceFabric {
    pub fn new(descriptor: NodeDescriptor) -> Self {
        Self {
            descriptor,
            health: NodeHealth::Unknown,
            capabilities: HashSet::new(),
            devices: DeviceRegistry::new(),
        }
    }

    pub fn descriptor(&self) -> &NodeDescriptor {
        &self.descriptor
    }

    pub fn health(&self) -> NodeHealth {
        self.health
    }

    pub fn set_health(&mut self, health: NodeHealth) {
        self.health = health;
    }

    pub fn advertise_capability(
        &mut self,
        capability_id: impl Into<String>,
    ) -> Result<(), DeviceFabricError> {
        let capability_id = capability_id.into();
        if capability_id.trim().is_empty() {
            return Err(DeviceFabricError::InvalidCapabilityId);
        }
        self.capabilities.insert(capability_id);
        Ok(())
    }

    pub fn capabilities(&self) -> Vec<String> {
        let mut capabilities: Vec<_> = self.capabilities.iter().cloned().collect();
        capabilities.sort();
        capabilities
    }

    pub fn enroll_device(
        &mut self,
        device_id: impl Into<String>,
        trust: DeviceTrust,
        permissions: impl IntoIterator<Item = Permission>,
    ) -> Result<(), DeviceFabricError> {
        let device = DeviceRecord::new(device_id, trust, permissions)?;
        self.devices.register(device)?;
        Ok(())
    }

    pub fn set_device_trust(
        &mut self,
        device_id: &str,
        trust: DeviceTrust,
    ) -> Result<(), DeviceFabricError> {
        self.devices.set_trust(device_id, trust)?;
        Ok(())
    }

    pub fn authorize_device(
        &self,
        device_id: &str,
        required_permissions: &[Permission],
    ) -> Result<(), DeviceFabricError> {
        self.devices.authorize(device_id, required_permissions)?;
        Ok(())
    }

    pub(crate) fn device_registry(&self) -> &DeviceRegistry {
        &self.devices
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceFabricError {
    InvalidNodeId,
    InvalidNodeType,
    InvalidSoftwareVersion,
    InvalidCapabilityId,
    InvalidDevice(DeviceSecurityError),
}

impl From<DeviceSecurityError> for DeviceFabricError {
    fn from(error: DeviceSecurityError) -> Self {
        Self::InvalidDevice(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> NodeDescriptor {
        NodeDescriptor::new("host-local", "host", "0.1.0").expect("descriptor")
    }

    #[test]
    fn descriptor_rejects_empty_identity_fields() {
        assert_eq!(
            NodeDescriptor::new("", "host", "0.1.0"),
            Err(DeviceFabricError::InvalidNodeId)
        );
        assert_eq!(
            NodeDescriptor::new("host-local", "", "0.1.0"),
            Err(DeviceFabricError::InvalidNodeType)
        );
        assert_eq!(
            NodeDescriptor::new("host-local", "host", ""),
            Err(DeviceFabricError::InvalidSoftwareVersion)
        );
    }

    #[test]
    fn capabilities_are_advertised_without_hardware_specific_enums() {
        let mut fabric = DeviceFabric::new(descriptor());
        fabric
            .advertise_capability("system.telemetry.read")
            .expect("capability");
        fabric
            .advertise_capability("camera.capture")
            .expect("capability");

        assert_eq!(
            fabric.capabilities(),
            vec!["camera.capture".to_string(), "system.telemetry.read".to_string()]
        );
    }

    #[test]
    fn device_authorization_still_requires_trust_and_permission() {
        let mut fabric = DeviceFabric::new(descriptor());
        fabric
            .enroll_device(
                "pi-01",
                DeviceTrust::Pending,
                [Permission::SystemTelemetryRead],
            )
            .expect("enroll");

        assert_eq!(
            fabric.authorize_device("pi-01", &[Permission::SystemTelemetryRead]),
            Err(DeviceFabricError::InvalidDevice(
                DeviceSecurityError::DeviceNotTrusted
            ))
        );

        fabric
            .set_device_trust("pi-01", DeviceTrust::Trusted)
            .expect("trust");

        assert_eq!(
            fabric.authorize_device("pi-01", &[Permission::SystemTelemetryRead]),
            Ok(())
        );
    }

    #[test]
    fn unknown_device_is_denied_by_default() {
        let fabric = DeviceFabric::new(descriptor());
        assert_eq!(
            fabric.authorize_device("unknown", &[Permission::NetworkAccess]),
            Err(DeviceFabricError::InvalidDevice(
                DeviceSecurityError::DeviceNotRegistered
            ))
        );
    }
}

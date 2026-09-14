use crate::auth::AuthenticatedSession;
use crate::device_security::{DeviceRegistry, DeviceSecurityError};
use serde::{Deserialize, Serialize};

/// Version of the host-side authorization policy contract.
///
/// This is intentionally independent of the future Rust↔Python wire protocol.
pub const SECURITY_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    SystemTelemetryRead,
    AudioCapture,
    FilesystemRead,
    FilesystemWrite,
    ProcessExecute,
    NetworkAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Host-owned capability policy. The caller cannot choose its own permissions
/// or risk level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPolicy {
    capability_id: String,
    permissions: Vec<Permission>,
    risk: RiskLevel,
}

impl CapabilityPolicy {
    pub(crate) fn new(
        capability_id: impl Into<String>,
        permissions: Vec<Permission>,
        risk: RiskLevel,
    ) -> Self {
        Self {
            capability_id: capability_id.into(),
            permissions,
            risk,
        }
    }

    pub fn permissions(&self) -> &[Permission] {
        &self.permissions
    }

    pub fn risk(&self) -> RiskLevel {
        self.risk
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub capability_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityDecision {
    Allow,
    RequireConfirmation,
    Deny(SecurityDenial),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityDenial {
    Unauthenticated,
    SessionInactive,
    EmptyCapabilityId,
    CapabilityNotRegistered,
    HighRiskRequiresConfirmation,
    DeviceNotRegistered,
    DeviceNotTrusted,
    DevicePermissionNotGranted,
}

/// Deny-by-default authorization policy owned by the Rust host.
///
/// Authentication is deliberately separate: this gateway only accepts an
/// `AuthenticatedSession` issued by the authentication boundary.
pub struct SecurityGateway {
    policies: Vec<CapabilityPolicy>,
}

impl SecurityGateway {
    pub fn new(policies: Vec<CapabilityPolicy>) -> Self {
        Self { policies }
    }

    pub fn authorize_at(
        &self,
        now_ms: u64,
        session: Option<&AuthenticatedSession>,
        request: &CapabilityRequest,
        confirmed: bool,
    ) -> SecurityDecision {
        let Some(session) = session else {
            return SecurityDecision::Deny(SecurityDenial::Unauthenticated);
        };

        if !session.is_active_at(now_ms) {
            return SecurityDecision::Deny(SecurityDenial::SessionInactive);
        }

        if request.capability_id.trim().is_empty() {
            return SecurityDecision::Deny(SecurityDenial::EmptyCapabilityId);
        }

        let Some(policy) = self.find_policy(&request.capability_id) else {
            return SecurityDecision::Deny(SecurityDenial::CapabilityNotRegistered);
        };

        if policy.risk == RiskLevel::High && !confirmed {
            return SecurityDecision::RequireConfirmation;
        }

        SecurityDecision::Allow
    }

    /// Authorize a capability against both the authenticated session and the
    /// target device's host-owned trust/permission policy.
    pub fn authorize_for_device_at(
        &self,
        now_ms: u64,
        session: Option<&AuthenticatedSession>,
        device_id: &str,
        request: &CapabilityRequest,
        confirmed: bool,
        devices: &DeviceRegistry,
    ) -> SecurityDecision {
        match self.authorize_at(now_ms, session, request, confirmed) {
            SecurityDecision::Allow => {}
            decision => return decision,
        }

        let required_permissions = match self.required_permissions(&request.capability_id) {
            Ok(permissions) => permissions,
            Err(reason) => return SecurityDecision::Deny(reason),
        };

        match devices.authorize(device_id, required_permissions) {
            Ok(()) => SecurityDecision::Allow,
            Err(DeviceSecurityError::DeviceNotRegistered) => {
                SecurityDecision::Deny(SecurityDenial::DeviceNotRegistered)
            }
            Err(DeviceSecurityError::DeviceNotTrusted) => {
                SecurityDecision::Deny(SecurityDenial::DeviceNotTrusted)
            }
            Err(DeviceSecurityError::PermissionNotGranted) => {
                SecurityDecision::Deny(SecurityDenial::DevicePermissionNotGranted)
            }
            Err(
                DeviceSecurityError::InvalidDeviceId
                | DeviceSecurityError::DeviceAlreadyRegistered,
            ) => SecurityDecision::Deny(SecurityDenial::DeviceNotRegistered),
        }
    }

    /// Return the host-owned permission set for a registered capability.
    ///
    /// Callers must consume this metadata instead of supplying or overriding
    /// the permission requirements themselves.
    pub fn required_permissions(
        &self,
        capability_id: &str,
    ) -> Result<&[Permission], SecurityDenial> {
        if capability_id.trim().is_empty() {
            return Err(SecurityDenial::EmptyCapabilityId);
        }

        self.find_policy(capability_id)
            .map(|policy| policy.permissions())
            .ok_or(SecurityDenial::CapabilityNotRegistered)
    }

    fn find_policy(&self, capability_id: &str) -> Option<&CapabilityPolicy> {
        self.policies
            .iter()
            .find(|policy| policy.capability_id == capability_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::test_session;
    use crate::device_security::{DeviceRecord, DeviceTrust};

    fn session() -> AuthenticatedSession {
        test_session(10_000)
    }

    fn policy(risk: RiskLevel) -> CapabilityPolicy {
        CapabilityPolicy::new(
            "system.telemetry.read",
            vec![Permission::SystemTelemetryRead],
            risk,
        )
    }

    fn request() -> CapabilityRequest {
        CapabilityRequest {
            capability_id: "system.telemetry.read".to_string(),
        }
    }

    fn trusted_device() -> DeviceRegistry {
        let mut devices = DeviceRegistry::new();
        devices
            .register(
                DeviceRecord::new(
                    "pi-01",
                    DeviceTrust::Trusted,
                    [Permission::SystemTelemetryRead],
                )
                .expect("valid device"),
            )
            .expect("registration succeeds");
        devices
    }

    #[test]
    fn unauthenticated_requests_are_denied() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);

        assert_eq!(
            gateway.authorize_at(1_000, None, &request(), false),
            SecurityDecision::Deny(SecurityDenial::Unauthenticated)
        );
    }

    #[test]
    fn inactive_sessions_are_denied() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);
        let expired = test_session(1_000);

        assert_eq!(
            gateway.authorize_at(1_000, Some(&expired), &request(), false),
            SecurityDecision::Deny(SecurityDenial::SessionInactive)
        );
    }

    #[test]
    fn unregistered_capabilities_are_denied() {
        let gateway = SecurityGateway::new(vec![]);

        assert_eq!(
            gateway.authorize_at(1_000, Some(&session()), &request(), false),
            SecurityDecision::Deny(SecurityDenial::CapabilityNotRegistered)
        );
    }

    #[test]
    fn high_risk_requires_explicit_confirmation() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::High)]);

        assert_eq!(
            gateway.authorize_at(1_000, Some(&session()), &request(), false),
            SecurityDecision::RequireConfirmation
        );

        assert_eq!(
            gateway.authorize_at(1_000, Some(&session()), &request(), true),
            SecurityDecision::Allow
        );
    }

    #[test]
    fn registered_low_risk_request_is_permitted() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);

        assert_eq!(
            gateway.authorize_at(1_000, Some(&session()), &request(), false),
            SecurityDecision::Allow
        );
    }

    #[test]
    fn empty_capability_ids_are_denied() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);
        let request = CapabilityRequest {
            capability_id: "  ".to_string(),
        };

        assert_eq!(
            gateway.authorize_at(1_000, Some(&session()), &request, false),
            SecurityDecision::Deny(SecurityDenial::EmptyCapabilityId)
        );
    }

    #[test]
    fn caller_cannot_raise_or_lower_host_policy_risk() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::High)]);

        assert_eq!(
            gateway.authorize_at(1_000, Some(&session()), &request(), false),
            SecurityDecision::RequireConfirmation
        );
    }

    #[test]
    fn required_permissions_are_host_owned() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);

        assert_eq!(
            gateway.required_permissions("system.telemetry.read"),
            Ok([Permission::SystemTelemetryRead].as_slice())
        );
    }

    #[test]
    fn required_permissions_reject_unknown_capabilities() {
        let gateway = SecurityGateway::new(vec![]);

        assert_eq!(
            gateway.required_permissions("unknown"),
            Err(SecurityDenial::CapabilityNotRegistered)
        );
    }

    #[test]
    fn authenticated_trusted_device_with_required_permission_is_allowed() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);
        let devices = trusted_device();

        assert_eq!(
            gateway.authorize_for_device_at(
                1_000,
                Some(&session()),
                "pi-01",
                &request(),
                false,
                &devices,
            ),
            SecurityDecision::Allow
        );
    }

    #[test]
    fn authenticated_untrusted_device_is_denied() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);
        let mut devices = trusted_device();
        devices
            .set_trust("pi-01", DeviceTrust::Quarantined)
            .expect("device exists");

        assert_eq!(
            gateway.authorize_for_device_at(
                1_000,
                Some(&session()),
                "pi-01",
                &request(),
                false,
                &devices,
            ),
            SecurityDecision::Deny(SecurityDenial::DeviceNotTrusted)
        );
    }

    #[test]
    fn authenticated_device_without_required_permission_is_denied() {
        let mut devices = DeviceRegistry::new();
        devices
            .register(
                DeviceRecord::new(
                    "pi-02",
                    DeviceTrust::Trusted,
                    [Permission::SystemTelemetryRead],
                )
                .expect("valid device"),
            )
            .expect("registration succeeds");

        let gateway = SecurityGateway::new(vec![CapabilityPolicy::new(
            "system.telemetry.read",
            vec![Permission::FilesystemWrite],
            RiskLevel::Low,
        )]);

        assert_eq!(
            gateway.authorize_for_device_at(
                1_000,
                Some(&session()),
                "pi-02",
                &request(),
                false,
                &devices,
            ),
            SecurityDecision::Deny(SecurityDenial::DevicePermissionNotGranted)
        );
    }

    #[test]
    fn authentication_fails_before_device_checks() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);
        let devices = trusted_device();

        assert_eq!(
            gateway.authorize_for_device_at(
                1_000,
                None,
                "unknown-device",
                &request(),
                false,
                &devices,
            ),
            SecurityDecision::Deny(SecurityDenial::Unauthenticated)
        );
    }
}

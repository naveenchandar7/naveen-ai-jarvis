use serde::{Deserialize, Serialize};

/// Version of the host-side security contract.
///
/// This is intentionally independent of any future IPC wire protocol.
pub const SECURITY_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub capability_id: String,
    pub permissions: Vec<Permission>,
    pub risk: RiskLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedSession {
    session_id: String,
}

impl AuthenticatedSession {
    /// Constructed only after an authentication mechanism has accepted a session.
    /// The actual credential/token mechanism remains deliberately open.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
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
    EmptyCapabilityId,
    PermissionNotGranted(Permission),
    HighRiskRequiresConfirmation,
}

/// Minimal deny-by-default Security Gateway foundation.
///
/// This is deliberately policy-only: it does not execute OS operations,
/// manage secrets, or choose an authentication/token implementation.
pub struct SecurityGateway {
    allowed_permissions: Vec<Permission>,
}

impl SecurityGateway {
    pub fn new(allowed_permissions: Vec<Permission>) -> Self {
        Self { allowed_permissions }
    }

    pub fn authorize(
        &self,
        session: Option<&AuthenticatedSession>,
        request: &CapabilityRequest,
        confirmed: bool,
    ) -> SecurityDecision {
        if session.is_none() {
            return SecurityDecision::Deny(SecurityDenial::Unauthenticated);
        }

        if request.capability_id.trim().is_empty() {
            return SecurityDecision::Deny(SecurityDenial::EmptyCapabilityId);
        }

        if let Some(permission) = request
            .permissions
            .iter()
            .find(|permission| !self.allowed_permissions.contains(permission))
        {
            return SecurityDecision::Deny(SecurityDenial::PermissionNotGranted(
                permission.clone(),
            ));
        }

        if request.risk == RiskLevel::High && !confirmed {
            return SecurityDecision::RequireConfirmation;
        }

        SecurityDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> AuthenticatedSession {
        AuthenticatedSession::new("test-session")
    }

    fn request(risk: RiskLevel) -> CapabilityRequest {
        CapabilityRequest {
            capability_id: "system.telemetry.read".to_string(),
            permissions: vec![Permission::SystemTelemetryRead],
            risk,
        }
    }

    #[test]
    fn unauthenticated_requests_are_denied() {
        let gateway = SecurityGateway::new(vec![Permission::SystemTelemetryRead]);

        assert_eq!(
            gateway.authorize(None, &request(RiskLevel::Low), false),
            SecurityDecision::Deny(SecurityDenial::Unauthenticated)
        );
    }

    #[test]
    fn ungranted_permissions_are_denied() {
        let gateway = SecurityGateway::new(vec![]);

        assert_eq!(
            gateway.authorize(Some(&session()), &request(RiskLevel::Low), false),
            SecurityDecision::Deny(SecurityDenial::PermissionNotGranted(
                Permission::SystemTelemetryRead
            ))
        );
    }

    #[test]
    fn high_risk_requires_explicit_confirmation() {
        let gateway = SecurityGateway::new(vec![Permission::SystemTelemetryRead]);

        assert_eq!(
            gateway.authorize(Some(&session()), &request(RiskLevel::High), false),
            SecurityDecision::RequireConfirmation
        );

        assert_eq!(
            gateway.authorize(Some(&session()), &request(RiskLevel::High), true),
            SecurityDecision::Allow
        );
    }

    #[test]
    fn allowed_low_risk_request_is_permitted() {
        let gateway = SecurityGateway::new(vec![Permission::SystemTelemetryRead]);

        assert_eq!(
            gateway.authorize(Some(&session()), &request(RiskLevel::Low), false),
            SecurityDecision::Allow
        );
    }

    #[test]
    fn empty_capability_ids_are_denied() {
        let gateway = SecurityGateway::new(vec![Permission::SystemTelemetryRead]);
        let mut request = request(RiskLevel::Low);
        request.capability_id = "  ".to_string();

        assert_eq!(
            gateway.authorize(Some(&session()), &request, false),
            SecurityDecision::Deny(SecurityDenial::EmptyCapabilityId)
        );
    }
}

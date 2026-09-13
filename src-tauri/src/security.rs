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

/// Host-owned capability policy. The caller cannot choose its own permissions
/// or risk level; those values come from this policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPolicy {
    pub capability_id: String,
    pub permissions: Vec<Permission>,
    pub risk: RiskLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub capability_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedSession {
    session_id: String,
}

impl AuthenticatedSession {
    /// Constructed by the host after a future authentication mechanism accepts
    /// a session. The credential/token mechanism remains intentionally open.
    pub(crate) fn new(session_id: impl Into<String>) -> Self {
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
    CapabilityNotRegistered,
    HighRiskRequiresConfirmation,
}

/// Minimal deny-by-default Security Gateway foundation.
///
/// Policy is host-owned. This layer does not execute OS operations, manage
/// secrets, or choose an authentication/token implementation.
pub struct SecurityGateway {
    policies: Vec<CapabilityPolicy>,
}

impl SecurityGateway {
    pub fn new(policies: Vec<CapabilityPolicy>) -> Self {
        Self { policies }
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

        let Some(policy) = self
            .policies
            .iter()
            .find(|policy| policy.capability_id == request.capability_id)
        else {
            return SecurityDecision::Deny(SecurityDenial::CapabilityNotRegistered);
        };

        if policy.risk == RiskLevel::High && !confirmed {
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

    fn policy(risk: RiskLevel) -> CapabilityPolicy {
        CapabilityPolicy {
            capability_id: "system.telemetry.read".to_string(),
            permissions: vec![Permission::SystemTelemetryRead],
            risk,
        }
    }

    fn request() -> CapabilityRequest {
        CapabilityRequest {
            capability_id: "system.telemetry.read".to_string(),
        }
    }

    #[test]
    fn unauthenticated_requests_are_denied() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);

        assert_eq!(
            gateway.authorize(None, &request(), false),
            SecurityDecision::Deny(SecurityDenial::Unauthenticated)
        );
    }

    #[test]
    fn unregistered_capabilities_are_denied() {
        let gateway = SecurityGateway::new(vec![]);

        assert_eq!(
            gateway.authorize(Some(&session()), &request(), false),
            SecurityDecision::Deny(SecurityDenial::CapabilityNotRegistered)
        );
    }

    #[test]
    fn high_risk_requires_explicit_confirmation() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::High)]);

        assert_eq!(
            gateway.authorize(Some(&session()), &request(), false),
            SecurityDecision::RequireConfirmation
        );

        assert_eq!(
            gateway.authorize(Some(&session()), &request(), true),
            SecurityDecision::Allow
        );
    }

    #[test]
    fn registered_low_risk_request_is_permitted() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::Low)]);

        assert_eq!(
            gateway.authorize(Some(&session()), &request(), false),
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
            gateway.authorize(Some(&session()), &request, false),
            SecurityDecision::Deny(SecurityDenial::EmptyCapabilityId)
        );
    }

    #[test]
    fn caller_cannot_raise_or_lower_host_policy_risk() {
        let gateway = SecurityGateway::new(vec![policy(RiskLevel::High)]);

        assert_eq!(
            gateway.authorize(Some(&session()), &request(), false),
            SecurityDecision::RequireConfirmation
        );
    }
}

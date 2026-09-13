use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub const AUTH_PROTOCOL_VERSION: u16 = 1;
pub const AUTH_LAUNCH_ID_LEN: usize = 16;
pub const AUTH_CHALLENGE_ID_LEN: usize = 16;
pub const AUTH_CHALLENGE_NONCE_LEN: usize = 32;
pub const AUTH_SESSION_ID_LEN: usize = 16;
pub const AUTH_SECRET_LEN: usize = 32;
pub const AUTH_PROOF_LEN: usize = 32;
pub const AUTH_CHALLENGE_TTL_MS: u64 = 30_000;
pub const AUTH_SESSION_TTL_MS: u64 = 10 * 60 * 1_000;
pub const AUTH_MAX_CLIENT_ID_LEN: usize = 64;
pub const AUTH_MAX_CORRELATION_ID_LEN: usize = 128;
pub const AUTH_MAX_PENDING_CHALLENGES: usize = 64;
pub const AUTH_MAX_AUDIT_EVENTS: usize = 256;

const AUTH_DOMAIN: &[u8] = b"NAVEEN-AUTH-V1";
const SESSION_DOMAIN: &[u8] = b"NAVEEN-SESSION-V1";
const MESSAGE_DOMAIN: &[u8] = b"NAVEEN-MESSAGE-V1";
const CORE_CLIENT_ID: &str = "naveen-ai-core";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LaunchId([u8; AUTH_LAUNCH_ID_LEN]);

impl LaunchId {
    pub(crate) fn from_bytes(bytes: [u8; AUTH_LAUNCH_ID_LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; AUTH_LAUNCH_ID_LEN] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChallengeId([u8; AUTH_CHALLENGE_ID_LEN]);

impl ChallengeId {
    pub(crate) fn from_bytes(bytes: [u8; AUTH_CHALLENGE_ID_LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; AUTH_CHALLENGE_ID_LEN] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId([u8; AUTH_SESSION_ID_LEN]);

impl SessionId {
    pub fn as_bytes(&self) -> &[u8; AUTH_SESSION_ID_LEN] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex_encode(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthChallenge {
    pub protocol_version: u16,
    pub launch_id: LaunchId,
    pub challenge_id: ChallengeId,
    pub nonce: [u8; AUTH_CHALLENGE_NONCE_LEN],
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub expected_client_id: String,
    pub correlation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeResponse {
    pub protocol_version: u16,
    pub launch_id: LaunchId,
    pub challenge_id: ChallengeId,
    pub client_id: String,
    pub correlation_id: String,
    pub proof: [u8; AUTH_PROOF_LEN],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthFailureReason {
    UnsupportedProtocolVersion,
    MalformedInput,
    UnknownChallenge,
    ChallengeExpired,
    ChallengeReplayed,
    LaunchIdentityMismatch,
    CorrelationMismatch,
    ClientIdentityMismatch,
    InvalidProof,
    SessionExpired,
    SessionRevoked,
    UnknownSession,
    InvalidSequence,
    ReplayDetected,
    CryptoUnavailable,
    CryptoFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    UnsupportedProtocolVersion { expected: u16, received: u16 },
    MalformedInput(&'static str),
    UnknownChallenge,
    ChallengeExpired,
    ChallengeReplayed,
    LaunchIdentityMismatch,
    CorrelationMismatch,
    ClientIdentityMismatch,
    InvalidProof,
    SessionExpired,
    SessionRevoked,
    UnknownSession,
    InvalidSequence,
    ReplayDetected,
    CryptoUnavailable,
    CryptoFailure,
}

impl AuthError {
    fn audit_reason(&self) -> AuthFailureReason {
        match self {
            Self::UnsupportedProtocolVersion { .. } => {
                AuthFailureReason::UnsupportedProtocolVersion
            }
            Self::MalformedInput(_) => AuthFailureReason::MalformedInput,
            Self::UnknownChallenge => AuthFailureReason::UnknownChallenge,
            Self::ChallengeExpired => AuthFailureReason::ChallengeExpired,
            Self::ChallengeReplayed => AuthFailureReason::ChallengeReplayed,
            Self::LaunchIdentityMismatch => AuthFailureReason::LaunchIdentityMismatch,
            Self::CorrelationMismatch => AuthFailureReason::CorrelationMismatch,
            Self::ClientIdentityMismatch => AuthFailureReason::ClientIdentityMismatch,
            Self::InvalidProof => AuthFailureReason::InvalidProof,
            Self::SessionExpired => AuthFailureReason::SessionExpired,
            Self::SessionRevoked => AuthFailureReason::SessionRevoked,
            Self::UnknownSession => AuthFailureReason::UnknownSession,
            Self::InvalidSequence => AuthFailureReason::InvalidSequence,
            Self::ReplayDetected => AuthFailureReason::ReplayDetected,
            Self::CryptoUnavailable => AuthFailureReason::CryptoUnavailable,
            Self::CryptoFailure => AuthFailureReason::CryptoFailure,
        }
    }
}

pub trait AuthCrypto {
    fn fill_random(&self, output: &mut [u8]) -> Result<(), AuthError>;
    fn hmac_sha256(&self, key: &[u8], message: &[u8]) -> Result<[u8; 32], AuthError>;
}

#[cfg(windows)]
#[derive(Clone, Copy, Default)]
pub struct PlatformCrypto;

#[cfg(not(windows))]
#[derive(Clone, Copy, Default)]
pub struct PlatformCrypto;

#[cfg(windows)]
impl AuthCrypto for PlatformCrypto {
    fn fill_random(&self, output: &mut [u8]) -> Result<(), AuthError> {
        type AlgHandle = *mut std::ffi::c_void;

        #[link(name = "bcrypt")]
        extern "system" {
            fn BCryptGenRandom(
                h_algorithm: AlgHandle,
                buffer: *mut u8,
                buffer_len: u32,
                flags: u32,
            ) -> i32;
        }

        const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;

        let buffer_len = u32::try_from(output.len()).map_err(|_| AuthError::CryptoFailure)?;
        let status = unsafe {
            BCryptGenRandom(
                std::ptr::null_mut(),
                output.as_mut_ptr(),
                buffer_len,
                BCRYPT_USE_SYSTEM_PREFERRED_RNG,
            )
        };

        if status == 0 {
            Ok(())
        } else {
            Err(AuthError::CryptoFailure)
        }
    }

    fn hmac_sha256(&self, key: &[u8], message: &[u8]) -> Result<[u8; 32], AuthError> {
        type AlgHandle = *mut std::ffi::c_void;

        #[link(name = "bcrypt")]
        extern "system" {
            fn BCryptOpenAlgorithmProvider(
                algorithm: *mut AlgHandle,
                algorithm_name: *const u16,
                implementation: *const u16,
                flags: u32,
            ) -> i32;
            fn BCryptHash(
                algorithm: AlgHandle,
                secret: *mut u8,
                secret_len: u32,
                input: *mut u8,
                input_len: u32,
                output: *mut u8,
                output_len: u32,
            ) -> i32;
            fn BCryptCloseAlgorithmProvider(
                algorithm: AlgHandle,
                flags: u32,
            ) -> i32;
        }

        const BCRYPT_ALG_HANDLE_HMAC_FLAG: u32 = 0x0000_0008;
        const BCRYPT_SHA256_ALGORITHM: [u16; 7] = [
            b'S' as u16,
            b'H' as u16,
            b'A' as u16,
            b'2' as u16,
            b'5' as u16,
            b'6' as u16,
            0,
        ];

        let key_len = u32::try_from(key.len()).map_err(|_| AuthError::CryptoFailure)?;
        let message_len = u32::try_from(message.len()).map_err(|_| AuthError::CryptoFailure)?;
        let mut algorithm: AlgHandle = std::ptr::null_mut();

        let status = unsafe {
            BCryptOpenAlgorithmProvider(
                &mut algorithm,
                BCRYPT_SHA256_ALGORITHM.as_ptr(),
                std::ptr::null(),
                BCRYPT_ALG_HANDLE_HMAC_FLAG,
            )
        };

        if status != 0 {
            return Err(AuthError::CryptoFailure);
        }

        let mut output = [0_u8; 32];
        let hash_status = unsafe {
            BCryptHash(
                algorithm,
                key.as_ptr() as *mut u8,
                key_len,
                message.as_ptr() as *mut u8,
                message_len,
                output.as_mut_ptr(),
                output.len() as u32,
            )
        };

        let _ = unsafe { BCryptCloseAlgorithmProvider(algorithm, 0) };

        if hash_status == 0 {
            Ok(output)
        } else {
            Err(AuthError::CryptoFailure)
        }
    }
}

#[cfg(not(windows))]
impl AuthCrypto for PlatformCrypto {
    fn fill_random(&self, _output: &mut [u8]) -> Result<(), AuthError> {
        Err(AuthError::CryptoUnavailable)
    }

    fn hmac_sha256(&self, _key: &[u8], _message: &[u8]) -> Result<[u8; 32], AuthError> {
        Err(AuthError::CryptoUnavailable)
    }
}

struct SecretKey([u8; AUTH_SECRET_LEN]);

impl SecretKey {
    fn new(bytes: [u8; AUTH_SECRET_LEN]) -> Self {
        Self(bytes)
    }

    fn as_bytes(&self) -> &[u8; AUTH_SECRET_LEN] {
        &self.0
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        secure_zero(&mut self.0);
    }
}

struct SessionState {
    issued_at_ms: u64,
    expires_at_ms: u64,
    revoked: AtomicBool,
    last_sequence: Mutex<u64>,
    session_key: SecretKey,
}

impl SessionState {
    fn is_active_at(&self, now_ms: u64) -> bool {
        !self.revoked.load(Ordering::Acquire) && now_ms < self.expires_at_ms
    }
}

#[derive(Clone)]
pub struct AuthenticatedSession {
    session_id: SessionId,
    state: Arc<SessionState>,
}

impl AuthenticatedSession {
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn is_active_at(&self, now_ms: u64) -> bool {
        self.state.is_active_at(now_ms)
    }

    pub fn issued_at_ms(&self) -> u64 {
        self.state.issued_at_ms
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.state.expires_at_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthAuditEventType {
    ChallengeIssued,
    AuthenticationSucceeded,
    AuthenticationFailed,
    SessionExpired,
    SessionInvalidated,
    SessionMessageRejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthAuditEvent {
    pub event_type: AuthAuditEventType,
    pub timestamp_ms: u64,
    pub protocol_version: u16,
    pub correlation_id: Option<String>,
    pub session_id: Option<SessionId>,
    pub reason: Option<AuthFailureReason>,
}

struct ChallengeState {
    challenge: AuthChallenge,
    consumed: bool,
}

pub trait AuthenticationProvider {
    fn issue_challenge(
        &mut self,
        protocol_version: u16,
        correlation_id: &str,
        now_ms: u64,
    ) -> Result<AuthChallenge, AuthError>;

    fn complete_challenge(
        &mut self,
        response: &ChallengeResponse,
        now_ms: u64,
    ) -> Result<AuthenticatedSession, AuthError>;

    fn verify_session_message(
        &mut self,
        session: &AuthenticatedSession,
        protocol_version: u16,
        correlation_id: &str,
        sequence: u64,
        message_digest: &[u8],
        proof: &[u8],
        now_ms: u64,
    ) -> Result<(), AuthError>;

    fn invalidate_session(&mut self, session: &AuthenticatedSession, now_ms: u64) -> bool;
    fn invalidate_all_sessions(&mut self, now_ms: u64);
}

pub struct AuthenticationServer<C: AuthCrypto = PlatformCrypto> {
    crypto: C,
    launch_id: LaunchId,
    launch_secret: SecretKey,
    expected_client_id: String,
    challenges: HashMap<ChallengeId, ChallengeState>,
    sessions: HashMap<SessionId, AuthenticatedSession>,
    audit_events: VecDeque<AuthAuditEvent>,
}

impl AuthenticationServer<PlatformCrypto> {
    pub fn new() -> Result<Self, AuthError> {
        Self::with_crypto(PlatformCrypto, CORE_CLIENT_ID)
    }
}

impl<C: AuthCrypto> AuthenticationServer<C> {
    pub fn with_crypto(crypto: C, expected_client_id: &str) -> Result<Self, AuthError> {
        validate_text(expected_client_id, AUTH_MAX_CLIENT_ID_LEN, "client_id")?;

        let mut launch_id = [0_u8; AUTH_LAUNCH_ID_LEN];
        crypto.fill_random(&mut launch_id)?;

        let mut launch_secret = [0_u8; AUTH_SECRET_LEN];
        crypto.fill_random(&mut launch_secret)?;

        Ok(Self {
            crypto,
            launch_id: LaunchId::from_bytes(launch_id),
            launch_secret: SecretKey::new(launch_secret),
            expected_client_id: expected_client_id.to_string(),
            challenges: HashMap::new(),
            sessions: HashMap::new(),
            audit_events: VecDeque::with_capacity(AUTH_MAX_AUDIT_EVENTS),
        })
    }

    pub fn launch_id(&self) -> LaunchId {
        self.launch_id
    }

    pub fn expected_client_id(&self) -> &str {
        &self.expected_client_id
    }

    pub(crate) fn bootstrap_secret(&self) -> [u8; AUTH_SECRET_LEN] {
        self.launch_secret.0
    }

    pub fn take_audit_events(&mut self) -> Vec<AuthAuditEvent> {
        self.audit_events.drain(..).collect()
    }

    fn record(&mut self, event: AuthAuditEvent) {
        if self.audit_events.len() == AUTH_MAX_AUDIT_EVENTS {
            self.audit_events.pop_front();
        }
        self.audit_events.push_back(event);
    }

    fn record_failure(
        &mut self,
        now_ms: u64,
        response: &ChallengeResponse,
        error: &AuthError,
    ) {
        let correlation_id = if validate_text(
            &response.correlation_id,
            AUTH_MAX_CORRELATION_ID_LEN,
            "correlation_id",
        )
        .is_ok()
        {
            Some(response.correlation_id.clone())
        } else {
            None
        };

        self.record(AuthAuditEvent {
            event_type: AuthAuditEventType::AuthenticationFailed,
            timestamp_ms: now_ms,
            protocol_version: response.protocol_version,
            correlation_id,
            session_id: None,
            reason: Some(error.audit_reason()),
        });
    }

    fn new_unique_session_id(&self) -> Result<SessionId, AuthError> {
        for _ in 0..8 {
            let mut bytes = [0_u8; AUTH_SESSION_ID_LEN];
            self.crypto.fill_random(&mut bytes)?;
            let candidate = SessionId(bytes);
            if !self.sessions.contains_key(&candidate) {
                return Ok(candidate);
            }
        }
        Err(AuthError::CryptoFailure)
    }

    fn new_unique_challenge_id(&self) -> Result<ChallengeId, AuthError> {
        for _ in 0..8 {
            let mut bytes = [0_u8; AUTH_CHALLENGE_ID_LEN];
            self.crypto.fill_random(&mut bytes)?;
            let candidate = ChallengeId(bytes);
            if !self.challenges.contains_key(&candidate) {
                return Ok(candidate);
            }
        }
        Err(AuthError::CryptoFailure)
    }
}

impl<C: AuthCrypto> AuthenticationProvider for AuthenticationServer<C> {
    fn issue_challenge(
        &mut self,
        protocol_version: u16,
        correlation_id: &str,
        now_ms: u64,
    ) -> Result<AuthChallenge, AuthError> {
        if protocol_version != AUTH_PROTOCOL_VERSION {
            return Err(AuthError::UnsupportedProtocolVersion {
                expected: AUTH_PROTOCOL_VERSION,
                received: protocol_version,
            });
        }

        validate_text(
            correlation_id,
            AUTH_MAX_CORRELATION_ID_LEN,
            "correlation_id",
        )?;

        if self.challenges.len() >= AUTH_MAX_PENDING_CHALLENGES {
            if let Some(oldest) = self
                .challenges
                .iter()
                .min_by_key(|(_, state)| state.challenge.issued_at_ms)
                .map(|(id, _)| *id)
            {
                self.challenges.remove(&oldest);
            }
        }

        let challenge_id = self.new_unique_challenge_id()?;
        let mut nonce = [0_u8; AUTH_CHALLENGE_NONCE_LEN];
        self.crypto.fill_random(&mut nonce)?;

        let challenge = AuthChallenge {
            protocol_version: AUTH_PROTOCOL_VERSION,
            launch_id: self.launch_id,
            challenge_id,
            nonce,
            issued_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(AUTH_CHALLENGE_TTL_MS),
            expected_client_id: self.expected_client_id.clone(),
            correlation_id: correlation_id.to_string(),
        };

        self.challenges.insert(
            challenge_id,
            ChallengeState {
                challenge: challenge.clone(),
                consumed: false,
            },
        );

        self.record(AuthAuditEvent {
            event_type: AuthAuditEventType::ChallengeIssued,
            timestamp_ms: now_ms,
            protocol_version: AUTH_PROTOCOL_VERSION,
            correlation_id: Some(correlation_id.to_string()),
            session_id: None,
            reason: None,
        });

        Ok(challenge)
    }

    fn complete_challenge(
        &mut self,
        response: &ChallengeResponse,
        now_ms: u64,
    ) -> Result<AuthenticatedSession, AuthError> {
        let result = (|| {
            if response.protocol_version != AUTH_PROTOCOL_VERSION {
                return Err(AuthError::UnsupportedProtocolVersion {
                    expected: AUTH_PROTOCOL_VERSION,
                    received: response.protocol_version,
                });
            }

            validate_text(&response.client_id, AUTH_MAX_CLIENT_ID_LEN, "client_id")?;
            validate_text(
                &response.correlation_id,
                AUTH_MAX_CORRELATION_ID_LEN,
                "correlation_id",
            )?;

            let challenge = self
                .challenges
                .get(&response.challenge_id)
                .ok_or(AuthError::UnknownChallenge)?
                .challenge
                .clone();

            let consumed = self
                .challenges
                .get(&response.challenge_id)
                .map(|state| state.consumed)
                .unwrap_or(false);

            if challenge.launch_id != response.launch_id {
                return Err(AuthError::LaunchIdentityMismatch);
            }

            if consumed {
                return Err(AuthError::ChallengeReplayed);
            }

            if now_ms < challenge.issued_at_ms || now_ms >= challenge.expires_at_ms {
                return Err(AuthError::ChallengeExpired);
            }

            if response.correlation_id != challenge.correlation_id {
                return Err(AuthError::CorrelationMismatch);
            }

            if response.client_id != challenge.expected_client_id
                || response.client_id != self.expected_client_id
            {
                return Err(AuthError::ClientIdentityMismatch);
            }

            let transcript = challenge_transcript(&challenge);
            let expected = self
                .crypto
                .hmac_sha256(self.launch_secret.as_bytes(), &transcript)?;

            if !constant_time_eq(&expected, &response.proof) {
                return Err(AuthError::InvalidProof);
            }

            let session_id = self.new_unique_session_id()?;
            let session_material = session_key_material(&transcript, &session_id);
            let session_key = self
                .crypto
                .hmac_sha256(self.launch_secret.as_bytes(), &session_material)?;

            if let Some(challenge_state) = self.challenges.get_mut(&response.challenge_id) {
                challenge_state.consumed = true;
            } else {
                return Err(AuthError::UnknownChallenge);
            }

            let state = Arc::new(SessionState {
                issued_at_ms: now_ms,
                expires_at_ms: now_ms.saturating_add(AUTH_SESSION_TTL_MS),
                revoked: AtomicBool::new(false),
                last_sequence: Mutex::new(0),
                session_key: SecretKey::new(session_key),
            });

            let session = AuthenticatedSession { session_id, state };
            self.sessions.insert(session_id, session.clone());

            self.record(AuthAuditEvent {
                event_type: AuthAuditEventType::AuthenticationSucceeded,
                timestamp_ms: now_ms,
                protocol_version: AUTH_PROTOCOL_VERSION,
                correlation_id: Some(response.correlation_id.clone()),
                session_id: Some(session_id),
                reason: None,
            });

            Ok(session)
        })();

        if let Err(error) = &result {
            self.record_failure(now_ms, response, error);
        }

        result
    }

    fn verify_session_message(
        &mut self,
        session: &AuthenticatedSession,
        protocol_version: u16,
        correlation_id: &str,
        sequence: u64,
        message_digest: &[u8],
        proof: &[u8],
        now_ms: u64,
    ) -> Result<(), AuthError> {
        let result = (|| {
            if protocol_version != AUTH_PROTOCOL_VERSION {
                return Err(AuthError::UnsupportedProtocolVersion {
                    expected: AUTH_PROTOCOL_VERSION,
                    received: protocol_version,
                });
            }

            validate_text(
                correlation_id,
                AUTH_MAX_CORRELATION_ID_LEN,
                "correlation_id",
            )?;

            if sequence == 0 {
                return Err(AuthError::InvalidSequence);
            }

            if proof.len() != AUTH_PROOF_LEN {
                return Err(AuthError::MalformedInput("proof"));
            }

            let Some(registered) = self.sessions.get(&session.session_id) else {
                return Err(AuthError::UnknownSession);
            };

            if !Arc::ptr_eq(&registered.state, &session.state) {
                return Err(AuthError::UnknownSession);
            }

            if registered.state.revoked.load(Ordering::Acquire) {
                return Err(AuthError::SessionRevoked);
            }

            if now_ms >= registered.state.expires_at_ms {
                registered.state.revoked.store(true, Ordering::Release);
                return Err(AuthError::SessionExpired);
            }

            let transcript = message_transcript(
                protocol_version,
                &session.session_id,
                correlation_id,
                sequence,
                message_digest,
            );
            let expected = self
                .crypto
                .hmac_sha256(registered.state.session_key.as_bytes(), &transcript)?;

            if !constant_time_slice_eq(&expected, proof) {
                return Err(AuthError::InvalidProof);
            }

            let mut last_sequence = registered
                .state
                .last_sequence
                .lock()
                .map_err(|_| AuthError::CryptoFailure)?;

            if sequence <= *last_sequence {
                return Err(AuthError::ReplayDetected);
            }

            *last_sequence = sequence;
            Ok(())
        })();

        if let Err(error) = &result {
            let event_type = match error {
                AuthError::SessionExpired => AuthAuditEventType::SessionExpired,
                _ => AuthAuditEventType::SessionMessageRejected,
            };

            self.record(AuthAuditEvent {
                event_type,
                timestamp_ms: now_ms,
                protocol_version,
                correlation_id: if validate_text(
                    correlation_id,
                    AUTH_MAX_CORRELATION_ID_LEN,
                    "correlation_id",
                )
                .is_ok()
                {
                    Some(correlation_id.to_string())
                } else {
                    None
                },
                session_id: Some(session.session_id),
                reason: Some(error.audit_reason()),
            });
        }

        result
    }

    fn invalidate_session(&mut self, session: &AuthenticatedSession, now_ms: u64) -> bool {
        let Some(registered) = self.sessions.get(&session.session_id) else {
            return false;
        };

        if !Arc::ptr_eq(&registered.state, &session.state) {
            return false;
        }

        let was_active = registered.state.is_active_at(now_ms);
        registered.state.revoked.store(true, Ordering::Release);

        if was_active {
            self.record(AuthAuditEvent {
                event_type: AuthAuditEventType::SessionInvalidated,
                timestamp_ms: now_ms,
                protocol_version: AUTH_PROTOCOL_VERSION,
                correlation_id: None,
                session_id: Some(session.session_id),
                reason: Some(AuthFailureReason::SessionRevoked),
            });
        }

        was_active
    }

    fn invalidate_all_sessions(&mut self, now_ms: u64) {
        let session_ids: Vec<SessionId> = self.sessions.keys().copied().collect();

        for session_id in session_ids {
            if let Some(session) = self.sessions.get(&session_id).cloned() {
                self.invalidate_session(&session, now_ms);
            }
        }
    }
}

fn challenge_transcript(challenge: &AuthChallenge) -> Vec<u8> {
    let mut output = Vec::with_capacity(256);
    output.extend_from_slice(AUTH_DOMAIN);
    push_u16(&mut output, challenge.protocol_version);
    push_bytes(&mut output, challenge.launch_id.as_bytes());
    push_bytes(&mut output, challenge.challenge_id.as_bytes());
    push_bytes(&mut output, &challenge.nonce);
    push_u64(&mut output, challenge.issued_at_ms);
    push_u64(&mut output, challenge.expires_at_ms);
    push_string(&mut output, &challenge.expected_client_id);
    push_string(&mut output, &challenge.correlation_id);
    output
}

fn session_key_material(transcript: &[u8], session_id: &SessionId) -> Vec<u8> {
    let mut output = Vec::with_capacity(transcript.len() + 64);
    output.extend_from_slice(SESSION_DOMAIN);
    push_bytes(&mut output, transcript);
    push_bytes(&mut output, session_id.as_bytes());
    output
}

fn message_transcript(
    protocol_version: u16,
    session_id: &SessionId,
    correlation_id: &str,
    sequence: u64,
    message_digest: &[u8],
) -> Vec<u8> {
    let mut output = Vec::with_capacity(128 + message_digest.len());
    output.extend_from_slice(MESSAGE_DOMAIN);
    push_u16(&mut output, protocol_version);
    push_bytes(&mut output, session_id.as_bytes());
    push_string(&mut output, correlation_id);
    push_u64(&mut output, sequence);
    push_bytes(&mut output, message_digest);
    output
}

fn validate_text(value: &str, max_len: usize, field: &'static str) -> Result<(), AuthError> {
    if value.is_empty() || value.len() > max_len || value.bytes().any(|byte| byte == 0) {
        return Err(AuthError::MalformedInput(field));
    }
    Ok(())
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_bytes(output: &mut Vec<u8>, value: &[u8]) {
    let length = u32::try_from(value.len()).unwrap_or(u32::MAX);
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
}

fn push_string(output: &mut Vec<u8>, value: &str) {
    push_bytes(output, value.as_bytes());
}

fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    let mut difference = 0_u8;
    for index in 0..32 {
        difference |= left[index] ^ right[index];
    }
    difference == 0
}

fn constant_time_slice_eq(expected: &[u8; 32], actual: &[u8]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }

    let mut difference = 0_u8;
    for index in 0..expected.len() {
        difference |= expected[index] ^ actual[index];
    }
    difference == 0
}

fn secure_zero(bytes: &mut [u8]) {
    for byte in bytes.iter_mut() {
        unsafe {
            std::ptr::write_volatile(byte, 0);
        }
    }
    std::sync::atomic::compiler_fence(Ordering::SeqCst);
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
fn proof_for_test<C: AuthCrypto>(
    server: &AuthenticationServer<C>,
    challenge: &AuthChallenge,
    client_id: &str,
) -> [u8; AUTH_PROOF_LEN] {
    let transcript = challenge_transcript(&AuthChallenge {
        expected_client_id: client_id.to_string(),
        ..challenge.clone()
    });
    server
        .crypto
        .hmac_sha256(server.launch_secret.as_bytes(), &transcript)
        .expect("test crypto")
}

#[cfg(test)]
fn session_message_proof_for_test<C: AuthCrypto>(
    server: &AuthenticationServer<C>,
    session: &AuthenticatedSession,
    protocol_version: u16,
    correlation_id: &str,
    sequence: u64,
    message_digest: &[u8],
) -> [u8; AUTH_PROOF_LEN] {
    let material = message_transcript(
        protocol_version,
        &session.session_id,
        correlation_id,
        sequence,
        message_digest,
    );
    let registered = server
        .sessions
        .get(&session.session_id)
        .expect("test session");
    server
        .crypto
        .hmac_sha256(registered.state.session_key.as_bytes(), &material)
        .expect("test crypto")
}

#[cfg(test)]
pub(crate) fn test_session(expires_at_ms: u64) -> AuthenticatedSession {
    let state = Arc::new(SessionState {
        issued_at_ms: 0,
        expires_at_ms,
        revoked: AtomicBool::new(false),
        last_sequence: Mutex::new(0),
        session_key: SecretKey::new([0x5A; AUTH_SECRET_LEN]),
    });
    AuthenticatedSession {
        session_id: SessionId([0xA5; AUTH_SESSION_ID_LEN]),
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU8;

    #[derive(Default)]
    struct DeterministicCrypto {
        counter: AtomicU8,
    }

    impl AuthCrypto for DeterministicCrypto {
        fn fill_random(&self, output: &mut [u8]) -> Result<(), AuthError> {
            let start = self.counter.fetch_add(1, Ordering::Relaxed);
            for (index, byte) in output.iter_mut().enumerate() {
                *byte = start.wrapping_add(index as u8).wrapping_add(1);
            }
            Ok(())
        }

        fn hmac_sha256(&self, key: &[u8], message: &[u8]) -> Result<[u8; 32], AuthError> {
            let mut state = 0x243F_6A88_85A3_08D3_u64;
            for byte in key.iter().chain(message.iter()) {
                state ^= u64::from(*byte);
                state = state.rotate_left(7).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            }

            let mut output = [0_u8; 32];
            for (index, byte) in output.iter_mut().enumerate() {
                state ^= u64::from(index as u8);
                state = state.rotate_left(11).wrapping_mul(0xD6E8_FEB8_6659_FD93);
                *byte = (state >> ((index % 8) * 8)) as u8;
            }
            Ok(output)
        }
    }

    fn server() -> AuthenticationServer<DeterministicCrypto> {
        AuthenticationServer::with_crypto(DeterministicCrypto::default(), CORE_CLIENT_ID)
            .expect("server")
    }

    fn response_for(
        server: &AuthenticationServer<DeterministicCrypto>,
        challenge: &AuthChallenge,
    ) -> ChallengeResponse {
        ChallengeResponse {
            protocol_version: AUTH_PROTOCOL_VERSION,
            launch_id: challenge.launch_id,
            challenge_id: challenge.challenge_id,
            client_id: challenge.expected_client_id.clone(),
            correlation_id: challenge.correlation_id.clone(),
            proof: proof_for_test(server, challenge, CORE_CLIENT_ID),
        }
    }

    #[test]
    fn valid_challenge_response_establishes_session() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-1", 1_000)
            .expect("challenge");
        let response = response_for(&server, &challenge);

        let session = server
            .complete_challenge(&response, 1_001)
            .expect("session");

        assert!(session.is_active_at(1_001));
        assert_ne!(session.session_id().as_bytes(), &[0_u8; AUTH_SESSION_ID_LEN]);
        assert_eq!(server.take_audit_events().len(), 2);
    }

    #[test]
    fn wrong_client_identity_is_rejected() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-2", 1_000)
            .expect("challenge");
        let mut response = response_for(&server, &challenge);
        response.client_id = "impostor".to_string();

        assert!(matches!(
            server.complete_challenge(&response, 1_001),
            Err(AuthError::ClientIdentityMismatch)
        ));
    }

    #[test]
    fn invalid_proof_is_rejected() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-3", 1_000)
            .expect("challenge");
        let mut response = response_for(&server, &challenge);
        response.proof[0] ^= 0xFF;

        assert!(matches!(
            server.complete_challenge(&response, 1_001),
            Err(AuthError::InvalidProof)
        ));
    }

    #[test]
    fn expired_challenge_is_rejected() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-4", 1_000)
            .expect("challenge");
        let response = response_for(&server, &challenge);

        assert!(matches!(
            server.complete_challenge(&response, challenge.expires_at_ms),
            Err(AuthError::ChallengeExpired)
        ));
    }

    #[test]
    fn consumed_challenge_cannot_be_replayed() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-5", 1_000)
            .expect("challenge");
        let response = response_for(&server, &challenge);
        let _session = server
            .complete_challenge(&response, 1_001)
            .expect("session");

        assert!(matches!(
            server.complete_challenge(&response, 1_002),
            Err(AuthError::ChallengeReplayed)
        ));
    }

    #[test]
    fn wrong_protocol_version_is_rejected() {
        let mut server = server();
        assert!(matches!(
            server.issue_challenge(99, "corr-6", 1_000),
            Err(AuthError::UnsupportedProtocolVersion {
                expected: AUTH_PROTOCOL_VERSION,
                received: 99,
            })
        ));
    }

    #[test]
    fn malformed_correlation_id_is_rejected() {
        let mut server = server();
        assert!(matches!(
            server.issue_challenge(AUTH_PROTOCOL_VERSION, "", 1_000),
            Err(AuthError::MalformedInput("correlation_id"))
        ));
    }

    #[test]
    fn launch_identity_mismatch_is_rejected() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-7", 1_000)
            .expect("challenge");
        let mut response = response_for(&server, &challenge);
        let mut launch_id = *response.launch_id.as_bytes();
        launch_id[0] ^= 0x01;
        response.launch_id = LaunchId::from_bytes(launch_id);

        assert!(matches!(
            server.complete_challenge(&response, 1_001),
            Err(AuthError::LaunchIdentityMismatch)
        ));
    }

    #[test]
    fn session_message_requires_valid_proof_and_monotonic_sequence() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-8", 1_000)
            .expect("challenge");
        let response = response_for(&server, &challenge);
        let session = server
            .complete_challenge(&response, 1_001)
            .expect("session");
        let digest = b"payload-digest";
        let proof = session_message_proof_for_test(
            &server,
            &session,
            AUTH_PROTOCOL_VERSION,
            "msg-1",
            1,
            digest,
        );

        server
            .verify_session_message(
                &session,
                AUTH_PROTOCOL_VERSION,
                "msg-1",
                1,
                digest,
                &proof,
                1_002,
            )
            .expect("message accepted");

        assert!(matches!(
            server.verify_session_message(
                &session,
                AUTH_PROTOCOL_VERSION,
                "msg-1",
                1,
                digest,
                &proof,
                1_003,
            ),
            Err(AuthError::ReplayDetected)
        ));
    }

    #[test]
    fn expired_session_is_rejected_and_revoked() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-9", 1_000)
            .expect("challenge");
        let response = response_for(&server, &challenge);
        let session = server
            .complete_challenge(&response, 1_001)
            .expect("session");
        let digest = b"payload-digest";
        let proof = session_message_proof_for_test(
            &server,
            &session,
            AUTH_PROTOCOL_VERSION,
            "msg-2",
            1,
            digest,
        );

        assert!(matches!(
            server.verify_session_message(
                &session,
                AUTH_PROTOCOL_VERSION,
                "msg-2",
                1,
                digest,
                &proof,
                session.expires_at_ms(),
            ),
            Err(AuthError::SessionExpired)
        ));
        assert!(!session.is_active_at(session.expires_at_ms()));
    }

    #[test]
    fn explicit_invalidation_blocks_later_use() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-10", 1_000)
            .expect("challenge");
        let response = response_for(&server, &challenge);
        let session = server
            .complete_challenge(&response, 1_001)
            .expect("session");
        assert!(server.invalidate_session(&session, 1_002));

        assert!(matches!(
            server.verify_session_message(
                &session,
                AUTH_PROTOCOL_VERSION,
                "msg-3",
                1,
                b"digest",
                &[0_u8; AUTH_PROOF_LEN],
                1_003,
            ),
            Err(AuthError::SessionRevoked)
        ));
    }

    #[test]
    fn old_session_is_not_valid_in_a_new_launch() {
        let mut first = server();
        let challenge = first
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-11", 1_000)
            .expect("challenge");
        let response = response_for(&first, &challenge);
        let session = first
            .complete_challenge(&response, 1_001)
            .expect("session");

        let mut second = server();

        assert!(matches!(
            second.verify_session_message(
                &session,
                AUTH_PROTOCOL_VERSION,
                "msg-4",
                1,
                b"digest",
                &[0_u8; AUTH_PROOF_LEN],
                1_002,
            ),
            Err(AuthError::UnknownSession)
        ));
    }

    #[test]
    fn malformed_session_message_is_rejected() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-12", 1_000)
            .expect("challenge");
        let response = response_for(&server, &challenge);
        let session = server
            .complete_challenge(&response, 1_001)
            .expect("session");

        assert!(matches!(
            server.verify_session_message(
                &session,
                AUTH_PROTOCOL_VERSION,
                "msg-5",
                0,
                b"digest",
                &[0_u8; AUTH_PROOF_LEN],
                1_002,
            ),
            Err(AuthError::InvalidSequence)
        ));

        assert!(matches!(
            server.verify_session_message(
                &session,
                AUTH_PROTOCOL_VERSION,
                "msg-6",
                1,
                b"digest",
                &[0_u8; AUTH_PROOF_LEN - 1],
                1_003,
            ),
            Err(AuthError::MalformedInput("proof"))
        ));
    }

    #[test]
    fn audit_events_contain_no_challenge_nonce_or_proof() {
        let mut server = server();
        let challenge = server
            .issue_challenge(AUTH_PROTOCOL_VERSION, "corr-13", 1_000)
            .expect("challenge");
        let mut response = response_for(&server, &challenge);
        let nonce_text = hex_encode(&challenge.nonce);
        let proof_text = hex_encode(&response.proof);
        response.proof[0] ^= 1;
        let _ = server.complete_challenge(&response, 1_001);

        let rendered = format!("{:?}", server.take_audit_events());
        assert!(!rendered.contains(&nonce_text));
        assert!(!rendered.contains(&proof_text));
        assert!(!rendered.contains("launch_secret"));
        assert!(!rendered.contains("session_key"));
    }
}

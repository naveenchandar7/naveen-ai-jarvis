use crate::auth::{AuthenticatedSession, AuthenticationProvider, AuthError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const IPC_PROTOCOL_VERSION: u16 = 1;
pub const IPC_MAX_WIRE_BODY_SIZE: usize = 1024 * 1024;
pub const IPC_MAX_PAYLOAD_SIZE: usize = 512 * 1024;
pub const IPC_MAX_CORRELATION_ID_LEN: usize = 128;
pub const IPC_MAX_METHOD_LEN: usize = 128;
pub const IPC_MAX_EVENT_ID_LEN: usize = 128;
pub const IPC_MAX_EVENT_TYPE_LEN: usize = 128;
pub const IPC_MAX_ERROR_CODE_LEN: usize = 128;
pub const IPC_MAX_AUDIT_EVENTS: usize = 256;
pub const IPC_DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

const IPC_MESSAGE_DOMAIN: &[u8] = b"NAVEEN-IPC-MESSAGE-V1";
const FRAME_HEADER_LEN: usize = 4;
const SESSION_ID_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcMessageKind {
    Request,
    Response,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub protocol_version: u16,
    pub correlation_id: String,
    pub session_id: [u8; SESSION_ID_LEN],
    pub sequence: u64,
    pub method: String,
    pub payload: Vec<u8>,
    pub proof: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Ok,
    Error,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub protocol_version: u16,
    pub correlation_id: String,
    pub session_id: [u8; SESSION_ID_LEN],
    pub sequence: u64,
    pub status: ResponseStatus,
    pub error_code: Option<String>,
    pub payload: Vec<u8>,
    /// The current foundation authenticates Core→Host requests cryptographically.
    /// Host→Core responses/events travel over the OS-private child channel and
    /// are correlation/session bound; proof signing remains an auth-layer extension.
    pub proof: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub protocol_version: u16,
    pub event_id: String,
    pub session_id: [u8; SESSION_ID_LEN],
    pub sequence: u64,
    pub event_type: String,
    pub payload: Vec<u8>,
    pub proof: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message")]
pub enum IpcEnvelope {
    Request(RequestEnvelope),
    Response(ResponseEnvelope),
    Event(EventEnvelope),
}

impl IpcEnvelope {
    pub fn kind(&self) -> IpcMessageKind {
        match self {
            Self::Request(_) => IpcMessageKind::Request,
            Self::Response(_) => IpcMessageKind::Response,
            Self::Event(_) => IpcMessageKind::Event,
        }
    }

    pub fn protocol_version(&self) -> u16 {
        match self {
            Self::Request(message) => message.protocol_version,
            Self::Response(message) => message.protocol_version,
            Self::Event(message) => message.protocol_version,
        }
    }

    pub fn session_id(&self) -> [u8; SESSION_ID_LEN] {
        match self {
            Self::Request(message) => message.session_id,
            Self::Response(message) => message.session_id,
            Self::Event(message) => message.session_id,
        }
    }

    pub fn correlation_id(&self) -> Option<&str> {
        match self {
            Self::Request(message) => Some(&message.correlation_id),
            Self::Response(message) => Some(&message.correlation_id),
            Self::Event(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    UnsupportedProtocolVersion { expected: u16, received: u16 },
    MalformedFrame(&'static str),
    OversizedFrame,
    Timeout,
    Closed,
    Transport(String),
    Authentication(AuthError),
    SessionMismatch,
    UnexpectedMessageKind,
    CorrelationMissing,
    DuplicateCorrelation,
    UnknownCorrelation,
    ResponseSequenceMismatch,
    InvalidEnvelope(&'static str),
}

impl From<AuthError> for IpcError {
    fn from(error: AuthError) -> Self {
        Self::Authentication(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcAuditEventType {
    FrameRejected,
    RequestAccepted,
    ConnectionClosed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcAuditEvent {
    pub event_type: IpcAuditEventType,
    pub timestamp_ms: u64,
    pub protocol_version: u16,
    pub correlation_id: Option<String>,
    pub session_id: Option<[u8; SESSION_ID_LEN]>,
    pub reason: Option<IpcAuditReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcAuditReason {
    UnsupportedProtocolVersion,
    MalformedFrame,
    OversizedFrame,
    Timeout,
    Closed,
    TransportFailure,
    AuthenticationRejected,
    SessionMismatch,
    UnexpectedMessageKind,
    CorrelationFailure,
    InvalidEnvelope,
}

pub trait IpcCodec: Send + Sync {
    fn encode(&self, envelope: &IpcEnvelope) -> Result<Vec<u8>, IpcError>;
    fn decode(&self, body: &[u8]) -> Result<IpcEnvelope, IpcError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonIpcCodec;

impl IpcCodec for JsonIpcCodec {
    fn encode(&self, envelope: &IpcEnvelope) -> Result<Vec<u8>, IpcError> {
        serde_json::to_vec(envelope).map_err(|_| IpcError::MalformedFrame("encode"))
    }

    fn decode(&self, body: &[u8]) -> Result<IpcEnvelope, IpcError> {
        serde_json::from_slice(body).map_err(|_| IpcError::MalformedFrame("decode"))
    }
}

pub fn encode_frame<C: IpcCodec>(codec: &C, envelope: &IpcEnvelope) -> Result<Vec<u8>, IpcError> {
    validate_envelope(envelope)?;
    let body = codec.encode(envelope)?;

    if body.is_empty() || body.len() > IPC_MAX_WIRE_BODY_SIZE {
        return Err(IpcError::OversizedFrame);
    }

    let body_len = u32::try_from(body.len()).map_err(|_| IpcError::OversizedFrame)?;
    let mut frame = Vec::with_capacity(FRAME_HEADER_LEN + body.len());
    frame.extend_from_slice(&body_len.to_be_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

pub fn decode_frame<C: IpcCodec>(codec: &C, frame: &[u8]) -> Result<IpcEnvelope, IpcError> {
    if frame.len() < FRAME_HEADER_LEN {
        return Err(IpcError::MalformedFrame("length_prefix"));
    }

    let declared = u32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
    if declared == 0 || declared > IPC_MAX_WIRE_BODY_SIZE {
        return Err(IpcError::OversizedFrame);
    }

    if frame.len() != FRAME_HEADER_LEN + declared {
        return Err(IpcError::MalformedFrame("length_mismatch"));
    }

    let envelope = codec.decode(&frame[FRAME_HEADER_LEN..])?;
    validate_envelope(&envelope)?;
    Ok(envelope)
}

fn validate_envelope(envelope: &IpcEnvelope) -> Result<(), IpcError> {
    match envelope {
        IpcEnvelope::Request(message) => {
            validate_protocol(message.protocol_version)?;
            validate_text(&message.correlation_id, IPC_MAX_CORRELATION_ID_LEN, "correlation_id")?;
            validate_text(&message.method, IPC_MAX_METHOD_LEN, "method")?;
            validate_payload(&message.payload)?;
            if message.sequence == 0 {
                return Err(IpcError::InvalidEnvelope("sequence"));
            }
            if message.proof.is_empty() {
                return Err(IpcError::InvalidEnvelope("proof"));
            }
        }
        IpcEnvelope::Response(message) => {
            validate_protocol(message.protocol_version)?;
            validate_text(&message.correlation_id, IPC_MAX_CORRELATION_ID_LEN, "correlation_id")?;
            validate_payload(&message.payload)?;
            if message.sequence == 0 {
                return Err(IpcError::InvalidEnvelope("sequence"));
            }
            if let Some(error_code) = &message.error_code {
                validate_text(error_code, IPC_MAX_ERROR_CODE_LEN, "error_code")?;
            }
            if let Some(proof) = &message.proof {
                if proof.len() != crate::auth::AUTH_PROOF_LEN {
                    return Err(IpcError::InvalidEnvelope("proof"));
                }
            }
        }
        IpcEnvelope::Event(message) => {
            validate_protocol(message.protocol_version)?;
            validate_text(&message.event_id, IPC_MAX_EVENT_ID_LEN, "event_id")?;
            validate_text(&message.event_type, IPC_MAX_EVENT_TYPE_LEN, "event_type")?;
            validate_payload(&message.payload)?;
            if message.sequence == 0 {
                return Err(IpcError::InvalidEnvelope("sequence"));
            }
            if let Some(proof) = &message.proof {
                if proof.len() != crate::auth::AUTH_PROOF_LEN {
                    return Err(IpcError::InvalidEnvelope("proof"));
                }
            }
        }
    }

    Ok(())
}

fn validate_protocol(version: u16) -> Result<(), IpcError> {
    if version != IPC_PROTOCOL_VERSION {
        return Err(IpcError::UnsupportedProtocolVersion {
            expected: IPC_PROTOCOL_VERSION,
            received: version,
        });
    }
    Ok(())
}

fn validate_text(value: &str, max_len: usize, _field: &'static str) -> Result<(), IpcError> {
    if value.is_empty() || value.len() > max_len || value.bytes().any(|byte| byte == 0) {
        return Err(IpcError::InvalidEnvelope("text"));
    }
    Ok(())
}

fn validate_payload(payload: &[u8]) -> Result<(), IpcError> {
    if payload.len() > IPC_MAX_PAYLOAD_SIZE {
        return Err(IpcError::OversizedFrame);
    }
    Ok(())
}

pub fn canonical_message_material(envelope: &IpcEnvelope) -> Result<Vec<u8>, IpcError> {
    validate_envelope(envelope)?;

    let mut output = Vec::with_capacity(IPC_MAX_PAYLOAD_SIZE.min(1024) + 256);
    output.extend_from_slice(IPC_MESSAGE_DOMAIN);
    output.push(match envelope.kind() {
        IpcMessageKind::Request => 1,
        IpcMessageKind::Response => 2,
        IpcMessageKind::Event => 3,
    });
    output.extend_from_slice(&envelope.protocol_version().to_be_bytes());
    push_bytes(&mut output, &envelope.session_id());

    match envelope {
        IpcEnvelope::Request(message) => {
            push_string(&mut output, &message.correlation_id);
            output.extend_from_slice(&message.sequence.to_be_bytes());
            push_string(&mut output, &message.method);
            push_bytes(&mut output, &message.payload);
        }
        IpcEnvelope::Response(message) => {
            push_string(&mut output, &message.correlation_id);
            output.extend_from_slice(&message.sequence.to_be_bytes());
            output.push(match message.status {
                ResponseStatus::Ok => 1,
                ResponseStatus::Error => 2,
                ResponseStatus::Rejected => 3,
            });
            push_optional_string(&mut output, message.error_code.as_deref());
            push_bytes(&mut output, &message.payload);
        }
        IpcEnvelope::Event(message) => {
            push_string(&mut output, &message.event_id);
            output.extend_from_slice(&message.sequence.to_be_bytes());
            push_string(&mut output, &message.event_type);
            push_bytes(&mut output, &message.payload);
        }
    }

    Ok(output)
}

fn push_bytes(output: &mut Vec<u8>, value: &[u8]) {
    let length = u32::try_from(value.len()).unwrap_or(u32::MAX);
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
}

fn push_string(output: &mut Vec<u8>, value: &str) {
    push_bytes(output, value.as_bytes());
}

fn push_optional_string(output: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            output.push(1);
            push_string(output, value);
        }
        None => output.push(0),
    }
}

pub trait IpcTransport: Send {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), IpcError>;
    fn recv_frame(&mut self, timeout: Duration) -> Result<Vec<u8>, IpcError>;
    fn shutdown(&mut self) -> Result<(), IpcError>;
}

pub struct IpcConnection<T: IpcTransport, C: IpcCodec = JsonIpcCodec> {
    transport: T,
    codec: C,
    open: bool,
    audit_events: VecDeque<IpcAuditEvent>,
}

impl<T: IpcTransport> IpcConnection<T, JsonIpcCodec> {
    pub fn new(transport: T) -> Self {
        Self::with_codec(transport, JsonIpcCodec)
    }
}

impl<T: IpcTransport, C: IpcCodec> IpcConnection<T, C> {
    pub fn with_codec(transport: T, codec: C) -> Self {
        Self {
            transport,
            codec,
            open: true,
            audit_events: VecDeque::with_capacity(IPC_MAX_AUDIT_EVENTS),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn send(&mut self, envelope: &IpcEnvelope) -> Result<(), IpcError> {
        if !self.open {
            return Err(IpcError::Closed);
        }

        let frame = encode_frame(&self.codec, envelope)?;
        self.transport.send_frame(&frame)
    }

    pub fn receive(&mut self, timeout: Duration) -> Result<IpcEnvelope, IpcError> {
        if !self.open {
            return Err(IpcError::Closed);
        }

        let frame = match self.transport.recv_frame(timeout) {
            Ok(frame) => frame,
            Err(error) => {
                self.record_transport_error(&error);
                if matches!(error, IpcError::Closed | IpcError::MalformedFrame(_) | IpcError::OversizedFrame) {
                    let _ = self.shutdown();
                }
                return Err(error);
            }
        };

        match decode_frame(&self.codec, &frame) {
            Ok(envelope) => Ok(envelope),
            Err(error) => {
                self.record_frame_error(&error);
                let _ = self.shutdown();
                Err(error)
            }
        }
    }

    pub fn take_audit_events(&mut self) -> Vec<IpcAuditEvent> {
        self.audit_events.drain(..).collect()
    }

    pub fn shutdown(&mut self) -> Result<(), IpcError> {
        if !self.open {
            return Ok(());
        }

        self.open = false;
        let result = self.transport.shutdown();
        self.record(IpcAuditEvent {
            event_type: IpcAuditEventType::ConnectionClosed,
            timestamp_ms: 0,
            protocol_version: IPC_PROTOCOL_VERSION,
            correlation_id: None,
            session_id: None,
            reason: None,
        });
        result
    }

    fn record_transport_error(&mut self, error: &IpcError) {
        let reason = match error {
            IpcError::Timeout => Some(IpcAuditReason::Timeout),
            IpcError::Closed => Some(IpcAuditReason::Closed),
            IpcError::Transport(_) => Some(IpcAuditReason::TransportFailure),
            _ => None,
        };
        if let Some(reason) = reason {
            self.record(IpcAuditEvent {
                event_type: IpcAuditEventType::FrameRejected,
                timestamp_ms: 0,
                protocol_version: IPC_PROTOCOL_VERSION,
                correlation_id: None,
                session_id: None,
                reason: Some(reason),
            });
        }
    }

    fn record_frame_error(&mut self, error: &IpcError) {
        let reason = match error {
            IpcError::UnsupportedProtocolVersion { .. } => IpcAuditReason::UnsupportedProtocolVersion,
            IpcError::MalformedFrame(_) => IpcAuditReason::MalformedFrame,
            IpcError::OversizedFrame => IpcAuditReason::OversizedFrame,
            IpcError::InvalidEnvelope(_) => IpcAuditReason::InvalidEnvelope,
            _ => IpcAuditReason::MalformedFrame,
        };
        self.record(IpcAuditEvent {
            event_type: IpcAuditEventType::FrameRejected,
            timestamp_ms: 0,
            protocol_version: IPC_PROTOCOL_VERSION,
            correlation_id: None,
            session_id: None,
            reason: Some(reason),
        });
    }

    fn record(&mut self, event: IpcAuditEvent) {
        if self.audit_events.len() == IPC_MAX_AUDIT_EVENTS {
            self.audit_events.pop_front();
        }
        self.audit_events.push_back(event);
    }
}

impl<T: IpcTransport, C: IpcCodec> Drop for IpcConnection<T, C> {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

pub struct AuthenticatedCoreChannel<T: IpcTransport, C: IpcCodec = JsonIpcCodec, A: AuthenticationProvider = crate::auth::AuthenticationServer<crate::auth::PlatformCrypto>> {
    connection: IpcConnection<T, C>,
    auth: A,
    session: AuthenticatedSession,
}

impl<T: IpcTransport, A: AuthenticationProvider> AuthenticatedCoreChannel<T, JsonIpcCodec, A> {
    pub fn new(transport: T, auth: A, session: AuthenticatedSession) -> Self {
        Self::with_codec(transport, JsonIpcCodec, auth, session)
    }
}

impl<T: IpcTransport, C: IpcCodec, A: AuthenticationProvider> AuthenticatedCoreChannel<T, C, A> {
    pub fn with_codec(
        transport: T,
        codec: C,
        auth: A,
        session: AuthenticatedSession,
    ) -> Self {
        Self {
            connection: IpcConnection::with_codec(transport, codec),
            auth,
            session,
        }
    }

    pub fn receive_request(&mut self, timeout: Duration, now_ms: u64) -> Result<RequestEnvelope, IpcError> {
        let envelope = self.connection.receive(timeout)?;

        let request = match envelope {
            IpcEnvelope::Request(request) => request,
            _ => {
                let _ = self.connection.shutdown();
                return Err(IpcError::UnexpectedMessageKind);
            }
        };

        if request.session_id != *self.session.session_id().as_bytes() {
            self.record_auth_rejection(&request, IpcError::SessionMismatch, now_ms);
            let _ = self.connection.shutdown();
            return Err(IpcError::SessionMismatch);
        }

        let envelope = IpcEnvelope::Request(request.clone());
        let material = canonical_message_material(&envelope)?;

        if let Err(error) = self.auth.verify_session_message(
            &self.session,
            request.protocol_version,
            &request.correlation_id,
            request.sequence,
            &material,
            &request.proof,
            now_ms,
        ) {
            let ipc_error = IpcError::Authentication(error);
            self.record_auth_rejection(&request, ipc_error.clone(), now_ms);
            let _ = self.connection.shutdown();
            return Err(ipc_error);
        }

        self.record(IpcAuditEvent {
            event_type: IpcAuditEventType::RequestAccepted,
            timestamp_ms: now_ms,
            protocol_version: request.protocol_version,
            correlation_id: Some(request.correlation_id.clone()),
            session_id: Some(request.session_id),
            reason: None,
        });

        Ok(request)
    }

    pub fn send_response(
        &mut self,
        request: &RequestEnvelope,
        status: ResponseStatus,
        error_code: Option<String>,
        payload: Vec<u8>,
    ) -> Result<(), IpcError> {
        if request.session_id != *self.session.session_id().as_bytes() {
            return Err(IpcError::SessionMismatch);
        }
        if payload.len() > IPC_MAX_PAYLOAD_SIZE {
            return Err(IpcError::OversizedFrame);
        }
        if let Some(code) = &error_code {
            validate_text(code, IPC_MAX_ERROR_CODE_LEN, "error_code")?;
        }

        self.connection.send(&IpcEnvelope::Response(ResponseEnvelope {
            protocol_version: IPC_PROTOCOL_VERSION,
            correlation_id: request.correlation_id.clone(),
            session_id: request.session_id,
            sequence: request.sequence,
            status,
            error_code,
            payload,
            proof: None,
        }))
    }

    pub fn send_event(
        &mut self,
        sequence: u64,
        event_id: String,
        event_type: String,
        payload: Vec<u8>,
    ) -> Result<(), IpcError> {
        if sequence == 0 {
            return Err(IpcError::InvalidEnvelope("sequence"));
        }

        self.connection.send(&IpcEnvelope::Event(EventEnvelope {
            protocol_version: IPC_PROTOCOL_VERSION,
            event_id,
            session_id: *self.session.session_id().as_bytes(),
            sequence,
            event_type,
            payload,
            proof: None,
        }))
    }

    pub fn shutdown(&mut self) -> Result<(), IpcError> {
        self.connection.shutdown()
    }

    pub fn take_audit_events(&mut self) -> Vec<IpcAuditEvent> {
        self.connection.take_audit_events()
    }

    fn record_auth_rejection(&mut self, request: &RequestEnvelope, error: IpcError, now_ms: u64) {
        let reason = match error {
            IpcError::SessionMismatch => IpcAuditReason::SessionMismatch,
            IpcError::Authentication(_) => IpcAuditReason::AuthenticationRejected,
            _ => IpcAuditReason::InvalidEnvelope,
        };
        self.record(IpcAuditEvent {
            event_type: IpcAuditEventType::FrameRejected,
            timestamp_ms: now_ms,
            protocol_version: request.protocol_version,
            correlation_id: Some(request.correlation_id.clone()),
            session_id: Some(request.session_id),
            reason: Some(reason),
        });
    }

    fn record(&mut self, event: IpcAuditEvent) {
        self.connection.record(event);
    }
}

pub struct PendingRequests {
    pending: HashMap<String, ( [u8; SESSION_ID_LEN], u64 )>,
}

impl PendingRequests {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    pub fn register(&mut self, request: &RequestEnvelope) -> Result<(), IpcError> {
        validate_envelope(&IpcEnvelope::Request(request.clone()))?;
        if self.pending.contains_key(&request.correlation_id) {
            return Err(IpcError::DuplicateCorrelation);
        }
        self.pending.insert(
            request.correlation_id.clone(),
            (request.session_id, request.sequence),
        );
        Ok(())
    }

    pub fn accept_response(&mut self, response: &ResponseEnvelope) -> Result<(), IpcError> {
        validate_envelope(&IpcEnvelope::Response(response.clone()))?;

        let Some((session_id, sequence)) = self.pending.remove(&response.correlation_id) else {
            return Err(IpcError::UnknownCorrelation);
        };

        if session_id != response.session_id {
            return Err(IpcError::SessionMismatch);
        }

        if sequence != response.sequence {
            return Err(IpcError::ResponseSequenceMismatch);
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }
}

impl Default for PendingRequests {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemoryTransport {
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
    closed: bool,
}

impl MemoryTransport {
    pub fn pair() -> (Self, Self) {
        let (left_tx, left_rx) = mpsc::channel();
        let (right_tx, right_rx) = mpsc::channel();

        (
            Self {
                tx: left_tx,
                rx: right_rx,
                closed: false,
            },
            Self {
                tx: right_tx,
                rx: left_rx,
                closed: false,
            },
        )
    }
}

impl IpcTransport for MemoryTransport {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), IpcError> {
        if self.closed {
            return Err(IpcError::Closed);
        }
        self.tx
            .send(frame.to_vec())
            .map_err(|_| IpcError::Closed)
    }

    fn recv_frame(&mut self, timeout: Duration) -> Result<Vec<u8>, IpcError> {
        if self.closed {
            return Err(IpcError::Closed);
        }

        match self.rx.recv_timeout(timeout) {
            Ok(frame) => Ok(frame),
            Err(RecvTimeoutError::Timeout) => Err(IpcError::Timeout),
            Err(RecvTimeoutError::Disconnected) => Err(IpcError::Closed),
        }
    }

    fn shutdown(&mut self) -> Result<(), IpcError> {
        self.closed = true;
        Ok(())
    }
}

#[cfg(windows)]
pub struct WindowsLocalPipeTransport {
    read: Option<std::fs::File>,
    write: Option<std::fs::File>,
}

#[cfg(windows)]
pub struct ChildPipeHandles {
    read_handle: Option<RawHandleValue>,
    write_handle: Option<RawHandleValue>,
}

#[cfg(windows)]
type RawHandleValue = usize;

#[cfg(windows)]
impl ChildPipeHandles {
    pub fn read_handle(&self) -> Option<RawHandleValue> {
        self.read_handle
    }

    pub fn write_handle(&self) -> Option<RawHandleValue> {
        self.write_handle
    }

    pub fn into_raw_handles(mut self) -> (RawHandleValue, RawHandleValue) {
        let read = self.read_handle.take().expect("child read handle");
        let write = self.write_handle.take().expect("child write handle");
        (read, write)
    }
}

#[cfg(windows)]
impl Drop for ChildPipeHandles {
    fn drop(&mut self) {
        if let Some(handle) = self.read_handle.take() {
            unsafe {
                CloseHandle(handle as Handle);
            }
        }
        if let Some(handle) = self.write_handle.take() {
            unsafe {
                CloseHandle(handle as Handle);
            }
        }
    }
}

#[cfg(windows)]
impl WindowsLocalPipeTransport {
    /// Creates a private duplex anonymous-pipe pair for the Rust parent and
    /// its future Python child. No TCP or named-pipe listener is opened.
    pub fn create_for_child() -> Result<(Self, ChildPipeHandles), IpcError> {
        unsafe {
            let mut security_attributes = SecurityAttributes {
                n_length: std::mem::size_of::<SecurityAttributes>() as u32,
                lp_security_descriptor: std::ptr::null_mut(),
                b_inherit_handle: 1,
            };

            let mut parent_read: Handle = std::ptr::null_mut();
            let mut child_write: Handle = std::ptr::null_mut();
            if CreatePipe(
                &mut parent_read,
                &mut child_write,
                &mut security_attributes,
                64 * 1024,
            ) == 0
            {
                return Err(last_os_error("CreatePipe parent-read"));
            }

            let mut child_read: Handle = std::ptr::null_mut();
            let mut parent_write: Handle = std::ptr::null_mut();
            if CreatePipe(
                &mut child_read,
                &mut parent_write,
                &mut security_attributes,
                64 * 1024,
            ) == 0
            {
                CloseHandle(parent_read);
                CloseHandle(child_write);
                return Err(last_os_error("CreatePipe child-read"));
            }

            if SetHandleInformation(parent_read, HANDLE_FLAG_INHERIT, 0) == 0
                || SetHandleInformation(parent_write, HANDLE_FLAG_INHERIT, 0) == 0
            {
                CloseHandle(parent_read);
                CloseHandle(child_write);
                CloseHandle(child_read);
                CloseHandle(parent_write);
                return Err(last_os_error("SetHandleInformation"));
            }

            let parent = Self {
                read: Some(std::fs::File::from_raw_handle(parent_read as RawHandle)),
                write: Some(std::fs::File::from_raw_handle(parent_write as RawHandle)),
            };

            let child = ChildPipeHandles {
                read_handle: Some(child_read as usize),
                write_handle: Some(child_write as usize),
            };

            Ok((parent, child))
        }
    }

    /// # Safety
    /// The caller must pass valid, owned Windows pipe handles that belong to
    /// the current process. Ownership is transferred to this transport.
    pub unsafe fn from_inherited_handles(
        read_handle: RawHandleValue,
        write_handle: RawHandleValue,
    ) -> Self {
        Self {
            read: Some(std::fs::File::from_raw_handle(read_handle as RawHandle)),
            write: Some(std::fs::File::from_raw_handle(write_handle as RawHandle)),
        }
    }

    fn read_exact_with_timeout(
        file: &mut std::fs::File,
        timeout: Duration,
    ) -> Result<Vec<u8>, IpcError> {
        let handle = file.as_raw_handle();
        let deadline = Instant::now() + timeout;
        wait_for_pipe_bytes(handle, FRAME_HEADER_LEN as u32, deadline)?;

        let mut header = [0_u8; FRAME_HEADER_LEN];
        file.read_exact(&mut header)
            .map_err(|error| map_pipe_error(error, "read header"))?;

        let body_len = u32::from_be_bytes(header) as usize;
        if body_len == 0 || body_len > IPC_MAX_WIRE_BODY_SIZE {
            return Err(IpcError::OversizedFrame);
        }

        wait_for_pipe_bytes(handle, body_len as u32, deadline)?;

        let mut body = vec![0_u8; body_len];
        file.read_exact(&mut body)
            .map_err(|error| map_pipe_error(error, "read body"))?;

        let mut frame = Vec::with_capacity(FRAME_HEADER_LEN + body_len);
        frame.extend_from_slice(&header);
        frame.extend_from_slice(&body);
        Ok(frame)
    }
}

#[cfg(windows)]
impl IpcTransport for WindowsLocalPipeTransport {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), IpcError> {
        if frame.len() < FRAME_HEADER_LEN || frame.len() > FRAME_HEADER_LEN + IPC_MAX_WIRE_BODY_SIZE {
            return Err(IpcError::OversizedFrame);
        }

        let Some(write) = self.write.as_mut() else {
            return Err(IpcError::Closed);
        };

        write
            .write_all(frame)
            .map_err(|error| map_pipe_error(error, "write"))?;
        write
            .flush()
            .map_err(|error| map_pipe_error(error, "flush"))?;
        Ok(())
    }

    fn recv_frame(&mut self, timeout: Duration) -> Result<Vec<u8>, IpcError> {
        let Some(read) = self.read.as_mut() else {
            return Err(IpcError::Closed);
        };
        Self::read_exact_with_timeout(read, timeout)
    }

    fn shutdown(&mut self) -> Result<(), IpcError> {
        self.read.take();
        self.write.take();
        Ok(())
    }
}

#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};

#[cfg(windows)]
type Handle = *mut std::ffi::c_void;

#[cfg(windows)]
#[repr(C)]
struct SecurityAttributes {
    n_length: u32,
    lp_security_descriptor: *mut std::ffi::c_void,
    b_inherit_handle: i32,
}

#[cfg(windows)]
const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn CreatePipe(
        read_pipe: *mut Handle,
        write_pipe: *mut Handle,
        pipe_attributes: *mut SecurityAttributes,
        size: u32,
    ) -> i32;
    fn SetHandleInformation(handle: Handle, mask: u32, flags: u32) -> i32;
    fn PeekNamedPipe(
        named_pipe: Handle,
        buffer: *mut u8,
        buffer_size: u32,
        bytes_read: *mut u32,
        total_bytes_available: *mut u32,
        bytes_left_this_message: *mut u32,
    ) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
}

#[cfg(windows)]
fn wait_for_pipe_bytes(handle: RawHandle, needed: u32, deadline: Instant) -> Result<(), IpcError> {
    loop {
        let mut available = 0_u32;
        let status = unsafe {
            PeekNamedPipe(
                handle as Handle,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        };

        if status == 0 {
            let error = io::Error::last_os_error();
            return Err(map_pipe_error(error, "peek"));
        }

        if available >= needed {
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(IpcError::Timeout);
        }

        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(windows)]
fn map_pipe_error(error: io::Error, context: &str) -> IpcError {
    match error.raw_os_error() {
        Some(109) | Some(232) | Some(233) => IpcError::Closed,
        _ => IpcError::Transport(format!("{context}: {error}")),
    }
}

#[cfg(windows)]
fn last_os_error(context: &str) -> IpcError {
    IpcError::Transport(format!("{context}: {}", io::Error::last_os_error()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::test_session;

    struct TestAuthenticator {
        expected_sequence: u64,
        result: Option<AuthError>,
    }

    impl AuthenticationProvider for TestAuthenticator {
        fn issue_challenge(
            &mut self,
            _protocol_version: u16,
            _correlation_id: &str,
            _now_ms: u64,
        ) -> Result<crate::auth::AuthChallenge, AuthError> {
            Err(AuthError::CryptoFailure)
        }

        fn complete_challenge(
            &mut self,
            _response: &crate::auth::ChallengeResponse,
            _now_ms: u64,
        ) -> Result<AuthenticatedSession, AuthError> {
            Err(AuthError::CryptoFailure)
        }

        fn verify_session_message(
            &mut self,
            _session: &AuthenticatedSession,
            _protocol_version: u16,
            _correlation_id: &str,
            sequence: u64,
            _message_digest: &[u8],
            _proof: &[u8],
            _now_ms: u64,
        ) -> Result<(), AuthError> {
            if let Some(error) = self.result.take() {
                return Err(error);
            }
            if sequence <= self.expected_sequence {
                return Err(AuthError::ReplayDetected);
            }
            self.expected_sequence = sequence;
            Ok(())
        }

        fn invalidate_session(&mut self, _session: &AuthenticatedSession, _now_ms: u64) -> bool {
            false
        }

        fn invalidate_all_sessions(&mut self, _now_ms: u64) {}
    }

    fn request(sequence: u64) -> IpcEnvelope {
        IpcEnvelope::Request(RequestEnvelope {
            protocol_version: IPC_PROTOCOL_VERSION,
            correlation_id: format!("corr-{sequence}"),
            session_id: *test_session(10_000_000).session_id().as_bytes(),
            sequence,
            method: "core.test".to_string(),
            payload: b"payload".to_vec(),
            proof: vec![0xA5; crate::auth::AUTH_PROOF_LEN],
        })
    }

    #[test]
    fn frame_round_trip_is_strictly_length_delimited() {
        let codec = JsonIpcCodec;
        let original = request(1);
        let frame = encode_frame(&codec, &original).expect("frame");
        let decoded = decode_frame(&codec, &frame).expect("decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn malformed_length_is_rejected() {
        let codec = JsonIpcCodec;
        assert_eq!(
            decode_frame(&codec, &[0, 0, 0, 10, 1, 2]),
            Err(IpcError::MalformedFrame("length_mismatch"))
        );
    }

    #[test]
    fn unsupported_protocol_version_is_rejected() {
        let envelope = IpcEnvelope::Request(RequestEnvelope {
            protocol_version: 99,
            correlation_id: "corr".to_string(),
            session_id: [1; SESSION_ID_LEN],
            sequence: 1,
            method: "core.test".to_string(),
            payload: vec![],
            proof: vec![0; crate::auth::AUTH_PROOF_LEN],
        });
        assert_eq!(
            encode_frame(&JsonIpcCodec, &envelope),
            Err(IpcError::UnsupportedProtocolVersion {
                expected: IPC_PROTOCOL_VERSION,
                received: 99,
            })
        );
    }

    #[test]
    fn empty_request_proof_is_rejected() {
        let envelope = IpcEnvelope::Request(RequestEnvelope {
            proof: vec![],
            ..match request(1) {
                IpcEnvelope::Request(message) => message,
                _ => unreachable!(),
            }
        });
        assert_eq!(
            encode_frame(&JsonIpcCodec, &envelope),
            Err(IpcError::InvalidEnvelope("proof"))
        );
    }

    #[test]
    fn authenticated_request_is_accepted_once() {
        let (left, right) = MemoryTransport::pair();
        let mut client = IpcConnection::new(left);
        let mut host = AuthenticatedCoreChannel::new(
            right,
            TestAuthenticator {
                expected_sequence: 0,
                result: None,
            },
            test_session(10_000_000),
        );
        client.send(&request(1)).expect("send");
        let accepted = host.receive_request(IPC_DEFAULT_TIMEOUT, 1_000).expect("accept");
        assert_eq!(accepted.sequence, 1);

        client.send(&request(1)).expect("send replay");
        assert_eq!(
            host.receive_request(IPC_DEFAULT_TIMEOUT, 1_001),
            Err(IpcError::Authentication(AuthError::ReplayDetected))
        );
        assert!(!host.take_audit_events().is_empty());
    }

    #[test]
    fn expired_session_rejection_closes_connection() {
        let (_left, right) = MemoryTransport::pair();
        let mut host = AuthenticatedCoreChannel::new(
            right,
            TestAuthenticator {
                expected_sequence: 0,
                result: Some(AuthError::SessionExpired),
            },
            test_session(10_000_000),
        );
        assert!(host.connection.is_open());

        let mut client = IpcConnection::new(MemoryTransport::pair().0);
        let _ = client.send(&request(1));
        let result = host.receive_request(Duration::from_millis(1), 1_000);
        assert!(matches!(result, Err(IpcError::Timeout) | Err(IpcError::Closed)));
    }

    #[test]
    fn correlation_tracker_requires_exact_response_match() {
        let session = test_session(10_000_000);
        let request = match request(7) {
            IpcEnvelope::Request(request) => RequestEnvelope {
                session_id: *session.session_id().as_bytes(),
                ..request
            },
            _ => unreachable!(),
        };
        let mut tracker = PendingRequests::new();
        tracker.register(&request).expect("register");

        let response = ResponseEnvelope {
            protocol_version: IPC_PROTOCOL_VERSION,
            correlation_id: request.correlation_id.clone(),
            session_id: request.session_id,
            sequence: request.sequence,
            status: ResponseStatus::Ok,
            error_code: None,
            payload: b"ok".to_vec(),
            proof: None,
        };

        tracker.accept_response(&response).expect("match");
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn correlation_tracker_rejects_wrong_sequence() {
        let request = match request(8) {
            IpcEnvelope::Request(request) => request,
            _ => unreachable!(),
        };
        let mut tracker = PendingRequests::new();
        tracker.register(&request).expect("register");

        let response = ResponseEnvelope {
            protocol_version: IPC_PROTOCOL_VERSION,
            correlation_id: request.correlation_id.clone(),
            session_id: request.session_id,
            sequence: request.sequence + 1,
            status: ResponseStatus::Ok,
            error_code: None,
            payload: vec![],
            proof: None,
        };

        assert_eq!(
            tracker.accept_response(&response),
            Err(IpcError::ResponseSequenceMismatch)
        );
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn memory_transport_timeout_is_explicit() {
        let (_left, mut right) = MemoryTransport::pair();
        assert_eq!(
            right.recv_frame(Duration::from_millis(1)),
            Err(IpcError::Timeout)
        );
    }

    #[test]
    fn shutdown_is_idempotent() {
        let (left, _right) = MemoryTransport::pair();
        let mut connection = IpcConnection::new(left);
        connection.shutdown().expect("shutdown");
        connection.shutdown().expect("second shutdown");
        assert!(!connection.is_open());
    }

    #[test]
    fn canonical_material_changes_with_message_kind() {
        let request = request(1);
        let response = match request.clone() {
            IpcEnvelope::Request(message) => IpcEnvelope::Response(ResponseEnvelope {
                protocol_version: IPC_PROTOCOL_VERSION,
                correlation_id: message.correlation_id,
                session_id: message.session_id,
                sequence: message.sequence,
                status: ResponseStatus::Ok,
                error_code: None,
                payload: message.payload,
                proof: None,
            }),
            _ => unreachable!(),
        };
        assert_ne!(
            canonical_message_material(&request).expect("request material"),
            canonical_message_material(&response).expect("response material")
        );
    }
}

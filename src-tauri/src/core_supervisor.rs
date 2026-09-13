use crate::auth::{
    AuthChallenge, AuthenticatedSession, AuthenticationProvider, AuthenticationServer,
    ChallengeResponse, AUTH_PROTOCOL_VERSION,
};
use crate::capabilities::{
    HostCapabilityRegistry, FILESYSTEM_READ_TEXT, SYSTEM_TELEMETRY_READ,
};
use crate::ipc::{
    canonical_message_material, decode_frame, encode_frame, EventEnvelope, IpcEnvelope,
    IpcError, IpcTransport, RequestEnvelope, ResponseEnvelope, ResponseStatus,
};
use crate::network_gateway::NetworkGateway;
use crate::security::{
    CapabilityPolicy, CapabilityRequest, Permission, RiskLevel, SecurityDecision, SecurityGateway,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
use crate::ipc::WindowsLocalPipeTransport;

const CORE_HEARTBEAT_METHOD: &str = "core.health";
const CORE_HEARTBEAT_PAYLOAD: &[u8] = b"heartbeat";
const CORE_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(8);
const CORE_RECONNECT_INITIAL: Duration = Duration::from_secs(1);
const CORE_RECONNECT_MAX: Duration = Duration::from_secs(30);
const CORE_COMMAND_QUEUE_SIZE: usize = 64;
const CORE_TEXT_MAX_LEN: usize = 8 * 1024;
const CORE_EVENT_NAME: &str = "host://core/event";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug)]
pub enum CoreSupervisorError {
    UnsupportedPlatform,
    ProcessLaunch,
    Bootstrap,
    Handshake,
    Authentication,
    Protocol,
    Transport,
    HeartbeatTimeout,
    ChildExited,
}

#[derive(Debug, Clone)]
pub struct CoreLaunchConfig {
    pub python_executable: PathBuf,
    pub script_path: PathBuf,
    pub memory_db_path: PathBuf,
    pub workspace_root: Option<PathBuf>,
    pub heartbeat_timeout: Duration,
}

#[derive(Debug)]
struct HostCommand {
    correlation_id: String,
    text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct HostCoreEvent {
    pub event_type: String,
    pub event_id: String,
    pub payload: Value,
}

#[derive(Debug, Deserialize)]
struct CapabilityRequestPayload {
    request_id: String,
    capability_id: String,
    input: Value,
}

impl CoreLaunchConfig {
    pub fn from_app(app: &AppHandle) -> Result<Self, CoreSupervisorError> {
        let python_executable = std::env::var_os("NAVEEN_PYTHON_EXECUTABLE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("python"));

        let script_path = if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../core/naveen_core/main.py")
        } else {
            app.path()
                .resolve("core/naveen_core/main.py", BaseDirectory::Resource)
                .map_err(|_| CoreSupervisorError::ProcessLaunch)?
        };

        if !script_path.is_file() {
            return Err(CoreSupervisorError::ProcessLaunch);
        }

        let memory_db_path = app
            .path()
            .app_local_data_dir()
            .map_err(|_| CoreSupervisorError::ProcessLaunch)?
            .join("memory.sqlite3");

        let workspace_root = std::env::var_os("NAVEEN_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .filter(|path| path.is_dir());

        Ok(Self {
            python_executable,
            script_path,
            memory_db_path,
            workspace_root,
            heartbeat_timeout: CORE_HEARTBEAT_TIMEOUT,
        })
    }
}

pub trait ManagedCoreProcess: Send {
    fn transport(&mut self) -> &mut dyn IpcTransport;
    fn is_running(&mut self) -> Result<bool, CoreSupervisorError>;
    fn shutdown(&mut self);
}

pub trait CoreProcessLauncher: Send + Sync {
    fn launch(
        &self,
        config: &CoreLaunchConfig,
    ) -> Result<Box<dyn ManagedCoreProcess>, CoreSupervisorError>;
}

#[derive(Clone, Copy, Default)]
pub struct PlatformCoreProcessLauncher;

impl CoreProcessLauncher for PlatformCoreProcessLauncher {
    fn launch(
        &self,
        config: &CoreLaunchConfig,
    ) -> Result<Box<dyn ManagedCoreProcess>, CoreSupervisorError> {
        #[cfg(windows)]
        {
            Ok(Box::new(WindowsCoreProcess::launch(config)?))
        }
        #[cfg(not(windows))]
        {
            let _ = config;
            Err(CoreSupervisorError::UnsupportedPlatform)
        }
    }
}

pub struct CoreSupervisor {
    stop: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
    command_tx: Option<SyncSender<HostCommand>>,
    connected: Arc<AtomicBool>,
    next_command_id: AtomicU64,
}

impl CoreSupervisor {
    pub fn disabled() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(true)),
            join: Mutex::new(None),
            command_tx: None,
            connected: Arc::new(AtomicBool::new(false)),
            next_command_id: AtomicU64::new(1),
        }
    }

    pub fn start(config: CoreLaunchConfig, app_handle: AppHandle) -> Self {
        Self::start_with_launcher(config, Arc::new(PlatformCoreProcessLauncher), app_handle)
    }

    pub fn start_with_launcher(
        config: CoreLaunchConfig,
        launcher: Arc<dyn CoreProcessLauncher>,
        app_handle: AppHandle,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::sync_channel(CORE_COMMAND_QUEUE_SIZE);
        let stop = Arc::new(AtomicBool::new(false));
        let connected = Arc::new(AtomicBool::new(false));
        let stop_for_thread = Arc::clone(&stop);
        let connected_for_thread = Arc::clone(&connected);
        let join = std::thread::spawn(move || {
            supervisor_loop(
                config,
                launcher,
                stop_for_thread,
                command_rx,
                app_handle,
                connected_for_thread,
            )
        });

        Self {
            stop,
            join: Mutex::new(Some(join)),
            command_tx: Some(command_tx),
            connected,
            next_command_id: AtomicU64::new(1),
        }
    }

    pub fn submit_text(&self, text: String) -> Result<String, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("Command cannot be empty".to_string());
        }
        if trimmed.len() > CORE_TEXT_MAX_LEN {
            return Err("Command is too long".to_string());
        }
        if self.stop.load(Ordering::Acquire) {
            return Err("NAVEEN Core is stopped".to_string());
        }
        if !self.connected.load(Ordering::Acquire) {
            return Err("NAVEEN Core is not connected".to_string());
        }

        let Some(command_tx) = &self.command_tx else {
            return Err("NAVEEN Core is unavailable".to_string());
        };

        let id = self.next_command_id.fetch_add(1, Ordering::Relaxed);
        let correlation_id = format!("ui-text-{id}");
        command_tx
            .try_send(HostCommand {
                correlation_id: correlation_id.clone(),
                text,
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => "NAVEEN Core command queue is full".to_string(),
                mpsc::TrySendError::Disconnected(_) => "NAVEEN Core stopped".to_string(),
            })?;

        Ok(correlation_id)
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Release);
    }

    pub fn shutdown_and_join(&self) {
        self.shutdown();
        if let Ok(mut guard) = self.join.lock() {
            if let Some(join) = guard.take() {
                let _ = join.join();
            }
        }
    }
}

impl Drop for CoreSupervisor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

fn supervisor_loop(
    config: CoreLaunchConfig,
    launcher: Arc<dyn CoreProcessLauncher>,
    stop: Arc<AtomicBool>,
    command_rx: Receiver<HostCommand>,
    app_handle: AppHandle,
    connected: Arc<AtomicBool>,
) {
    let mut auth = match AuthenticationServer::new() {
        Ok(server) => server,
        Err(_) => {
            log::error!("NAVEEN Core authentication initialization failed");
            return;
        }
    };

    let security = SecurityGateway::new(vec![
        CapabilityPolicy::new(
            SYSTEM_TELEMETRY_READ,
            vec![Permission::SystemTelemetryRead],
            RiskLevel::Low,
        ),
        CapabilityPolicy::new(
            FILESYSTEM_READ_TEXT,
            vec![Permission::FilesystemRead],
            RiskLevel::Medium,
        ),
        CapabilityPolicy::new(
            MODEL_COMPLETE,
            vec![Permission::NetworkAccess],
            RiskLevel::Medium,
        ),
        CapabilityPolicy::new(
            NETWORK_FETCH_TEXT,
            vec![Permission::NetworkAccess],
            RiskLevel::High,
        ),
    ]);
    let capabilities = HostCapabilityRegistry::new(
        config.workspace_root.clone(),
        NetworkGateway::from_env(),
    );
    let mut backoff = CORE_RECONNECT_INITIAL;

    while !stop.load(Ordering::Acquire) {
        let _ = emit_core_event(&app_handle, "core.status", "host-status-connecting", json!({
            "state": "connecting"
        }));

        match run_connection(
            &config,
            launcher.as_ref(),
            &stop,
            &mut auth,
            &command_rx,
            &app_handle,
            &security,
            &capabilities,
            &connected,
        ) {
            Ok(()) => backoff = CORE_RECONNECT_INITIAL,
            Err(error) => {
                connected.store(false, Ordering::Release);
                auth.invalidate_all_sessions(now_ms());
                let _ = emit_core_event(
                    &app_handle,
                    "core.status",
                    "host-status-disconnected",
                    json!({ "state": "disconnected" }),
                );
                log_core_failure(&error);
            }
        }

        if stop.load(Ordering::Acquire) {
            break;
        }

        sleep_interruptible(&stop, backoff);
        backoff = std::cmp::min(backoff.saturating_mul(2), CORE_RECONNECT_MAX);
    }

    connected.store(false, Ordering::Release);
    auth.invalidate_all_sessions(now_ms());
    let _ = emit_core_event(
        &app_handle,
        "core.status",
        "host-status-stopped",
        json!({ "state": "stopped" }),
    );
    log::info!("NAVEEN Core supervisor stopped");
}

fn run_connection(
    config: &CoreLaunchConfig,
    launcher: &dyn CoreProcessLauncher,
    stop: &Arc<AtomicBool>,
    auth: &mut AuthenticationServer,
    command_rx: &Receiver<HostCommand>,
    app_handle: &AppHandle,
    security: &SecurityGateway,
    capabilities: &HostCapabilityRegistry,
    connected: &Arc<AtomicBool>,
) -> Result<(), CoreSupervisorError> {
    let mut process = launcher.launch(config)?;
    let result = establish_and_run(
        config,
        &mut *process,
        stop,
        auth,
        command_rx,
        app_handle,
        security,
        capabilities,
        connected,
    );
    process.shutdown();
    result
}

fn establish_and_run(
    config: &CoreLaunchConfig,
    process: &mut dyn ManagedCoreProcess,
    stop: &Arc<AtomicBool>,
    auth: &mut AuthenticationServer,
    command_rx: &Receiver<HostCommand>,
    app_handle: &AppHandle,
    security: &SecurityGateway,
    capabilities: &HostCapabilityRegistry,
    connected: &Arc<AtomicBool>,
) -> Result<(), CoreSupervisorError> {
    let correlation_id = format!("core-auth-{}", now_ms());

    send_control(
        process.transport(),
        &CoreControlMessage::AuthBootstrap {
            ipc_protocol_version: crate::ipc::IPC_PROTOCOL_VERSION,
            launch_id: auth.launch_id().as_bytes().to_vec(),
            launch_secret: auth.bootstrap_secret().to_vec(),
        },
    )?;

    let challenge = auth
        .issue_challenge(AUTH_PROTOCOL_VERSION, &correlation_id, now_ms())
        .map_err(|_| CoreSupervisorError::Authentication)?;
    send_control(process.transport(), &challenge_to_wire(&challenge))?;

    let response: ChallengeResponse = match recv_control(
        process.transport(),
        Duration::from_secs(5),
    )? {
        CoreControlMessage::AuthResponse {
            ipc_protocol_version,
            auth_protocol_version,
            launch_id,
            challenge_id,
            client_id,
            correlation_id,
            proof,
        } => {
            if ipc_protocol_version != crate::ipc::IPC_PROTOCOL_VERSION
                || auth_protocol_version != AUTH_PROTOCOL_VERSION
            {
                return Err(CoreSupervisorError::Handshake);
            }

            ChallengeResponse {
                protocol_version: auth_protocol_version,
                launch_id: crate::auth::LaunchId::from_bytes(array_16(&launch_id)?),
                challenge_id: crate::auth::ChallengeId::from_bytes(array_16(&challenge_id)?),
                client_id,
                correlation_id,
                proof: array_32(&proof)?,
            }
        }
        _ => return Err(CoreSupervisorError::Handshake),
    };

    let session = auth
        .complete_challenge(&response, now_ms())
        .map_err(|_| CoreSupervisorError::Authentication)?;

    send_control(
        process.transport(),
        &CoreControlMessage::AuthSession {
            ipc_protocol_version: crate::ipc::IPC_PROTOCOL_VERSION,
            auth_protocol_version: AUTH_PROTOCOL_VERSION,
            session_id: session.session_id().as_bytes().to_vec(),
            expires_at_ms: session.expires_at_ms(),
            correlation_id,
        },
    )?;

    connected.store(true, Ordering::Release);
    let _ = emit_core_event(
        app_handle,
        "core.status",
        "host-status-authenticated",
        json!({ "state": "authenticated" }),
    );

    log::info!("NAVEEN Core authenticated");
    heartbeat_loop(
        config,
        process,
        stop,
        auth,
        &session,
        command_rx,
        app_handle,
        security,
        capabilities,
    )
}

fn heartbeat_loop(
    config: &CoreLaunchConfig,
    process: &mut dyn ManagedCoreProcess,
    stop: &Arc<AtomicBool>,
    auth: &mut AuthenticationServer,
    session: &AuthenticatedSession,
    command_rx: &Receiver<HostCommand>,
    app_handle: &AppHandle,
    security: &SecurityGateway,
    capabilities: &HostCapabilityRegistry,
) -> Result<(), CoreSupervisorError> {
    let mut last_core_event_sequence = 0_u64;
    let mut outbound_sequence = 0_u64;
    let mut command_in_flight = false;

    while !stop.load(Ordering::Acquire) {
        if !command_in_flight {
            match command_rx.try_recv() {
                Ok(command) => {
                    outbound_sequence = outbound_sequence.saturating_add(1);
                    send_host_event(
                        process.transport(),
                        session,
                        outbound_sequence,
                        &format!("input-{}", outbound_sequence),
                        "core.input.text",
                        json!({
                            "correlation_id": command.correlation_id,
                            "text": command.text,
                        }),
                    )?;
                    command_in_flight = true;
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => {}
            }
        }

        if !process.is_running()? {
            return Err(CoreSupervisorError::ChildExited);
        }

        match process
            .transport()
            .recv_frame(config.heartbeat_timeout.min(Duration::from_millis(250)))
        {
            Ok(frame) => {
                let envelope = decode_frame(&crate::ipc::JsonIpcCodec, &frame)
                    .map_err(|_| CoreSupervisorError::Protocol)?;

                match envelope {
                    IpcEnvelope::Request(request) => {
                        handle_core_request(process.transport(), auth, session, request)?;
                    }
                    IpcEnvelope::Event(event) => {
                        if event.session_id != *session.session_id().as_bytes()
                            || event.sequence <= last_core_event_sequence
                        {
                            return Err(CoreSupervisorError::Authentication);
                        }
                        last_core_event_sequence = event.sequence;

                        let completed = matches!(
                            event.event_type.as_str(),
                            "core.response" | "core.error"
                        );
                        handle_core_event(
                            process.transport(),
                            session,
                            &event,
                            &mut outbound_sequence,
                            app_handle,
                            security,
                            capabilities,
                        )?;
                        if completed {
                            command_in_flight = false;
                        }
                    }
                    IpcEnvelope::Response(_) => return Err(CoreSupervisorError::Protocol),
                }
            }
            Err(IpcError::Timeout) => {}
            Err(_) => return Err(CoreSupervisorError::Transport),
        }
    }

    Ok(())
}

fn handle_core_event(
    transport: &mut dyn IpcTransport,
    session: &AuthenticatedSession,
    event: &EventEnvelope,
    outbound_sequence: &mut u64,
    app_handle: &AppHandle,
    security: &SecurityGateway,
    capabilities: &HostCapabilityRegistry,
) -> Result<(), CoreSupervisorError> {
    match event.event_type.as_str() {
        "core.status" | "core.response" | "core.error" => {
            let payload: Value = serde_json::from_slice(&event.payload)
                .map_err(|_| CoreSupervisorError::Protocol)?;
            if !payload.is_object() {
                return Err(CoreSupervisorError::Protocol);
            }
            emit_core_event(
                app_handle,
                &event.event_type,
                &event.event_id,
                payload,
            )
            .map_err(|_| CoreSupervisorError::Transport)
        }
        "capability.request" => {
            let request: CapabilityRequestPayload = serde_json::from_slice(&event.payload)
                .map_err(|_| CoreSupervisorError::Protocol)?;

            if request.request_id.is_empty()
                || request.request_id.len() > crate::ipc::IPC_MAX_EVENT_ID_LEN
                || request.capability_id.is_empty()
                || request.capability_id.len() > crate::ipc::IPC_MAX_EVENT_TYPE_LEN
            {
                return Err(CoreSupervisorError::Protocol);
            }

            *outbound_sequence = outbound_sequence.saturating_add(1);
            let result = execute_capability(security, capabilities, session, &request);

            send_host_event(
                transport,
                session,
                *outbound_sequence,
                &format!("capability-result-{}", request.request_id),
                "capability.result",
                match result {
                    Ok(output) => json!({
                        "request_id": request.request_id,
                        "capability_id": request.capability_id,
                        "ok": true,
                        "output": output,
                    }),
                    Err(error) => json!({
                        "request_id": request.request_id,
                        "capability_id": request.capability_id,
                        "ok": false,
                        "output": {},
                        "error": error,
                    }),
                },
            )
        }
        _ => Err(CoreSupervisorError::Protocol),
    }
}

fn execute_capability(
    security: &SecurityGateway,
    capabilities: &HostCapabilityRegistry,
    session: &AuthenticatedSession,
    request: &CapabilityRequestPayload,
) -> Result<Value, String> {
    let capability_request = CapabilityRequest {
        capability_id: request.capability_id.clone(),
    };

    match security.authorize_at(now_ms(), Some(session), &capability_request, false) {
        SecurityDecision::Allow => {
            log::info!("NAVEEN capability allowed: {}", request.capability_id);
            capabilities.execute(&request.capability_id, request.input.clone())
        }
        SecurityDecision::RequireConfirmation => {
            log::warn!(
                "NAVEEN capability requires confirmation: {}",
                request.capability_id
            );
            Err("confirmation_required".to_string())
        }
        SecurityDecision::Deny(_) => {
            log::warn!("NAVEEN capability denied: {}", request.capability_id);
            Err("capability_denied".to_string())
        }
    }
}

fn emit_core_event(
    app_handle: &AppHandle,
    event_type: &str,
    event_id: &str,
    payload: Value,
) -> Result<(), tauri::Error> {
    app_handle.emit(
        CORE_EVENT_NAME,
        HostCoreEvent {
            event_type: event_type.to_string(),
            event_id: event_id.to_string(),
            payload,
        },
    )
}

fn send_host_event(
    transport: &mut dyn IpcTransport,
    session: &AuthenticatedSession,
    sequence: u64,
    event_id: &str,
    event_type: &str,
    payload: Value,
) -> Result<(), CoreSupervisorError> {
    let payload = serde_json::to_vec(&payload).map_err(|_| CoreSupervisorError::Protocol)?;
    let envelope = IpcEnvelope::Event(EventEnvelope {
        protocol_version: crate::ipc::IPC_PROTOCOL_VERSION,
        event_id: event_id.to_string(),
        session_id: *session.session_id().as_bytes(),
        sequence,
        event_type: event_type.to_string(),
        payload,
        proof: None,
    });
    let frame = encode_frame(&crate::ipc::JsonIpcCodec, &envelope)
        .map_err(|_| CoreSupervisorError::Protocol)?;
    transport
        .send_frame(&frame)
        .map_err(|_| CoreSupervisorError::Transport)
}

fn handle_core_request(
    transport: &mut dyn IpcTransport,
    auth: &mut AuthenticationServer,
    session: &AuthenticatedSession,
    request: RequestEnvelope,
) -> Result<(), CoreSupervisorError> {
    if request.session_id != *session.session_id().as_bytes() {
        return Err(CoreSupervisorError::Authentication);
    }

    let material = canonical_message_material(&IpcEnvelope::Request(request.clone()))
        .map_err(|_| CoreSupervisorError::Protocol)?;
    auth.verify_session_message(
        session,
        request.protocol_version,
        &request.correlation_id,
        request.sequence,
        &material,
        &request.proof,
        now_ms(),
    )
    .map_err(|_| CoreSupervisorError::Authentication)?;

    let (status, error_code, payload) =
        if request.method == CORE_HEARTBEAT_METHOD && request.payload == CORE_HEARTBEAT_PAYLOAD {
            (ResponseStatus::Ok, None, b"alive".to_vec())
        } else {
            (
                ResponseStatus::Rejected,
                Some("method_not_allowed".to_string()),
                Vec::new(),
            )
        };

    let response = ResponseEnvelope {
        protocol_version: crate::ipc::IPC_PROTOCOL_VERSION,
        correlation_id: request.correlation_id,
        session_id: request.session_id,
        sequence: request.sequence,
        status,
        error_code,
        payload,
        proof: None,
    };

    let frame = encode_frame(
        &crate::ipc::JsonIpcCodec,
        &IpcEnvelope::Response(response),
    )
    .map_err(|_| CoreSupervisorError::Protocol)?;
    transport
        .send_frame(&frame)
        .map_err(|_| CoreSupervisorError::Transport)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CoreControlMessage {
    AuthBootstrap {
        ipc_protocol_version: u16,
        launch_id: Vec<u8>,
        launch_secret: Vec<u8>,
    },
    AuthChallenge {
        ipc_protocol_version: u16,
        auth_protocol_version: u16,
        launch_id: Vec<u8>,
        challenge_id: Vec<u8>,
        nonce: Vec<u8>,
        issued_at_ms: u64,
        expires_at_ms: u64,
        expected_client_id: String,
        correlation_id: String,
    },
    AuthResponse {
        ipc_protocol_version: u16,
        auth_protocol_version: u16,
        launch_id: Vec<u8>,
        challenge_id: Vec<u8>,
        client_id: String,
        correlation_id: String,
        proof: Vec<u8>,
    },
    AuthSession {
        ipc_protocol_version: u16,
        auth_protocol_version: u16,
        session_id: Vec<u8>,
        expires_at_ms: u64,
        correlation_id: String,
    },
}

fn challenge_to_wire(challenge: &AuthChallenge) -> CoreControlMessage {
    CoreControlMessage::AuthChallenge {
        ipc_protocol_version: crate::ipc::IPC_PROTOCOL_VERSION,
        auth_protocol_version: challenge.protocol_version,
        launch_id: challenge.launch_id.as_bytes().to_vec(),
        challenge_id: challenge.challenge_id.as_bytes().to_vec(),
        nonce: challenge.nonce.to_vec(),
        issued_at_ms: challenge.issued_at_ms,
        expires_at_ms: challenge.expires_at_ms,
        expected_client_id: challenge.expected_client_id.clone(),
        correlation_id: challenge.correlation_id.clone(),
    }
}

fn send_control(
    transport: &mut dyn IpcTransport,
    message: &impl Serialize,
) -> Result<(), CoreSupervisorError> {
    let body = serde_json::to_vec(message).map_err(|_| CoreSupervisorError::Bootstrap)?;
    if body.is_empty() || body.len() > crate::ipc::IPC_MAX_WIRE_BODY_SIZE {
        return Err(CoreSupervisorError::Protocol);
    }

    let length = u32::try_from(body.len()).map_err(|_| CoreSupervisorError::Protocol)?;
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&body);
    transport
        .send_frame(&frame)
        .map_err(|_| CoreSupervisorError::Transport)
}

fn recv_control<T: DeserializeOwned>(
    transport: &mut dyn IpcTransport,
    timeout: Duration,
) -> Result<T, CoreSupervisorError> {
    let frame = transport
        .recv_frame(timeout)
        .map_err(|_| CoreSupervisorError::Handshake)?;
    if frame.len() < 4 {
        return Err(CoreSupervisorError::Handshake);
    }

    let declared = u32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
    if declared == 0
        || declared > crate::ipc::IPC_MAX_WIRE_BODY_SIZE
        || frame.len() != declared + 4
    {
        return Err(CoreSupervisorError::Protocol);
    }

    serde_json::from_slice(&frame[4..]).map_err(|_| CoreSupervisorError::Handshake)
}

fn array_16(value: &[u8]) -> Result<[u8; 16], CoreSupervisorError> {
    value
        .try_into()
        .map_err(|_| CoreSupervisorError::Handshake)
}

fn array_32(value: &[u8]) -> Result<[u8; 32], CoreSupervisorError> {
    value
        .try_into()
        .map_err(|_| CoreSupervisorError::Handshake)
}

fn sleep_interruptible(stop: &AtomicBool, duration: Duration) {
    let start = std::time::Instant::now();
    while !stop.load(Ordering::Acquire) && start.elapsed() < duration {
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn log_core_failure(error: &CoreSupervisorError) {
    match error {
        CoreSupervisorError::UnsupportedPlatform => {
            log::warn!("NAVEEN Core unavailable: unsupported platform")
        }
        CoreSupervisorError::ProcessLaunch => {
            log::warn!("NAVEEN Core unavailable: process launch failure")
        }
        CoreSupervisorError::Bootstrap => {
            log::warn!("NAVEEN Core unavailable: bootstrap failure")
        }
        CoreSupervisorError::Handshake => {
            log::warn!("NAVEEN Core unavailable: handshake failure")
        }
        CoreSupervisorError::Authentication => {
            log::warn!("NAVEEN Core disconnected: authentication failure")
        }
        CoreSupervisorError::Protocol => {
            log::warn!("NAVEEN Core disconnected: protocol failure")
        }
        CoreSupervisorError::Transport => {
            log::warn!("NAVEEN Core disconnected: transport failure")
        }
        CoreSupervisorError::HeartbeatTimeout => {
            log::warn!("NAVEEN Core disconnected: heartbeat timeout")
        }
        CoreSupervisorError::ChildExited => {
            log::warn!("NAVEEN Core disconnected: child exited")
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(windows)]
struct WindowsCoreProcess {
    child: Child,
    transport: Option<WindowsLocalPipeTransport>,
}

#[cfg(windows)]
impl WindowsCoreProcess {
    fn launch(config: &CoreLaunchConfig) -> Result<Self, CoreSupervisorError> {
        use std::os::windows::io::{FromRawHandle, RawHandle};
        use std::os::windows::process::CommandExt;

        let (transport, child_handles) = WindowsLocalPipeTransport::create_for_child()
            .map_err(|_| CoreSupervisorError::Transport)?;
        let (child_read, child_write) = child_handles.into_raw_handles();
        let stdin = unsafe { std::fs::File::from_raw_handle(child_read as RawHandle) };
        let stdout = unsafe { std::fs::File::from_raw_handle(child_write as RawHandle) };
        let working_dir = config
            .script_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or(CoreSupervisorError::ProcessLaunch)?;

        if let Some(parent) = config.memory_db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| CoreSupervisorError::ProcessLaunch)?;
        }

        let mut command = Command::new(&config.python_executable);
        command
            .arg("-E")
            .arg("-s")
            .arg(&config.script_path)
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::null())
            .env_clear()
            .env("NAVEEN_MEMORY_DB", &config.memory_db_path)
            .current_dir(working_dir)
            .creation_flags(CREATE_NO_WINDOW);

        if let Some(workspace_root) = &config.workspace_root {
            command.env("NAVEEN_WORKSPACE_ROOT", workspace_root);
        }

        for variable in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(variable) {
                command.env(variable, value);
            }
        }

        let child = command
            .spawn()
            .map_err(|_| CoreSupervisorError::ProcessLaunch)?;
        Ok(Self {
            child,
            transport: Some(transport),
        })
    }
}

#[cfg(windows)]
impl ManagedCoreProcess for WindowsCoreProcess {
    fn transport(&mut self) -> &mut dyn IpcTransport {
        self.transport
            .as_mut()
            .expect("core transport must remain available")
    }

    fn is_running(&mut self) -> Result<bool, CoreSupervisorError> {
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|_| CoreSupervisorError::ProcessLaunch)
    }

    fn shutdown(&mut self) {
        self.transport.take();
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

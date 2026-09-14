use crate::auth::{
    AuthChallenge, AuthenticatedSession, AuthenticationProvider, AuthenticationServer,
    ChallengeResponse, AUTH_PROTOCOL_VERSION,
};
use crate::capabilities::{HostCapabilityRegistry, FILESYSTEM_READ_TEXT, SYSTEM_TELEMETRY_READ};
use crate::ipc::{
    canonical_message_material, decode_frame, encode_frame, EventEnvelope, IpcEnvelope, IpcError,
    IpcTransport, RequestEnvelope, ResponseEnvelope, ResponseStatus,
};
use crate::network_gateway::{NetworkGateway, MODEL_COMPLETE, NETWORK_FETCH_TEXT};
use crate::security::{
    CapabilityPolicy, CapabilityRequest, Permission, RiskLevel, SecurityDecision, SecurityGateway,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
use crate::ipc::WindowsLocalPipeTransport;

const CORE_HEARTBEAT_METHOD: &str = "core.health";
const CORE_HEARTBEAT_PAYLOAD: &[u8] = b"heartbeat";
const CORE_RECONNECT_INITIAL: Duration = Duration::from_secs(1);
const CORE_RECONNECT_MAX: Duration = Duration::from_secs(30);
const CORE_COMMAND_QUEUE_SIZE: usize = 64;
const CORE_TEXT_MAX_LEN: usize = 8 * 1024;
const CORE_EVENT_NAME: &str = "host://core/event";
const CORE_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(8);
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
                app_handle,
                command_rx,
                stop_for_thread,
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
        let text = text.trim().to_string();
        if text.is_empty() || text.len() > CORE_TEXT_MAX_LEN {
            return Err("command is empty or too large".to_string());
        }
        if !self.connected.load(Ordering::Acquire) {
            return Err("NAVEEN Core is not connected".to_string());
        }

        let correlation_id = format!(
            "ui-{}",
            self.next_command_id.fetch_add(1, Ordering::Relaxed)
        );
        self.command_tx
            .as_ref()
            .ok_or_else(|| "NAVEEN Core is disabled".to_string())?
            .try_send(HostCommand {
                correlation_id: correlation_id.clone(),
                text,
            })
            .map_err(|_| "NAVEEN Core command queue is full".to_string())?;
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
    app_handle: AppHandle,
    command_rx: Receiver<HostCommand>,
    stop: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
) {
    let mut auth = match AuthenticationServer::new() {
        Ok(server) => server,
        Err(_) => {
            log::error!("NAVEEN Core authentication initialization failed");
            return;
        }
    };

    let mut backoff = CORE_RECONNECT_INITIAL;

    while !stop.load(Ordering::Acquire) {
        match run_connection(
            &config,
            launcher.as_ref(),
            &app_handle,
            &command_rx,
            &stop,
            &mut auth,
            &connected,
        ) {
            Ok(()) => backoff = CORE_RECONNECT_INITIAL,
            Err(error) => {
                connected.store(false, Ordering::Release);
                auth.invalidate_all_sessions(now_ms());
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
    log::info!("NAVEEN Core supervisor stopped");
}

fn run_connection(
    config: &CoreLaunchConfig,
    launcher: &dyn CoreProcessLauncher,
    app_handle: &AppHandle,
    command_rx: &Receiver<HostCommand>,
    stop: &Arc<AtomicBool>,
    auth: &mut AuthenticationServer,
    connected: &Arc<AtomicBool>,
) -> Result<(), CoreSupervisorError> {
    let mut process = launcher.launch(config)?;
    let result = establish_and_run(
        config,
        &mut *process,
        app_handle,
        command_rx,
        stop,
        auth,
        connected,
    );
    process.shutdown();
    result
}

fn establish_and_run(
    config: &CoreLaunchConfig,
    process: &mut dyn ManagedCoreProcess,
    app_handle: &AppHandle,
    command_rx: &Receiver<HostCommand>,
    stop: &Arc<AtomicBool>,
    auth: &mut AuthenticationServer,
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

    let response: ChallengeResponse =
        match recv_control(process.transport(), Duration::from_secs(5))? {
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
    log::info!("NAVEEN Core authenticated");
    heartbeat_loop(
        config, process, app_handle, command_rx, stop, auth, &session, connected,
    )
}

fn heartbeat_loop(
    config: &CoreLaunchConfig,
    process: &mut dyn ManagedCoreProcess,
    app_handle: &AppHandle,
    command_rx: &Receiver<HostCommand>,
    stop: &Arc<AtomicBool>,
    auth: &mut AuthenticationServer,
    session: &AuthenticatedSession,
    connected: &Arc<AtomicBool>,
) -> Result<(), CoreSupervisorError> {
    let registry =
        HostCapabilityRegistry::new(config.workspace_root.clone(), NetworkGateway::default());
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
            RiskLevel::Medium,
        ),
    ]);
    let mut sequence = 1_u64;
    let mut incoming_event_sequence = 0_u64;
    let mut missed_heartbeats = 0_u32;

    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if !process.is_running()? {
            connected.store(false, Ordering::Release);
            return Err(CoreSupervisorError::ChildExited);
        }

        while let Ok(command) = command_rx.try_recv() {
            send_host_command(process.transport(), session, sequence, &command)?;
            sequence = sequence.saturating_add(1);
        }

        match process
            .transport()
            .recv_frame(config.heartbeat_timeout.min(Duration::from_secs(2)))
        {
            Ok(frame) => {
                missed_heartbeats = 0;
                let envelope = decode_frame(&crate::ipc::JsonIpcCodec, &frame)
                    .map_err(|_| CoreSupervisorError::Protocol)?;
                match envelope {
                    IpcEnvelope::Request(request) => {
                        if handle_core_request(
                            process.transport(),
                            app_handle,
                            auth,
                            &security,
                            &registry,
                            session,
                            request,
                        )? {
                            continue;
                        }
                    }
                    IpcEnvelope::Event(event) => {
                        handle_core_event(
                            process.transport(),
                            app_handle,
                            &security,
                            &registry,
                            session,
                            event,
                            &mut incoming_event_sequence,
                            &mut sequence,
                        )?;
                    }
                    IpcEnvelope::Response(_) => return Err(CoreSupervisorError::Protocol),
                }
            }
            Err(IpcError::Timeout) => {
                missed_heartbeats += 1;
                if missed_heartbeats >= 4 {
                    return Err(CoreSupervisorError::HeartbeatTimeout);
                }
            }
            Err(_) => return Err(CoreSupervisorError::Transport),
        }
    }
}

fn send_host_command(
    transport: &mut dyn IpcTransport,
    session: &AuthenticatedSession,
    sequence: u64,
    command: &HostCommand,
) -> Result<(), CoreSupervisorError> {
    let payload = serde_json::to_vec(&json!({
        "correlation_id": command.correlation_id,
        "text": command.text,
    }))
    .map_err(|_| CoreSupervisorError::Protocol)?;

    send_core_event(
        transport,
        session,
        sequence,
        command.correlation_id.clone(),
        "core.input.text",
        payload,
    )
}

fn send_core_event(
    transport: &mut dyn IpcTransport,
    session: &AuthenticatedSession,
    sequence: u64,
    event_id: String,
    event_type: &str,
    payload: Vec<u8>,
) -> Result<(), CoreSupervisorError> {
    let event = EventEnvelope {
        protocol_version: crate::ipc::IPC_PROTOCOL_VERSION,
        event_id,
        session_id: *session.session_id().as_bytes(),
        sequence,
        event_type: event_type.to_string(),
        payload,
        proof: None,
    };
    let frame = encode_frame(&crate::ipc::JsonIpcCodec, &IpcEnvelope::Event(event))
        .map_err(|_| CoreSupervisorError::Protocol)?;
    transport
        .send_frame(&frame)
        .map_err(|_| CoreSupervisorError::Transport)
}

fn handle_core_event(
    transport: &mut dyn IpcTransport,
    app_handle: &AppHandle,
    security: &SecurityGateway,
    registry: &HostCapabilityRegistry,
    session: &AuthenticatedSession,
    event: EventEnvelope,
    last_event_sequence: &mut u64,
    outgoing_sequence: &mut u64,
) -> Result<(), CoreSupervisorError> {
    if event.session_id != *session.session_id().as_bytes() {
        return Err(CoreSupervisorError::Authentication);
    }

    if event.sequence <= *last_event_sequence {
        return Err(CoreSupervisorError::Authentication);
    }
    *last_event_sequence = event.sequence;

    let payload: Value =
        serde_json::from_slice(&event.payload).map_err(|_| CoreSupervisorError::Protocol)?;

    match event.event_type.as_str() {
        "core.status" | "core.response" | "core.error" => {
            app_handle
                .emit(
                    CORE_EVENT_NAME,
                    HostCoreEvent {
                        event_type: event.event_type,
                        event_id: event.event_id,
                        payload,
                    },
                )
                .map_err(|_| CoreSupervisorError::Transport)?;
            Ok(())
        }
        "capability.request" => {
            let payload: CapabilityRequestPayload =
                serde_json::from_value(payload).map_err(|_| CoreSupervisorError::Protocol)?;
            let capability_request = CapabilityRequest {
                capability_id: payload.capability_id.clone(),
            };
            let decision =
                security.authorize_at(now_ms(), Some(session), &capability_request, false);

            let response_payload = match decision {
                SecurityDecision::Allow => {
                    match registry.execute(&payload.capability_id, payload.input) {
                        Ok(value) => json!({
                            "request_id": payload.request_id,
                            "capability_id": payload.capability_id,
                            "ok": true,
                            "output": value,
                        }),
                        Err(error) => json!({
                            "request_id": payload.request_id,
                            "capability_id": payload.capability_id,
                            "ok": false,
                            "output": {},
                            "error": error,
                        }),
                    }
                }
                SecurityDecision::RequireConfirmation => json!({
                    "request_id": payload.request_id,
                    "capability_id": payload.capability_id,
                    "ok": false,
                    "output": {},
                    "error": "confirmation_required",
                }),
                SecurityDecision::Deny(_) => json!({
                    "request_id": payload.request_id,
                    "capability_id": payload.capability_id,
                    "ok": false,
                    "output": {},
                    "error": "capability_denied",
                }),
            };

            let request_id = response_payload
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let capability_id = response_payload
                .get("capability_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let event_id = format!("capability-result-{request_id}");
            let event_payload =
                serde_json::to_vec(&response_payload).map_err(|_| CoreSupervisorError::Protocol)?;
            send_core_event(
                transport,
                session,
                *outgoing_sequence,
                event_id,
                "capability.result",
                event_payload,
            )?;
            log::debug!("NAVEEN Core capability result sent: {capability_id}");
            *outgoing_sequence = outgoing_sequence.saturating_add(1);
            Ok(())
        }
        _ => Err(CoreSupervisorError::Protocol),
    }
}

fn handle_core_request(
    transport: &mut dyn IpcTransport,
    app_handle: &AppHandle,
    auth: &mut AuthenticationServer,
    security: &SecurityGateway,
    registry: &HostCapabilityRegistry,
    session: &AuthenticatedSession,
    request: RequestEnvelope,
) -> Result<bool, CoreSupervisorError> {
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

    if request.method == CORE_HEARTBEAT_METHOD && request.payload == CORE_HEARTBEAT_PAYLOAD {
        send_response(
            transport,
            session,
            request,
            ResponseStatus::Ok,
            None,
            b"alive".to_vec(),
        )?;
        return Ok(true);
    }

    if request.method == "core.capability.request" {
        let payload: CapabilityRequestPayload =
            serde_json::from_slice(&request.payload).map_err(|_| CoreSupervisorError::Protocol)?;
        let capability_request = CapabilityRequest {
            capability_id: payload.capability_id.clone(),
        };
        let decision = security.authorize_at(now_ms(), Some(session), &capability_request, false);

        let response_payload = match decision {
            SecurityDecision::Allow => {
                match registry.execute(&payload.capability_id, payload.input) {
                    Ok(value) => json!({
                        "request_id": payload.request_id,
                        "capability_id": payload.capability_id,
                        "ok": true,
                        "result": value,
                    }),
                    Err(error) => json!({
                        "request_id": payload.request_id,
                        "capability_id": payload.capability_id,
                        "ok": false,
                        "error": error,
                    }),
                }
            }
            SecurityDecision::RequireConfirmation => json!({
                "request_id": payload.request_id,
                "capability_id": payload.capability_id,
                "ok": false,
                "error": "confirmation_required",
            }),
            SecurityDecision::Deny(_) => json!({
                "request_id": payload.request_id,
                "capability_id": payload.capability_id,
                "ok": false,
                "error": "capability_denied",
            }),
        };

        let _ = app_handle;
        send_response(
            transport,
            session,
            request,
            ResponseStatus::Ok,
            None,
            serde_json::to_vec(&response_payload).map_err(|_| CoreSupervisorError::Protocol)?,
        )?;
        return Ok(true);
    }

    Err(CoreSupervisorError::Protocol)
}

fn send_response(
    transport: &mut dyn IpcTransport,
    session: &AuthenticatedSession,
    request: RequestEnvelope,
    status: ResponseStatus,
    error_code: Option<String>,
    payload: Vec<u8>,
) -> Result<(), CoreSupervisorError> {
    let response = ResponseEnvelope {
        protocol_version: crate::ipc::IPC_PROTOCOL_VERSION,
        correlation_id: request.correlation_id,
        session_id: *session.session_id().as_bytes(),
        sequence: request.sequence,
        status,
        error_code,
        payload,
        proof: None,
    };
    let frame = encode_frame(&crate::ipc::JsonIpcCodec, &IpcEnvelope::Response(response))
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
    if declared == 0 || declared > crate::ipc::IPC_MAX_WIRE_BODY_SIZE || frame.len() != declared + 4
    {
        return Err(CoreSupervisorError::Protocol);
    }

    serde_json::from_slice(&frame[4..]).map_err(|_| CoreSupervisorError::Handshake)
}

fn array_16(value: &[u8]) -> Result<[u8; 16], CoreSupervisorError> {
    value.try_into().map_err(|_| CoreSupervisorError::Handshake)
}

fn array_32(value: &[u8]) -> Result<[u8; 32], CoreSupervisorError> {
    value.try_into().map_err(|_| CoreSupervisorError::Handshake)
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

        let mut command = Command::new(&config.python_executable);
        command
            .arg("-E")
            .arg("-s")
            .arg(&config.script_path)
            .arg("--memory-db")
            .arg(&config.memory_db_path)
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::null())
            .env_clear()
            .current_dir(working_dir)
            .creation_flags(CREATE_NO_WINDOW);

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

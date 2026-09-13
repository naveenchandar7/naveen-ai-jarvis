# LIVE CORE CONNECTION

This milestone connects the existing Rust authentication/session boundary to a supervised Python Core skeleton over local Windows pipes.

## Scope

- Rust owns the child process and security boundary.
- Rust creates a per-launch secret and sends it exactly once over the private child stdin pipe.
- Existing HMAC challenge/response authentication establishes the existing in-memory session.
- Python sends only authenticated, versioned requests.
- The only live method is `core.health`; all other methods are rejected.
- Heartbeats use `core.health` / `heartbeat` and the host reconnects with exponential backoff after child exit, transport failure, protocol failure, authentication failure, or heartbeat timeout.
- No TCP listener is created.

## Process boundary

```text
Tauri/Rust host
  ├─ generates launch identity + secret
  ├─ creates two anonymous pipes
  ├─ launches Python child with stdin/stdout bound to those pipes
  ├─ bootstraps existing authentication material
  ├─ authenticates the exact child process
  ├─ validates every request with the existing session verifier
  └─ authorizes future capabilities through Security Gateway

Python Core
  ├─ receives bootstrap/challenge
  ├─ proves possession of the existing launch secret
  ├─ derives the existing session key
  └─ sends `core.health` heartbeats
```

The Python process is not trusted before authentication and receives no OS/device handles, Tauri permissions, capability policy, shell access, filesystem authority, or provider credentials.

## Bootstrap

The launch secret is delivered over the anonymous inherited pipe that is bound as the child process's stdin. The parent copies of both pipe directions are made non-inheritable. The child receives only the pipe ends intentionally attached to standard I/O.

The secret is never placed in command-line arguments, configuration, source-controlled files, logs, or React state. The child environment is cleared and only the minimum Windows runtime variables are restored.

## Authentication

The supervisor reuses `AuthenticationServer` and `AuthenticationProvider` from `auth.rs`.

```text
Rust → AuthBootstrap
Rust → AuthChallenge
Python → AuthResponse
Rust → AuthSession
```

No second token/session mechanism is introduced.

After the session is established, Python request proofs are verified through the existing session verifier. The session ID is only an identifier; possession of the ID alone is insufficient.

## IPC contract

Normal messages continue to use the existing length-delimited `IpcEnvelope` contract and `JsonIpcCodec` implementation.

Every request is checked for:

- protocol version;
- session ID;
- sequence number;
- correlation ID;
- canonical message material;
- existing authentication/session proof;
- payload and frame bounds.

Responses echo the request correlation/session/sequence and are accepted by the existing strict request/response matching rules.

## Health and lifecycle

Python emits `core.health` every two seconds. Rust treats four consecutive receive timeouts as heartbeat failure, closes the connection, terminates the child, invalidates the session, and retries with exponential backoff capped at 30 seconds.

Normal Tauri exit sets the supervisor stop flag, joins the supervisor thread, closes the pipe handles, and terminates/waits for the child if necessary.

## Replaceability

`CoreProcessLauncher` and `ManagedCoreProcess` isolate process supervision from the rest of the host. `IpcTransport`, `IpcCodec`, and `AuthenticationProvider` remain independent interfaces.

The current Windows implementation uses anonymous inherited pipes only because the target architecture is a single Rust-launched desktop child. A different local transport can be substituted behind `IpcTransport`/process launcher without changing the message contracts or authentication implementation.

## Deliberate non-goals

This milestone does not implement LLM, STT, VAD, TTS, memory, research, MCP, tools, browser control, computer control, or privileged capability execution.

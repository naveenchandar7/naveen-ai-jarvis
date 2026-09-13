# NAVEEN / JARVIS — Secure Rust↔Python IPC Foundation

## Status

This document records the implemented IPC foundation for the locked hybrid architecture. It does **not** launch Python, implement the Python Core, or expose any tools/capabilities.

The existing Core authentication/session mechanism remains the source of authentication. This milestone adds a protocol and transport boundary around that session; it does not create a second authentication system.

## 1. Locked boundaries

```text
React / Three.js
    = presentation only

Rust / Tauri
    = native host + security boundary + authn/authz + privileged execution

Python Core
    = cognition/orchestration only
```

React never receives the Rust↔Python transport endpoint and never talks to Python directly.

Python receives no unrestricted OS/device handle. Privileged execution remains:

```text
Python Core
  → Capability Request
  → Security Gateway
  → Rust Device Gateway
  → OS / device
```

## 2. Protocol version

Current IPC protocol version:

```text
IPC_PROTOCOL_VERSION = 1
```

Every request, response, and event carries the version. Unsupported versions are rejected before message handling.

The wire codec is behind `IpcCodec`. JSON is the current test/default codec, not a permanent protocol decision.

## 3. Message contracts

### Request

```text
RequestEnvelope {
    protocol_version
    correlation_id
    session_id
    sequence
    method
    payload
    proof
}
```

Requests require a 32-byte authentication proof and a non-zero sequence number.

The caller cannot provide permissions or risk levels in this envelope. Authorization policy remains host-owned in `security.rs`.

### Response

```text
ResponseEnvelope {
    protocol_version
    correlation_id
    session_id
    sequence
    status
    error_code?
    payload
    proof?
}
```

The response must keep the request's correlation ID and sequence number. `Ok` cannot carry an error code; `Error` and `Rejected` must carry one.

### Event

```text
EventEnvelope {
    protocol_version
    event_id
    session_id
    sequence
    event_type
    payload
    proof?
}
```

Events are intentionally generic so future Core state, task-progress, security, transcript, or research events can use the same envelope without coupling the transport to a provider.

## 4. Framing

Frames are length-delimited:

```text
4-byte big-endian body length
        +
encoded envelope body
```

The current maximum encoded body is 1 MiB and the maximum payload is 512 KiB.

The decoder rejects:

- missing/short length headers;
- zero-length bodies;
- oversized bodies;
- length mismatches;
- malformed codec data;
- unsupported protocol versions;
- invalid envelope fields.

Malformed protocol input is fail-closed and closes the connection.

## 5. Authentication integration

The IPC layer consumes the existing `AuthenticatedSession` and `AuthenticationProvider` from `auth.rs`.

For a Core request:

```text
wire request
  → protocol validation
  → session-id equality with the host's authenticated session
  → canonical IPC message material
  → existing AuthenticationProvider.verify_session_message()
  → request accepted/rejected
```

The canonical authentication material is independent of the JSON codec. It includes the message kind, protocol version, session ID, correlation/identity fields, sequence number, and payload. This prevents a proof calculated for one envelope kind from being reflected into another kind.

No session ID alone is sufficient for acceptance.

## 6. Request/response matching

`PendingRequests` tracks:

```text
correlation_id → (session_id, request_sequence)
```

A response is accepted only when all of these match exactly. A wrong correlation ID, wrong session, or wrong sequence is rejected.

Pending state is retained when a response mismatch occurs so a correct response is still matchable.

## 7. Replay and stale-session handling

Replay protection comes from the existing authentication/session layer for Core-originated requests:

- per-launch authenticated session;
- session expiry/revocation;
- strictly increasing message sequence;
- cryptographic proof over the canonical message material.

The IPC layer adds:

- session ID binding to the active connection;
- protocol version binding;
- strict request/response correlation;
- fail-closed connection shutdown after authentication/protocol rejection.

A session from a previous application launch cannot be accepted by a fresh authentication server instance.

## 8. Windows local transport

The current Windows transport abstraction is `WindowsLocalPipeTransport`.

It uses **two anonymous Windows pipes**, one per direction, and exposes a `ChildPipeHandles` object for the future Rust process supervisor.

```text
Rust parent
   │
   ├── parent read  ←── child write
   └── parent write ──→ child read
```

The transport does not open a TCP/UDP listener or a discoverable named endpoint.

The pipe handles are created inheritable for the future child and the Rust-side copies are explicitly marked non-inheritable. The actual process creation and handle-list transfer remain outside this milestone.

No credential or authentication secret is placed in a command line, source file, Git, React state, or ordinary environment variable by this transport layer.

## 9. Why this transport shape

Rust is the trusted parent and will eventually launch/supervise Python. A handle-based anonymous pipe keeps the transport private to that parent/child relationship and avoids the attack surface of a network listener.

The authentication design remains useful even with the private pipe: the OS channel controls which process can normally obtain the handles, while the existing cryptographic session proves that the connected Core speaks the expected per-launch protocol.

## 10. Timeout and lifecycle behavior

The transport API requires a timeout for receive operations.

Current behavior:

```text
timeout with no data → Timeout
peer closes/disconnects → Closed
malformed frame → protocol error + connection close
authentication failure → authentication error + connection close
explicit shutdown → idempotent close
```

The Windows anonymous-pipe implementation uses a bounded polling read based on the pipe's available-byte count so a receive deadline can be enforced without an open socket listener.

## 11. Audit safety

IPC audit events intentionally contain only:

- event type;
- timestamp;
- protocol version;
- correlation ID when valid;
- session ID when relevant;
- coarse rejection reason.

They never contain payload bytes, authentication proofs, session keys, launch secrets, passwords, or API keys.

The current audit buffer is bounded in memory.

## 12. Error vocabulary

The foundation distinguishes protocol, framing, timeout, closure, transport, authentication, session mismatch, correlation mismatch, and invalid-envelope failures internally.

A future external-facing transport adapter may collapse or redact detailed reasons where that reduces information disclosure.

## 13. Explicitly not implemented here

- Rust process launch/supervision of Python;
- bootstrap transfer of the existing per-launch auth secret;
- the actual Python client implementation;
- a live Rust↔Python socket/pipe connection;
- capability execution;
- Security Gateway integration with tools;
- STT/VAD/LLM/TTS;
- MCP;
- memory;
- browser/computer/filesystem tools;
- React↔Python communication.

The next milestone after this one may connect the existing authenticated session and these contracts to the Rust process supervisor and Python Core client. That work must not bypass the Security Gateway or introduce a second authentication mechanism.

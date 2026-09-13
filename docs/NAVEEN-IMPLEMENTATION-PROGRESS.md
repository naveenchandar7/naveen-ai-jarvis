# NAVEEN / JARVIS — Implementation Progress

The master architecture and `AGENTS.md` remain the architectural source of truth. This file records implementation milestones and verification status only.

## Starting checkpoint

- Requested baseline: `m0-secure-baseline` / `089c14c`
- The actual repository is re-inspected before each milestone; older audits are not treated as current state.

## Completed: Core Authentication / Session Boundary

The auth milestone provides a per-launch host secret, challenge/response, authenticated sessions, expiry/revocation, replay protection, and sanitized audit events. The session ID is an identifier only; the host authenticates requests using the in-memory session state and proof verification.

## Completed: Secure Rust↔Python IPC Foundation

Established the versioned message contracts, framing, codec/transport interfaces, strict correlation tracking, authenticated request verification, replay protection, bounded audit metadata, and a Windows-local anonymous-pipe transport. The transport layer remains independent from the authentication implementation.

## Completed: Live Rust↔Python Core Connection

The repository now contains the first real supervised Python Core skeleton and the Rust host-side lifecycle for it.

### Implemented

- `core/naveen_core/core_client.py` — Python client for the existing auth + IPC contracts.
- `core/naveen_core/main.py` — minimal Core process that authenticates and sends `core.health` heartbeats.
- `src-tauri/src/core_supervisor.rs` — replaceable process-launch interface plus Rust supervisor implementation.
- Existing `AuthenticationServer` is reused; no second token/session/authentication system is introduced.
- Per-launch bootstrap sends the existing host-created secret over the private child stdin pipe only after the child is launched.
- Windows child stdin/stdout are bound to the existing anonymous pipe transport; no TCP listener is created.
- Python child environment is cleared and only minimal Windows runtime variables are restored; the authentication secret is not passed through environment variables or command-line arguments.
- Successful challenge/response establishes the existing authenticated session, after which request proofs are verified by the existing authentication provider.
- Only `core.health` + `heartbeat` is accepted in this milestone; all other methods are rejected.
- Heartbeat timeout, child exit, authentication failure, protocol failure, and transport failure all close the connection, invalidate the session, terminate/wait for the child, and retry with capped exponential backoff.
- Tauri application exit signals the supervisor to stop and joins the supervisor thread before process shutdown completes.
- `tauri.conf.json` now bundles the `core/` directory as an application resource for release packaging.

### Security boundary preserved

```text
React / Three.js
        ↓ Tauri IPC only
Rust / Tauri host
        ↓ authenticated local IPC
Python Core
        ↓ future capability requests only
Security Gateway
        ↓
Rust Device Gateway
        ↓
OS / device
```

The live connection exposes no shell, filesystem, browser, process, MCP, microphone, model, or provider capability to Python.

## Tests and verification

### Executed in this environment

Python-side contract tests were executed locally against the same protocol shapes used by the new Core skeleton:

```text
python -m py_compile core_client.py test_core_client.py
python -m unittest -v
```

Result: **9 tests passed**.

The executed tests cover frame validation, canonical auth/session material, proof sequence binding, request material binding, successful auth + health contract, invalid bootstrap protocol, malformed/expired challenge input, and response correlation mismatch.

### Not executed here

The environment does not contain a Rust/Cargo toolchain or Windows runtime, so no `cargo test`, `cargo fmt`, `cargo clippy`, `cargo build`, or live Windows child-process smoke test is claimed.

Required local Windows verification:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Then run the Tauri desktop application and verify:

1. Rust launches the bundled/configured Python Core.
2. Bootstrap reaches only the child stdin pipe.
3. Challenge/response establishes the existing session.
4. `core.health` heartbeats remain stable.
5. Killing the Core causes clean detection, session invalidation, child cleanup, and reconnect backoff.
6. Closing the Tauri app terminates the child and releases pipe handles.
7. A malformed, replayed, expired, or unauthorized request is rejected and the connection fails closed.

## Explicitly deferred

- user authentication implementation
- richer Python Core cognition/orchestration
- Rust↔Python capability execution
- STT/VAD/LLM/TTS
- MCP
- memory/research
- browser/computer/filesystem tools
- provider/runtime selection
- cross-device adapters

## Next milestone boundary

The next milestone may expand the live connection into the real Python Core orchestration layer and a capability request path, but only through the existing authenticated/versioned IPC and Security Gateway. No privileged capability may be added by bypassing Rust authorization, and no new authentication or token scheme may be introduced.

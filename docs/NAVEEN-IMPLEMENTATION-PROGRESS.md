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

The repository contains the supervised Python Core process and Rust host-side lifecycle for it.

### Implemented

- `core/naveen_core/core_client.py` — Python client for the existing auth + IPC contracts.
- `core/naveen_core/main.py` — supervised Core process entry point.
- `src-tauri/src/core_supervisor.rs` — replaceable process-launch interface plus Rust supervisor implementation.
- Existing `AuthenticationServer` is reused; no second token/session/authentication system is introduced.
- Per-launch bootstrap sends the existing host-created secret over the private child stdin pipe only after the child is launched.
- Windows child stdin/stdout are bound to the existing anonymous pipe transport; no TCP listener is created.
- Python child environment is cleared and only minimal Windows runtime variables are restored; authentication secrets are not passed through environment variables or command-line arguments.
- Successful challenge/response establishes the existing authenticated session, after which request proofs are verified by the existing authentication provider.
- Heartbeat timeout, child exit, authentication failure, protocol failure, and transport failure close the connection, invalidate the session, terminate/wait for the child, and retry with capped exponential backoff.
- Tauri application exit signals the supervisor to stop and joins the supervisor thread before process shutdown completes.
- `tauri.conf.json` bundles the `core/` directory as an application resource for release packaging.

## Completed: Core Orchestration / Command Vertical Slice

The Python Core now crosses the locked React → Rust → authenticated Python architecture and returns structured responses to the HUD.

### Implemented

- Rust exposes `submit_text` as a Tauri command; React never talks directly to Python.
- `CoreSupervisor` maintains a bounded host-to-Core command queue and only accepts commands while an authenticated Core session is connected.
- Host commands are delivered to Python as authenticated-session IPC events.
- Python Core accepts host text events, performs deterministic intent routing, and emits structured `core.status`, `core.response`, and `core.error` events.
- Intent routing supports greeting, identity, help, explicit memory save/recall/forget, system status, research intent recognition, and a provider-independent conversation fallback.
- Python Core requests `system.telemetry.read` through a typed capability request instead of accessing the OS directly.
- Rust `HostCapabilityRegistry` executes host capabilities only after `SecurityGateway::authorize_at` permits them.
- Capability decisions log only capability identifiers and safe outcomes; request inputs and secrets are not logged.

## Completed: Provider Contract / Routing Hardening

The Core now exposes stable replaceable-provider boundaries without selecting permanent engines or requiring a backend installation.

### Provider boundaries

- `MemoryStore` — stable durable-memory contract; `SQLiteMemoryStore` is the current local implementation.
- `KnowledgeProvider` — stable indexed-document retrieval contract; `SQLiteKnowledgeStore` is the current local keyword-search implementation.
- `EmbeddingProvider` — separate semantic embedding boundary reserved for future RAG implementations; no embedding runtime is installed or selected.
- `ModelProvider` — model inference contract.
- `ModelManager` — task-aware provider routing for conversation, fast-response, reasoning, and research-synthesis roles. Providers can be registered independently; no model name is hardcoded.
- `ResearchProvider` — already isolated behind a provider boundary and remains host-capability mediated for network access.
- `VadProvider`, `SttProvider`, and `TtsProvider` — already isolated as voice provider contracts; concrete engines remain intentionally open.

The composition root (`core/naveen_core/main.py`) now injects the current memory and knowledge implementations explicitly into the Orchestrator. The Orchestrator depends on provider contracts rather than selecting its own storage backend.

This milestone does **not** choose or install a permanent database, vector database, embedding model, LLM, STT engine, VAD engine, TTS engine, or cloud provider.

## Current runtime scope

The current live runtime proves the Rust host can supervise and authenticate the Python Core and carry a text-command vertical slice through the Orchestrator. Deterministic identity/greeting/help paths remain pre-model bootstrap behavior; they are not the final LLM experience.

Current memory and knowledge implementations are provisional local implementations behind replaceable contracts. RAG is currently keyword retrieval; semantic/vector retrieval remains a future provider implementation.

## Current deferred capabilities

- real LLM inference/provider adapters and resource-aware model loading
- semantic/vector RAG and embedding provider implementation
- native STT/VAD/TTS and wake-word/barge-in pipeline
- network-backed research provider
- broader filesystem/browser/software integrations
- broader device adapters
- MCP integrations behind capability policy
- multi-device synchronization
- controlled self-improvement workflows

A model such as the user-mentioned “Needle 2” may be evaluated later as a **candidate** for a specific fast-response role, but it is not selected or installed as part of this milestone.

## Verification targets

Run locally after pulling the milestone:

```text
python -m unittest discover -v core/naveen_core
Get-ChildItem core\naveen_core\*.py | ForEach-Object { python -m py_compile $_.FullName }
```

Rust verification remains:

```text
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --all-features -- -D warnings
cargo build --manifest-path src-tauri/Cargo.toml
```

Live Windows verification should still cover Core bootstrap/authentication, stable connection, text round-trip, capability authorization, memory persistence, reconnect behavior, and fail-closed handling.

## Next logical milestone

Evaluate concrete provider candidates **without coupling them into Core**: first a resource-aware model/provider selection layer, then native voice providers and semantic RAG/embedding support. Selection will be based on the actual Windows hardware/runtime constraints, language coverage, latency, quality, licensing, and offline/cost requirements.

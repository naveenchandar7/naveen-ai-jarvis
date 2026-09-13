# NAVEEN Core

The Python Core now contains the first real orchestration vertical slice behind the existing authenticated Rust↔Python boundary.

Current scope:

- authenticated versioned IPC client
- host-to-Core text events
- deterministic intent routing
- explicit SQLite long-term memory with namespaces and forget
- replaceable `ModelProvider` / `ModelManager` boundary with an offline fallback
- typed capability requests from Core to the Rust Security Gateway
- structured Core status, response, and error events
- `core.health` heartbeat contract

The Core does not have unrestricted operating-system access. Host/device operations remain Rust-owned and policy-gated.

Provider-backed LLM, native STT/VAD/TTS, live research, browser/computer control, broader device adapters, and MCP integrations remain intentionally deferred until their provider and host boundaries are implemented and verified.

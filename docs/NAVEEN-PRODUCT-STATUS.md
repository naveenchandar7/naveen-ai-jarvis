# NAVEEN AI — Product Status

## What is real today

NAVEEN is a Tauri desktop architecture with a React presentation layer, a Rust security/native boundary, and a supervised Python Core.

The current Core supports:

- authenticated Rust↔Python startup and session lifecycle
- versioned framed IPC with request/response/event envelopes
- bounded text-command input
- English, Tamil, and Thanglish intent routing
- bounded working conversation context
- explicit long-term memory backed by SQLite
- approved-workspace text-file reads through the Rust capability gateway
- a local lexical document index for the first RAG slice
- host-mediated model routing through a configurable endpoint
- host-mediated network text retrieval for configured research sources
- host system telemetry through a typed Rust capability
- reconnect, heartbeat, timeout, shutdown, and audit handling

## Provider boundaries

The Core does not hardcode a permanent LLM, STT, VAD, TTS, embedding model, vector database, browser provider, or cloud vendor.

The current offline model fallback is intentionally deterministic and clearly labeled. Real model execution is available only when the host network/model configuration is supplied.

## Security boundary

React never talks directly to Python.

Python cannot directly invoke arbitrary OS, filesystem, process, browser, audio, or network APIs. It submits named capability requests. Rust validates the authenticated session, applies host-owned authorization policy, and executes only registered capability adapters.

Network access is also a capability and remains deny-by-default unless the host configuration permits the requested origin.

## Current limitations

The repository still requires Windows runtime verification for the desktop process supervisor, anonymous-pipe transport, CPAL microphone behavior, model runtime, and packaged release path.

The next provider milestones are real VAD/STT/TTS, stronger retrieval/indexing, model lifecycle/resource management, and additional explicitly permissioned capabilities. These must remain behind the existing security gateway.

## Verification policy

A feature is described as verified only when its test or runtime environment has actually executed it. The repository CI runs Python, frontend, and Rust checks on supported runners; local Windows smoke testing remains separate because it exercises the user's actual desktop/audio/model environment.

# AGENTS.md — NAVEEN / JARVIS engineering rules

Permanent rules for coding agents working in this repository.

The full blueprint is [`docs/NAVEEN-MASTER-ARCHITECTURE.md`](docs/NAVEEN-MASTER-ARCHITECTURE.md). If this file and application code disagree, **stop and ask**; do not silently invent a new architecture.

## What this project is

NAVEEN / JARVIS is a long-term **standalone personal AI assistant and second brain**.

Display names in [`src/config/assistantConfig.ts`](src/config/assistantConfig.ts):

- Product: N.A.V.E.E.N AI
- Visual core: NOVA
- Wake name: JARVIS

This repository currently contains a **desktop HUD shell** (React + Vite + Three.js) and a **thin Tauri 2 / Rust sidecar** (CPAL microphone RMS + sysinfo telemetry). JARVIS Core, intent/planning, memory, research, model routing, capability execution, and the security gateway are **not implemented yet**.

## Non-negotiable split

| Layer | Lives in | May do | Must not do |
| --- | --- | --- | --- |
| Presentation | `src/` React HUD + R3F | Display state, accept non-privileged input, subscribe to events | Orchestrate, infer, store secrets, own privileged OS work |
| JARVIS Core | Future Rust crate hosted by Tauri | Orchestrate, plan, route models, call capabilities | Depend on React components |
| Device Gateway | `src-tauri/` adapters | Hardware I/O behind traits | Leak OS-specific calls into Core or UI |
| Security Gateway | Future native core + Tauri ACL | Authorize every privileged action | Be bypassed by UI `invoke` or MCP |

**Cognition and privileged operations must not be implemented inside React UI components.**

## Replaceability

All major subsystems must be replaceable via **interfaces, traits, adapters, providers, configuration, and registries**.

Never hardcode:

- LLM provider or model
- STT / TTS provider
- database or memory implementation
- tool / capability provider
- hardware device
- OS-specific behavior where an abstraction belongs
- filesystem absolute paths
- API keys or secrets
- capability permissions
- cloud dependencies

The system must survive swapping Gemma, Ollama, whisper.cpp, sherpa-onnx, Kokoro, SQLite/vector stores, laptop vs phone, or the HUD itself **without rewriting Core**.

Name specific engines only as **replaceable candidates** in config and docs.

## Where code belongs today

Keep existing modules in their roles. Do not “temporarily” put Core logic in the HUD.

**Presentation (keep)**

- HUD: `src/components/hud/`
- Nova Core visuals: `src/components/core/`
- Theme: `src/config/theme.ts`, `src/config/themeManager.ts`
- Visual/UX state: `src/state/assistantState.ts` (visual state only, not cognition)
- HUD presentation model: `src/config/hudModel.ts`
- Thin IPC client: `src/services/native/tauriBridge.ts`

**Native host (evolve as Device Gateway)**

- Commands: `src-tauri/src/lib.rs` (`jarvis_ping`, `get_system_info`, mic start/stop/level)
- Audio capture: `src-tauri/src/audio.rs` (CPAL)
- IPC ACL (not the capability registry): `src-tauri/capabilities/default.json`

**Voice contracts (keep as boundaries; do not fill with webview STT)**

- `src/services/voiceTypes.ts`
- `src/services/voice/voiceEngine.ts`
- `src/services/voice/stt/sttTypes.ts`
- `src/services/voice/stt/sttProvider.ts` (factory only)
- `src/services/voice/voiceController.ts` (not wired from `App.tsx` yet)

Target Core location: **Rust workspace crate** (for example `crates/jarvis-core`), hosted by the Tauri binary. Do not place Core in `src/App.tsx`.

## Voice rules

Native pipeline only:

`CPAL / Device Gateway → PCM → VAD → SttProvider → text event → JARVIS Core`

- Do **not** put audio buffers or STT inference in the webview.
- Do **not** use browser `SpeechRecognition`.
- Do **not** treat HUD RMS gating in `VoicePanel` as VAD.
- The UI must **not** own microphone lifecycle long-term (`App.tsx` currently starts CPAL on mount; that is technical debt, not the target).
- Prefer events (`audio://level`, `vad://state`, `stt://partial`, `stt://final`) over `setInterval` + `invoke` polling.

Evaluate and wrap mature implementations behind `SttProvider`: **whisper.cpp**, **sherpa-onnx**, **Silero VAD**. Do not rebuild them.

## Model rules

Conceptual chain: `ModelProvider → ModelRouter / ModelManager → task-appropriate provider/model`.

Support local models, Ollama, llama.cpp, and future/optional cloud providers **behind the same traits**. No model is permanent.

On the initial target (Windows, 8 GB RAM, no NVIDIA GPU): never assume VRAM; avoid loading heavy STT and LLM models at the same time; unload/switch via a Resource Manager.

## Capability rules

JARVIS tools are **registry-driven**, not hardcoded in UI.

Each capability must declare: unique ID, input/output schema, required permissions, risk level, device requirements, execution adapter, verification strategy.

Example domains: computer, files, browser, coding, research, vision, voice, GitHub, automation, device management.

**Tauri capabilities JSON is IPC ACL only.** It is not the JARVIS Capability Registry.

MCP may be an interoperability protocol. **MCP must not bypass** JARVIS security, permissions, or audit.

## Security rules

Defense in depth. Deny by default.

Include (as the system grows): authentication, authorization, least privilege, capability-level permissions, confirmation for dangerous operations, secrets isolation, audit logs, sandboxing, network policy, safe/offline mode, kill switch, rollback, suspicious-activity detection, prompt-injection defenses for external content, secure updates.

JARVIS must never:

- bypass its own permission system
- silently elevate privileges
- disable authentication
- reveal secrets
- silently grant itself new capabilities
- execute arbitrary untrusted code without policy approval
- modify its security layer without controlled review

“User said so” is **not** enough for dangerous actions. High-risk actions need authentication and/or explicit confirmation.

Secrets never live in the frontend, git, or logs.

## Memory and research

Memory is first-class and **native**. Do not blindly persist every conversation.

Separate: working context, session, long-term, structured knowledge, project, learning, ideas, decisions, goals, preferences, documents.

Do not confuse:

- sysinfo RAM (`get_system_info`)
- HUD `MemoryPanel` (unused presentation)
- semantic / second-brain memory (future Core)

Research is a **capability**: if the answer may be unknown or stale → research → verify sources → synthesize → optional memory → response. Never assume the LLM is current or correct.

## Device rules

Stable **Device Gateway**. Today: Windows laptop, sysinfo telemetry, CPAL mic. Future: phone, Raspberry Pi, other devices.

OS- and hardware-specific code stays in adapters. Core talks to traits.

## Frontend rules

UI may: display state/telemetry, take non-privileged input, subscribe to core/device events.

UI must not: orchestrate, execute privileged OS operations, contain model-provider logic, contain secrets, own the microphone lifecycle, bypass security, or become the memory system.

`src/state/assistantState.ts` is **visual/UX state**. Keyboard 1–4 in `useAssistantKeyboard` is a HUD debug aid, not Core.

## Events over polling

Prefer events for telemetry, audio level, VAD, transcripts, core state, task progress, and security. Avoid unnecessary frontend polling (`App.tsx` currently polls sysinfo at 2s and mic level at 120ms).

## Reuse-first

Before building a major subsystem:

1. Inspect this repository
2. Search mature open-source implementations
3. Check license
4. Check maintenance/activity
5. Check platform compatibility (Windows CPU, 8 GB)
6. Evaluate resource usage
7. Define an adapter boundary
8. Reuse / wrap / adapt
9. Build from scratch only when necessary

Smart work over unnecessary custom implementation.

## Workflow

Major changes:

Architecture / research → plan → review → implement → tests → build → runtime verification → `git diff` → review → commit

- Do not make broad unrelated changes in one phase.
- Preserve completed functionality unless the change is an intentional replacement.
- Do not install packages, change dependencies, or refactor unless the task explicitly requires it.
- Do not commit unless asked.

## Testing (required as native/core appears)

Unit tests, integration tests, failure-path tests, permission tests for privileged actions, and CPU/RAM measurements for local AI.

## Scope discipline

- Documentation-only tasks: do not modify application source.
- Do not choose a permanent LLM/STT/TTS/database.
- Do not clone drive-by dependencies into Core.
- When adding a provider, add an adapter + config key, not a global singleton import in React.

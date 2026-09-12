# AGENTS.md — NAVEEN / JARVIS engineering rules

Permanent rules for coding agents working in this repository.

The full blueprint is [`docs/NAVEEN-MASTER-ARCHITECTURE.md`](docs/NAVEEN-MASTER-ARCHITECTURE.md). If this file and application code disagree, **stop and ask**; do not silently invent a new architecture.

**Implementation is now active.** Preserve the locked boundaries below while building the system. Do not install heavy AI runtimes until their contracts and security boundaries are ready.

## What this project is

NAVEEN / JARVIS is a long-term **standalone personal AI assistant and second brain**.

Display names in [`src/config/assistantConfig.ts`](src/config/assistantConfig.ts):

- Product: N.A.V.E.E.N AI
- Visual core: NOVA
- Wake name: JARVIS

This repository currently contains a **desktop HUD shell** (React + Vite + Three.js) and a **thin Tauri 2 / Rust host** (sysinfo telemetry plus a retained CPAL audio adapter that is not yet exposed through a privileged voice-session path). The Python AI Core, Capability Registry, Security Gateway, Device Gateway traits, and secure host↔core IPC **are not implemented yet**.

## Decision classes (read first)

| Class | Meaning |
| --- | --- |
| **LOCKED** | Must not be violated. Change only by explicit architecture revision of this file and the master blueprint. |
| **PROVISIONAL** | A non-invariant implementation decision. It may change only behind the locked boundaries. |
| **OPEN** | Must not be hardcoded. Decide later behind interfaces. |

## LOCKED split

| Layer | Lives in | May do | Must not do |
| --- | --- | --- | --- |
| Presentation | `src/` React HUD + R3F | Display state, accept non-privileged input, subscribe to events | Orchestrate, infer, store secrets, own privileged OS work, talk to Python Core directly |
| Privileged native | Rust / Tauri host | Device I/O, OS ops, security gateway, authn/authz, sandbox, secure IPC | Leak unrestricted OS access to Python or the webview |
| Cognition | Python JARVIS AI Core | Orchestrate, plan, route models, memory, research, request capabilities | Execute privileged OS operations itself |

**Cognition and privileged operations must not be implemented inside React UI components.**

**Python AI Core must not receive unrestricted operating-system access.**

Privileged path (locked):

`AI Core → Capability Registry → Security / Permission Gateway → Rust Device Gateway → Device / OS`

## LOCKED hybrid architecture

```text
React / Three.js     → presentation client only: render state/events and accept non-privileged input
Rust / Tauri         → native host, Device Gateway, CPAL/device access, telemetry, privileged OS/device work, Security / Permission Gateway, authn/authz, sandbox/isolation, secure IPC, future device adapters
Python JARVIS AI Core → orchestrator, intent, planning, model router/providers, memory/research orchestration, AI-side capability catalog, MCP and future AI/agent integrations
```

Communication (LOCKED boundary):

```text
React  ↔  Rust host          (Tauri IPC; existing tauriBridge pattern)
Rust host  ↔  Python AI Core (versioned, authenticated, least-privilege IPC with explicit message contracts)
```

The webview must never open a private channel to Python that bypasses the Rust host.

Python may request capabilities only. It must never receive unrestricted OS/device access; privileged execution always follows `AI Core → Capability Request → Security / Permission Gateway → Rust Device Gateway → Native OS / Device → Result → AI Core`.

Exact Python runtime/packaging, IPC protocol, and process layout remain **OPEN**.

## Replaceability (LOCKED)

All major subsystems must be replaceable via **interfaces, traits, adapters, providers, configuration, and registries**.

Never hardcode:

- LLM provider or model (not Gemma, Qwen, Llama, Ollama, or any model filename)
- STT / TTS provider
- database or memory implementation
- tool / capability provider
- hardware device
- OS-specific behavior where an abstraction belongs
- filesystem absolute paths
- API keys or secrets
- capability permissions
- cloud dependencies

Name specific engines only as **replaceable candidates** in config and docs.

## Where code belongs today

Keep existing modules in their roles. Do not put Core logic in the HUD. Do not give Python a backdoor to CPAL or the filesystem.

**Presentation (keep)**

- HUD: `src/components/hud/`
- Nova Core visuals: `src/components/core/`
- Theme: `src/config/theme.ts`, `src/config/themeManager.ts`
- Visual/UX state: `src/state/assistantState.ts` (visual state only)
- HUD presentation model: `src/config/hudModel.ts`
- Thin IPC client: `src/services/native/tauriBridge.ts` (React ↔ Rust only)

**Native host (evolve as Device Gateway + security boundary)**

- Host telemetry and approved commands: `src-tauri/src/lib.rs`
- Audio adapter: `src-tauri/src/audio.rs` (CPAL; no webview-owned capture lifecycle)
- IPC ACL (not the capability registry): `src-tauri/capabilities/default.json`

**Voice contracts (keep; do not fill with webview STT)**

- `src/services/voiceTypes.ts`
- `src/services/voice/voiceEngine.ts`
- `src/services/voice/stt/sttTypes.ts`
- `src/services/voice/stt/sttProvider.ts` (factory only)
- `src/services/voice/voiceController.ts` (not wired from `App.tsx` yet)

Do not place JARVIS Core in `src/App.tsx`. Host health or telemetry is not Core.

## Voice rules (LOCKED pipeline; OPEN engines)

Native audio stays **outside the webview**:

`CPAL → PCM pipeline → VAD → SttProvider → text event → JARVIS Core`

- Do **not** put audio buffers or STT inference in the webview.
- Do **not** use browser `SpeechRecognition`.
- Do **not** treat HUD RMS gating in `VoicePanel` as VAD.
- The UI must **not** own microphone lifecycle or start native capture automatically.
- Prefer events over polling.

STT candidates (replaceable, not locked): **sherpa-onnx**, **whisper.cpp**. VAD candidate: **Silero**. TTS must use a **TtsProvider** abstraction. Exact engines remain **OPEN**.

Capture stays in the Rust Device Gateway. Whether VAD/STT run in the host or a constrained adapter is **OPEN**; Python must not get an always-on raw mic without the gateway.

## Model rules (LOCKED interface; OPEN implementations)

`ModelProvider → ModelRouter / ModelManager → task-appropriate provider`

Must support replaceable providers such as: Ollama, llama.cpp, OpenAI-compatible APIs, future local/cloud. **No provider or model is permanent.**

Do not hardcode Gemma, Qwen, Llama, Ollama, or any GGUF/ONNX filename in Core or UI.

Resource Manager is required on the initial target (Windows, 8 GB RAM, no NVIDIA GPU).

## Memory rules (LOCKED interface; OPEN backend)

Memory is first-class, behind `MemoryStore`. Do not blindly persist every conversation.

Candidates (not locked): SQLite/vector, Mem0, MemPalace, LanceDB, future systems.

Do not confuse sysinfo RAM, HUD `MemoryPanel`, and semantic memory.

## Capability rules (LOCKED)

Registry-driven. Example domains: computer, filesystem, browser, coding, research, GitHub, voice, vision, automation, device management.

Each capability: unique ID, input/output schema, required permissions, risk level, device requirements, execution adapter, verification strategy.

**Tauri capabilities JSON is IPC ACL only.** It is not the JARVIS Capability Registry.

MCP may integrate tools. **MCP must not bypass** the permission gateway. MCP servers that touch the OS still go through Rust Device Gateway.

## Security rules (LOCKED)

Defense in depth:

Tauri ACL + Rust security gateway + authentication + authorization + least privilege + capability policies + sandbox + secret isolation + audit logging + network policy + safe/offline mode + kill switch + rollback.

The AI must never:

- bypass authentication or authorization
- grant itself privileges
- disable security controls
- expose secrets
- silently execute arbitrary untrusted code
- modify its own security controls without controlled review

“User said so” is **not** enough for dangerous actions.

Python Core talks to the host only through **versioned, authenticated, least-privilege IPC**. No unrestricted shell, FS, or network from Core by default.

Secrets never live in the frontend, git, or logs.

## Device rules (LOCKED abstraction)

Stable **Device Gateway** in the Rust host. Today: Windows laptop, sysinfo, CPAL. Future: GPU upgrade, phone, Raspberry Pi, other devices. Hardware-specific logic stays in adapters.

## Frontend rules (LOCKED)

UI may: display state/telemetry, take non-privileged input, subscribe to events.

UI must not: orchestrate, execute privileged OS operations, contain model-provider logic, contain secrets, own the microphone lifecycle, bypass security, become the memory system, or spawn/talk to Python directly.

`src/state/assistantState.ts` is **visual/UX state**. Keyboard 1–4 is a HUD debug aid, not Core.

## Events over polling

Prefer events for telemetry, audio level, VAD, transcripts, core state, task progress, security. Avoid unnecessary frontend polling.

## Reuse-first

Before building a major subsystem: inspect repo → search mature OSS → license → maintenance → platform (Windows CPU, 8 GB) → resource use → adapter boundary → wrap → build from scratch only if necessary.

Do **not** install heavy AI runtimes (Ollama, llama.cpp, sherpa, vector DBs, Python ML stacks) until IPC contracts, capability schemas, and the security gateway boundary are designed.

## Workflow

Architecture / research → plan → review → implement → tests → build → runtime verification → `git diff` → review → commit

- No broad unrelated changes in one phase.
- Preserve completed functionality unless intentionally replacing it.
- Documentation-only tasks: do not modify application source or dependencies.
- Do not commit unless asked.

## Testing (when implementation starts)

Unit, integration, failure-path, permission tests for privileged actions, IPC contract tests (React↔Rust and Rust↔Python), resource measurements for local AI.

## Scope discipline

- Do not implement Core or IPC in this documentation phase.
- Do not choose a permanent LLM/STT/TTS/database.
- Do not add a Python process that can `invoke` OS APIs without the gateway.
- When adding a provider later, add an adapter + config key, not a singleton in React.

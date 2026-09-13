# NAVEEN AI — Product Status

The master architecture and `AGENTS.md` remain the source of truth for the locked hybrid design.

## Implemented

- React / Three.js presentation HUD remains separate from cognition and privileged execution.
- Rust / Tauri host is the native security boundary.
- Rust↔Python authentication/session foundation and versioned local IPC are implemented.
- Supervised Python Core startup, heartbeat, timeout, reconnect and shutdown paths exist.
- Core text orchestration supports bounded conversation context, intent routing, explicit memory, knowledge indexing and host-mediated research.
- English, Tamil and Thanglish intent patterns are covered by Core tests.
- SQLite-backed durable memory is explicit and policy-controlled; working conversation context is bounded and non-persistent.
- Workspace text-file reads are host-mediated and confined to an explicitly configured workspace.
- Network and model access are host-mediated capabilities with configuration-based allowlisting; network matching uses exact normalized origins.
- The HUD can submit text through Rust and display Core events without talking directly to Python.
- Native CPAL microphone control/level telemetry remains host-owned; raw PCM does not enter React.
- Provider abstractions exist for model routing and research retrieval without permanently selecting a vendor.
- CI now checks Python, frontend, and Rust on Windows and Linux.

## Not yet production-complete

These are intentionally not claimed as complete until their runtime dependencies and machine behavior are verified:

- Real Windows packaged Core runtime using a bundled Python distribution rather than a developer-installed interpreter.
- Real model runtime installation/configuration and resource-aware model lifecycle.
- Production VAD/STT/TTS adapters and full voice turn orchestration.
- Broader capabilities such as browser, application control, filesystem write, GitHub actions and automation; each must receive its own typed host policy and verification path.
- Advanced semantic/vector RAG, source ranking and provenance-aware retrieval.
- Multi-device gateway adapters.
- Controlled self-improvement workflow.

## User-required machine setup

The repository can be developed without secrets in Git. A real model provider, optional research endpoint and Windows Core runtime may require local configuration on the user's machine. Those values must remain outside source control.

## Verification policy

A feature is described as verified only when the relevant automated or runtime environment actually executes it. CI is the source for Rust/frontend/Python build checks; Windows desktop smoke testing is still required for real microphone, child-process, packaging and model-runtime behavior.

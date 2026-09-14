# NAVEEN Core

The Python Core contains the first real orchestration vertical slice behind the existing authenticated Rust↔Python boundary.

Current scope:

- authenticated versioned IPC client
- host-to-Core text events
- deterministic intent routing
- explicit local memory behind the `MemoryStore` contract
- indexed document retrieval behind the `KnowledgeProvider` contract
- separate `EmbeddingProvider` boundary reserved for semantic RAG
- task-aware replaceable `ModelProvider` / `ModelManager` routing
- host-mediated research behind `ResearchProvider`
- voice `VadProvider` / `SttProvider` / `TtsProvider` contracts
- typed capability requests from Core to the Rust Security Gateway
- structured Core status, response, and error events
- `core.health` heartbeat contract

## Provider architecture

The Core depends on stable contracts, while concrete implementations are selected at the composition root. The current implementations are provisional and intentionally replaceable.

```text
Orchestrator
  ├── MemoryStore
  │     └── current: SQLiteMemoryStore
  ├── KnowledgeProvider
  │     └── current: SQLiteKnowledgeStore (keyword retrieval)
  ├── ModelManager
  │     └── ModelProvider(s), selected per task
  └── ResearchProvider

Voice contracts
  ├── VadProvider
  ├── SttProvider
  └── TtsProvider

Optional RAG extension
  └── EmbeddingProvider
```

Model tasks are role-based (`conversation`, `fast_response`, `reasoning`, `research_synthesis`) rather than model-name based. Different providers may be registered for different tasks without changing the Orchestrator. No model runtime is installed or selected by this layer.

The Core does not have unrestricted operating-system access. Host/device operations remain Rust-owned and policy-gated.

Provider-backed LLM inference, native STT/VAD/TTS execution, semantic/vector RAG, live research, browser/computer control, broader device adapters, and MCP integrations remain deferred until their provider and host boundaries are implemented and verified.

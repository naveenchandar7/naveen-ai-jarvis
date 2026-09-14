# NAVEEN AI — Architecture Audit Baseline

**Baseline commit:** `bdf442439b1a19d72805468a114c8be32fdbd199`
**Purpose:** classify existing implementation against `docs/NAVEEN-ARCHITECTURE-V2-LOCK.md` before further feature work.

## KEEP — correct foundation

### Python model boundary
`core/naveen_core/model.py`

- `ModelConfig` is provider-independent.
- `ModelProvider` is an interface boundary.
- `ModelProviderResolver` separates provider construction from task routing.
- `ModelProviderRegistry` keeps provider instances replaceable.
- Task routing is separate from provider construction.
- Current tests cover configuration, resolver behavior, duplicate rejection, unknown-provider failure, and task routing.

**Action:** keep architecture; continue strengthening around capability metadata, lifecycle, resources, health, and streaming without reintroducing vendor coupling.

### MemoryStore contract
`core/naveen_core/memory.py`

- `MemoryStore` is already a stable protocol.
- Current SQLite implementation is explicitly replaceable.
- Namespaces and forget/recall operations already exist.

**Action:** keep the contract; strengthen policy-gated writes, richer memory semantics, indexing/retrieval, provenance, lifecycle, and provider selection. SQLite must remain an implementation, never the permanent architecture.

### KnowledgeProvider / EmbeddingProvider boundary
`core/naveen_core/knowledge.py`

- `KnowledgeProvider` is replaceable.
- `EmbeddingProvider` is already reserved as an abstraction.

**Action:** keep interfaces; strengthen toward hybrid/semantic retrieval, agentic retrieval, provenance, knowledge graph/context retrieval, and pluggable backends. Do not turn the current keyword search into the permanent RAG design.

### Python orchestration boundary
`core/naveen_core/orchestrator.py`

- Orchestration is separated from model implementation.
- Memory, knowledge, research, and capability requests are injected dependencies.
- Host capability requests are already represented through a callback boundary.

**Action:** keep the high-level role; evolve it toward planner/agent loops, capability discovery, model selection, verification, and self-evaluation rather than adding direct provider/device logic.

### Rust host security direction
Current Rust tree includes `security.rs`, `auth.rs`, `capabilities.rs`, `ipc.rs`, `core_supervisor.rs`, `network_gateway.rs`, and `device_gateway.rs`.

**Action:** keep the Rust security boundary as the trusted host-side enforcement layer; audit each module before extending it.

## STRENGTHEN — right direction, not final-grade yet

### Capability Catalog
`core/naveen_core/capabilities.py`

Current catalog has only a static system telemetry descriptor. The final architecture requires registry-driven capability metadata including input/output schema, permission requirements, risk, device requirements, provider/adapter identity, verification, and lifecycle metadata.

**Planned:** evolve into a provider/adapter-neutral capability contract and discovery system. Do not hardcode future device types here.

### Device Gateway
`src-tauri/src/device_gateway.rs`

Current implementation is a local Windows system telemetry adapter. That is a valid **local gateway adapter**, but it is not the complete future Device Fabric.

**Planned:** preserve this local gateway role and add a Device Fabric layer above it: node registry, identity/trust, capability advertisement, node selection/routing, transport abstraction, events, health/resources, and enrollment/revocation.

### Host / Core integration
Rust already has IPC/supervisor/security modules and Python has capability request flow.

**Planned:** converge on versioned, authenticated, least-privilege contracts; remove assumptions that the laptop is the only execution node; expose device and capability identity through contracts rather than concrete hardware code.

### Research
`core/naveen_core/research.py` and orchestrator integration are useful foundations.

**Planned:** treat research as an agentic capability with source verification, multi-step retrieval, evidence management, model routing, and optional memory/knowledge promotion.

## REPLACE — direct coupling that should not become deeper

### Fixed keyword retrieval as the long-term knowledge strategy
`SQLiteKnowledgeStore.search()` is a valid bootstrap backend but should not be expanded into the final retrieval architecture.

**Replace at the abstraction level, not by deleting the current provider immediately:** introduce richer retrieval interfaces and an agentic retrieval controller while retaining the SQLite backend as a fallback/local implementation.

### Hardware-specific assumptions inside Core
Any future device behavior added directly to Python Core based on concrete device names/types is prohibited.

**Replace with:** Device Fabric capability discovery and routing.

### Single-host Device Gateway as the only device abstraction
Do not make the existing Rust `device_gateway.rs` grow into a monolith containing phone/Pi/ESP32/robot logic.

**Replace with:** local gateway + device adapters + Device Fabric registry/router.

### Provider-specific model logic in orchestration
Do not add model/vendor branches to `orchestrator.py` or other Core modules. New providers belong behind `ModelProvider`, resolver, capability metadata, and future lifecycle/resource managers.

## Immediate implementation order

1. Lock architecture V2 (done in `NAVEEN-ARCHITECTURE-V2-LOCK.md`).
2. Harden model provider contracts: capability metadata, health, resource requirements, lifecycle hooks, and streaming without vendor lock-in.
3. Convert capability catalog into a full registry contract.
4. Introduce Device Fabric contracts: node identity, capability advertisement, health, routing, and transport-neutral messaging.
5. Audit Rust security/auth/IPC/network modules against the distributed-node boundary before adding device integrations.
6. Upgrade memory/knowledge interfaces for long-lived personal memory and agentic retrieval.
7. Add lifecycle/evolution infrastructure only after verification, sandbox, rollback, and trust boundaries are explicit.
8. Then add concrete providers, n8n, mobile, Raspberry Pi, ESP32, robots, and other integrations as adapters.

## Rule

Do not add a new feature that deepens a `REPLACE` dependency. New code should target the locked final contracts directly.

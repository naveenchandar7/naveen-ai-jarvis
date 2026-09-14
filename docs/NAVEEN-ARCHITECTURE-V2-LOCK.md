# NAVEEN AI — Architecture V2 Lock

**Status: LOCKED — normative architecture amendment**

This document is a permanent architectural amendment to `docs/NAVEEN-MASTER-ARCHITECTURE.md`. It locks the distributed, provider-independent, hardware-independent direction required for long-term NAVEEN AI use. Future changes are allowed only by explicitly revising this lock and preserving the invariants below.

## 1. Core principle

NAVEEN is not a single-model desktop assistant. It is a long-lived personal AI platform composed of interchangeable intelligence, memory, capability, plugin, automation, device, and lifecycle components.

The Core depends on **contracts and capabilities**, not concrete vendors, model files, hardware types, or transport implementations.

## 2. Locked top-level architecture

```text
                         NAVEEN AI
                    Cognitive Control Plane
                              |
        +---------------------+----------------------+
        |                     |                      |
        v                     v                      v
 Intelligence Fabric    Memory/Knowledge       Capability Fabric
 Models + Router        Memory + RAG + Graph   Tools + Plugins + MCP
 Planning + Agents      Research + Learning    Connectors + n8n
        |                     |                      |
        +---------------------+----------------------+
                              |
                         Device Fabric
        Registry | Identity | Routing | Events | Health
                              |
                         Node / Edge Layer
        PC | Mobile | Raspberry Pi | ESP32 | Robots | Future
                              |
                      Security / Trust Plane
       Auth | Authorization | Secrets | Audit | Sandbox
                              |
                    Lifecycle / Evolution Plane
      Discover | Verify | Install | Benchmark | Update | Rollback
```

## 3. Device Fabric is the permanent abstraction

The previous single-host concept of a Rust Device Gateway remains valid as a **local execution boundary**, but it is no longer the whole device architecture.

Locked relationship:

```text
Python Core
  -> Capability Registry
  -> Device Fabric
  -> Device Registry / Router
  -> Node Adapter / Transport
  -> Local Device Gateway
  -> Hardware / OS
```

The Core must not contain hardware-specific branches such as `if device == esp32`.

A device is represented by a stable identity and an advertised capability set.

## 4. Node types are not hardcoded

Every future physical or software-capable endpoint may participate as a NAVEEN node.

Examples are informative only, not a closed enum:

- Windows/Linux/macOS host
- Android/iOS mobile
- Raspberry Pi or other SBC
- ESP32 / MCU
- robot controller
- sensor or actuator node
- future hardware

Node classes must be extensible through registration and capability descriptors.

A tiny MCU does **not** need the full Python AI Core. It may expose secure capabilities through a lightweight node agent.

## 5. Device identity and trust are first-class

Every enrolled node has, at minimum:

```text
node_id
identity / credential
trust state
capability descriptors
permissions / policy
software or firmware version
health / resource status
last_seen / connectivity state
```

Unknown nodes are not trusted merely because they are on the same LAN.

Enrollment, authentication, authorization, revocation, rotation, and recovery are part of the Device Fabric contract.

## 6. Device capabilities are dynamic

Nodes advertise capabilities instead of forcing Core code changes.

Example:

```text
robot-07
  movement
  camera.capture
  distance.read
  battery.status
```

The Core asks for a capability. Device routing selects an eligible node.

Capability IDs are the stable interface; hardware implementations remain replaceable.

## 7. Device transports are replaceable

Transport is a provider/adapter concern, never a Core invariant.

Possible implementations include local IPC, LAN sockets, WebSocket, MQTT, BLE, USB/serial, or future relays. None is permanent.

The Device Fabric operates through transport-neutral operations such as discovery, send, receive, and event subscription.

## 8. Intelligence Fabric

NAVEEN must support multiple model providers and multiple model specializations.

The system may use different models for conversation, fast response, reasoning, coding, research, vision, speech, embeddings, classification, or other future tasks.

Model selection is a routing decision based on task requirements, capability, resource availability, privacy, latency, quality, and other policy signals.

No single LLM is the permanent brain.

## 9. Memory / Knowledge Fabric

Memory is first-class and provider-independent.

Required conceptual namespaces include working context, session, long-term personal memory, structured knowledge, projects, learning, ideas, decisions, goals, preferences, documents, and future extensible namespaces.

Retrieval is not limited to classic RAG. The target is agentic retrieval combining semantic/hybrid retrieval, knowledge graph/contextual retrieval, research, tool use, verification, and self-evaluation.

Long-term memory writes are policy-gated; not every utterance becomes permanent memory.

Future memory backends must be replaceable without rewriting Core semantics.

## 10. Capability / Plugin / Connector Fabric

All external functionality is integration-driven.

Examples include GitHub, browser automation, file systems, coding tools, n8n, note systems, communication services, cloud APIs, local applications, and future connectors.

MCP may be used as an integration protocol, but it does not bypass the Capability Registry or Security Gateway.

A plugin is an implementation behind a stable capability/connector contract, not a Core special case.

## 11. Automation is provider-based

n8n is a supported automation integration, not a permanently hardcoded Core dependency.

Automation capabilities route through an abstraction so another workflow engine can be added later without rewriting orchestration.

## 12. Lifecycle / Evolution Plane

NAVEEN is intended to remain useful for years and must therefore be designed for controlled evolution.

Potential lifecycle operations include:

```text
Discover
  -> Verify authenticity/integrity
  -> Inspect metadata and compatibility
  -> Sandbox / stage
  -> Benchmark / health-test
  -> Register
  -> Deploy / activate
  -> Observe
  -> Roll back on failure
```

This applies to providers, models, plugins, node software, and future components.

Self-update does **not** grant the AI permission to weaken or bypass its own security boundary.

## 13. Security plane remains above evolution

The Security / Permission Gateway and trusted host boundary are not replaceable by an agent decision.

The AI must not silently:

- grant itself privileges
- disable authorization
- expose secrets
- replace security policies without controlled approval
- install untrusted artifacts
- execute arbitrary unverified code

## 14. Hardware and resource independence

Resource availability is runtime data, not architecture.

The system may use CPU-only hardware, GPUs, edge nodes, cloud resources, or combinations thereof without changing Core contracts.

Resource-aware scheduling, load/unload, health, and fallback belong behind interfaces.

## 15. Changes allowed later

This architecture is locked, but it is not frozen forever.

A future change is allowed when it is deliberately designed and documented as an amendment. Adding a new provider, model family, transport, node type, plugin, storage backend, automation provider, or device does **not** require changing the locked contracts unless the capability itself is genuinely new.

Breaking an invariant requires an explicit architecture revision, not an ad-hoc implementation shortcut.

## 16. Engineering rule for all future phases

**Design against the final target first; implement incrementally behind the locked contracts. Never intentionally ship a disposable/basic architecture with the plan to rewrite its boundary later.**

Before extending a subsystem, audit existing code and classify it as:

- **KEEP** — already compatible with the lock
- **STRENGTHEN** — correct direction, missing final-grade contracts
- **REPLACE** — temporary or directly coupled implementation

No new feature should knowingly deepen a `REPLACE` category dependency.

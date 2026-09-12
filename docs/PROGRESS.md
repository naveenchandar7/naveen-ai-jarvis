# NAVEEN — Implementation Progress Log

This file tracks what has actually been done against the milestones in
`docs/NAVEEN-MASTER-ARCHITECTURE.md` and `AGENTS.md`. It is a running log,
not architecture — if it ever conflicts with those two files, they win.

---

## Reconciliation note (this session's starting point)

Before making changes, the actual uploaded repository snapshot was
re-inspected rather than trusting an earlier in-conversation audit. It had
already moved beyond that earlier audit in some ways and regressed in
others:

- **Already done, no action needed:** CSP hardened in `tauri.conf.json`
  (no longer `null`); `capabilities/default.json` narrowed to explicit
  `core:event:allow-listen` / `allow-unlisten`; system telemetry converted
  from invoke-polling to a host-emitted `host://telemetry/system` event
  (`lib.rs` background thread + `subscribeToSystemTelemetry` in
  `tauriBridge.ts`); Rust/Cargo package renamed from generic `app` /
  `app_lib` to `naveen-ai-host` / `naveen_ai_host_lib`; the dev-only
  1-4 keyboard debug shortcuts were already gated behind
  `import.meta.env.DEV`.
- **Regressed to zero:** the native mic path. `start_microphone` /
  `stop_microphone` / `get_microphone_level` commands, the `AudioState`
  managed state, and `mod audio;` had all been removed from `lib.rs`
  (though `audio.rs` itself was still sitting in the tree, orphaned and
  not compiled in). `App.tsx` no longer imported or called any mic
  function at all, and `VoicePanel` was hardcoded to `micLevel={0}`.
- **Still open, unchanged:** `visualState.ts` and `visualProfiles.ts`
  remained two separate, unmerged, per-state config modules, and
  `NovaCoreScene` still never received a `visualState` prop — so the
  neural palette never actually reacted to `assistantState`.

## M0 — Host Hardening & Debt Cleanup

**Status: core items done this session.**

1. **Unified visual-state config.** `src/config/visualState.ts` and
   `src/config/visualProfiles.ts` are merged into a single
   `src/config/assistantVisualProfiles.ts` (`AssistantState`,
   `AssistantVisualProfile`). Field names were kept distinct
   (`scene*` vs `neural*`/`activity`/`density`) rather than colliding, since
   they're genuinely two multiplier stages (outer scene scale × inner
   per-state neural profile), not duplicate data.
   `NovaCoreScene` now accepts and forwards a `visualState` prop, and
   `App.tsx` passes the live `assistantState` through — so the palette,
   activity, and density genuinely change with assistant state now, not
   just the HUD text.
   `usesSpeakingCore` is now actually consumed: `App.tsx` swaps in
   `<SpeakingCoreScene>` when the current profile sets it, instead of the
   flag sitting unread.

2. **Native mic path restored, but staying spec-compliant this time.**
   `audio.rs` (CPAL RMS capture, unchanged) is wired back into `lib.rs`
   behind `start_microphone` / `stop_microphone` commands, but the level
   readout is now a host-emitted event (`host://voice/mic-level`, ~80ms
   interval) instead of the old invoke-poll pattern — matching the
   telemetry event precedent already set in this repo. The frontend never
   auto-starts capture: `useAssistantKeyboard` gained an always-on "M" key
   (separate from the dev-only 1-4 debug keys) that calls a new
   `useNativeMicrophone` hook's `toggle()`. No PCM ever reaches the
   webview; only a float level.

3. **Not done this session, deliberately deferred:** trait-based
   `AudioDevice` / `TelemetryProvider` abstractions in Rust. The current
   code reuses the existing concrete `AudioState`/`audio::start_microphone`
   shape (previously proven working in this exact codebase) rather than
   introducing polymorphism with only one implementation to justify it.
   Revisit when a second STT/VAD backend is actually being evaluated
   (M1) — that's when the trait boundary earns its complexity.

4. **Not done this session:** granular per-command ACL scrutiny beyond
   what's already there. Custom `#[tauri::command]` functions in this app
   aren't gated by `capabilities/*.json` in Tauri v2 (that governs plugin
   commands / `core:*` APIs), so `start_microphone`/`stop_microphone`
   needed no new capability entries — confirmed by `get_system_info`
   already working the same way before this change.

## Next up: M1 — Native Voice Pipeline

PCM ring buffer → `VadProvider` (Silero candidate) → real `SttProvider`
(sherpa-onnx / whisper.cpp candidates, evaluate for Tamil/English/
Thanglish support) behind the existing `SttProvider` interface in
`src/services/voice/stt/`. `VoiceController` and `desktopVoiceEngine.ts`
already exist as the wiring target and were not touched this session
beyond what M0 required.

## 2026-09-12 — Secure host baseline review (ChatGPT)

- Reviewed the Claude-generated M0 snapshot before local adoption.
- Kept the useful unified assistant visual-profile work.
- Removed the frontend microphone lifecycle surface (`start_microphone`, `stop_microphone`, mic-level subscription hook) from the presentation layer.
- Kept host telemetry event-driven; no raw audio crosses into the webview.
- Removed the active CPAL dependency from this checkpoint because real voice capability will be introduced only behind the native Device Gateway and the future Security Gateway.
- Updated the agent rules to mark implementation as active rather than documentation-only.
- This checkpoint intentionally does **not** implement STT, VAD, Python Core, IPC, capabilities, memory, research, or TTS.
- Validation status: static/source checks completed in this environment; full TypeScript/Rust builds are blocked here because the uploaded snapshot excludes `node_modules`/Rust toolchain.

## M0 — Secure Host Baseline & Visual Debt Cleanup

**Status: completed as a candidate checkpoint.**

1. **Unified visual-state config.** `src/config/assistantVisualProfiles.ts` is
   now the single presentation profile source. `NovaCoreScene` receives the
   live `assistantState`, so palette/activity/density and scene behaviour can
   actually react to visual state. The existing `usesSpeakingCore` flag is
   also consumed by the HUD.

2. **Event-driven host telemetry.** System telemetry remains owned by Rust
   and emitted as a typed host event. React only subscribes; it does not poll
   the host in a loop.

3. **Microphone security boundary preserved.** The Claude-generated direct
   `start_microphone` / `stop_microphone` webview control surface was not
   adopted. `audio.rs` is retained as the future native CPAL adapter, but it
   is not exposed to React yet. No microphone capture is started by the HUD
   and no raw PCM reaches the webview. This keeps the current presentation
   layer inside the locked security boundary until a real Security Gateway /
   voice-session contract exists.

4. **Frontend privilege surface reduced.** `tauriBridge.ts` contains only the
   current low-risk host telemetry command/subscription surface. The old
   `useNativeMicrophone` hook and production "M" microphone toggle were
   removed. Numeric 1-4 keyboard state controls remain development-only.

5. **Heavy runtimes remain deferred.** No STT/VAD/LLM/memory stack was added
   in this checkpoint. Those components will enter only behind replaceable
   interfaces and the native security boundary.

## Next up: M1 — Security Gateway + Versioned Core Boundary

Define and implement the minimum Rust-side capability policy, request/result
contracts, authenticated Rust↔Python transport boundary, and device-gateway
interfaces needed before exposing voice, files, shell, or other privileged
capabilities. After that foundation is verified, introduce the native PCM/VAD
and replaceable STT pipeline.


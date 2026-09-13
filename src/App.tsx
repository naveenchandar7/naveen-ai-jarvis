import { useEffect, useState } from "react";

import { CommandPanel } from "./components/hud/CommandPanel";
import { DataStreamPanel } from "./components/hud/DataStreamPanel";
import { HudFrame } from "./components/hud/HudFrame";
import { NeuralActivityPanel } from "./components/hud/NeuralActivityPanel";
import { ResponsePanel } from "./components/hud/ResponsePanel";
import { SystemLogPanel } from "./components/hud/SystemLogPanel";
import { SystemPanel } from "./components/hud/SystemPanel";
import { TaskPanel } from "./components/hud/TaskPanel";
import { VoicePanel } from "./components/hud/VoicePanel";
import { WeatherPanel } from "./components/hud/WeatherPanel";
import { NovaCoreScene } from "./components/core/NovaCoreScene";
import { VirtualCursor } from "./components/interaction/VirtualCursor";
import { assistantConfig } from "./config/assistantConfig";
import {
  applyTheme,
  toggleTheme,
} from "./config/themeManager";
import { visualStateProfiles } from "./config/visualState";
import { useAssistantKeyboard } from "./hooks/useAssistantKeyboard";
import { useAssistantState } from "./hooks/useAssistantState";
import { useThemeName } from "./hooks/useTheme";
import {
  getSystemInfo,
  submitText,
  subscribeToCoreEvents,
  subscribeToSystemTelemetry,
  type CoreEvent,
  type SystemInfo,
} from "./services/native/tauriBridge";
import { setAssistantState } from "./state/assistantState";

import "./App.css";
import "./styles/command.css";
import "./styles/hud.css";
import "./styles/theme.css";

function formatUptime(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  return [
    String(hours).padStart(2, "0"),
    String(minutes).padStart(2, "0"),
    String(seconds).padStart(2, "0"),
  ].join(":");
}

function eventPayload(event: CoreEvent): Record<string, unknown> {
  return event.payload ?? {};
}

function App() {
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [coreStatus, setCoreStatus] = useState("CONNECTING");
  const [responseStatus, setResponseStatus] = useState("READY");
  const [responseMessage, setResponseMessage] = useState("Awaiting command...");
  const [responseProgress, setResponseProgress] = useState(0);

  useEffect(() => {
    let mounted = true;
    let unlistenTelemetry: (() => void) | undefined;
    let unlistenCore: (() => void) | undefined;

    const loadSystemInfo = async () => {
      try {
        const info = await getSystemInfo();
        if (mounted) {
          setSystemInfo(info);
        }
      } catch (error) {
        console.error("NAVEEN HOST SYSTEM INFO ERROR:", error);
      }
    };

    const handleCoreEvent = (event: CoreEvent) => {
      if (!mounted) {
        return;
      }

      const payload = eventPayload(event);

      if (event.event_type === "core.status") {
        const state = typeof payload.state === "string" ? payload.state : "unknown";
        setCoreStatus(state.toUpperCase());

        if (state === "processing") {
          setAssistantState("processing");
          setResponseStatus("PROCESSING");
          setResponseProgress(65);
        } else if (state === "ready") {
          setAssistantState("idle");
          setResponseStatus("READY");
          setResponseProgress(100);
        } else if (state === "authenticated" || state === "connecting") {
          setResponseStatus(state.toUpperCase());
        } else if (state === "disconnected" || state === "stopped") {
          setAssistantState("error");
          setResponseStatus("OFFLINE");
          setResponseProgress(0);
        }
        return;
      }

      if (event.event_type === "core.response") {
        setAssistantState("idle");
        setResponseStatus("NAVEEN");
        setResponseProgress(100);
        setResponseMessage(
          typeof payload.message === "string"
            ? payload.message
            : "NAVEEN completed the request.",
        );
        return;
      }

      if (event.event_type === "core.error") {
        setAssistantState("error");
        setResponseStatus("ERROR");
        setResponseProgress(0);
        setResponseMessage(
          typeof payload.message === "string"
            ? payload.message
            : "NAVEEN could not complete the request.",
        );
      }
    };

    loadSystemInfo();

    void subscribeToSystemTelemetry((info) => {
      if (mounted) {
        setSystemInfo(info);
      }
    })
      .then((unlisten) => {
        if (mounted) {
          unlistenTelemetry = unlisten;
        } else {
          void unlisten();
        }
      })
      .catch((error) => {
        console.error("NAVEEN HOST TELEMETRY ERROR:", error);
      });

    void subscribeToCoreEvents(handleCoreEvent)
      .then((unlisten) => {
        if (mounted) {
          unlistenCore = unlisten;
        } else {
          void unlisten();
        }
      })
      .catch((error) => {
        console.error("NAVEEN CORE EVENT ERROR:", error);
      });

    return () => {
      mounted = false;
      void unlistenTelemetry?.();
      void unlistenCore?.();
    };
  }, []);

  useAssistantKeyboard();

  const themeName = useThemeName();
  const assistantState = useAssistantState();
  const profile = visualStateProfiles[assistantState];

  useEffect(() => {
    applyTheme("blue");
  }, []);

  async function handleCommand(text: string) {
    setAssistantState("thinking");
    setResponseStatus("QUEUED");
    setResponseProgress(20);
    setResponseMessage("Sending your request to NAVEEN Core...");

    try {
      await submitText(text);
      setResponseStatus("PROCESSING");
    } catch (error) {
      setAssistantState("error");
      setResponseStatus("ERROR");
      setResponseProgress(0);
      setResponseMessage(
        error instanceof Error ? error.message : "NAVEEN Core is unavailable.",
      );
    }
  }

  const voiceStatus =
    assistantState === "speaking"
      ? "speaking"
      : assistantState === "thinking" || assistantState === "processing"
        ? "processing"
        : assistantState === "listening"
          ? "listening"
          : "standby";

  const voiceTranscript =
    assistantState === "thinking" || assistantState === "processing"
      ? "Processing..."
      : assistantState === "speaking"
        ? "Responding..."
        : "Say your command";

  const memoryPercent = systemInfo
    ? (systemInfo.memory_used / Math.max(systemInfo.memory_total, 1)) * 100
    : 0;

  return (
    <main className="naveen-app">
      <NovaCoreScene
        intensity={profile.intensity}
        particleDensity={profile.particleDensity}
        pulseSpeed={profile.pulseSpeed}
        rotationSpeed={profile.rotationSpeed}
      />

      <HudFrame
        title={assistantConfig.displayName}
        coreName={assistantConfig.coreName}
        coreState={assistantState.toUpperCase()}
        statusLabel="CORE STATUS"
        statusValue={coreStatus}
        themeName={themeName}
        onToggleTheme={toggleTheme}
        left={
          <>
            <SystemPanel
              cpu={systemInfo?.cpu_usage ?? 0}
              memory={memoryPercent}
              network="HOST"
              uptime={
                systemInfo ? formatUptime(systemInfo.uptime) : "00:00:00"
              }
              state={assistantState.toUpperCase()}
              vision="OFFLINE"
            />
            <DataStreamPanel />
            <NeuralActivityPanel />
            <SystemLogPanel />
            <VoicePanel
              status={voiceStatus}
              micLevel={0}
              transcript={voiceTranscript}
            />
          </>
        }
        right={
          <>
            <TaskPanel
              tasks={[
                {
                  name: "Core Connection",
                  progress:
                    coreStatus === "READY" || coreStatus === "AUTHENTICATED"
                      ? 100
                      : 35,
                },
                {
                  name: "Security Boundary",
                  progress: 100,
                },
                {
                  name: "Memory Store",
                  progress: 100,
                },
                {
                  name: "Model Provider",
                  progress: 20,
                },
              ]}
            />
            <CommandPanel
              disabled={coreStatus === "CONNECTING" || coreStatus === "OFFLINE"}
              onSubmit={handleCommand}
            />
            <WeatherPanel
              temperature="--°C"
              condition="WAITING"
              wind="-- km/h"
              humidity="--%"
            />
            <ResponsePanel
              status={responseStatus}
              message={responseMessage}
              progress={responseProgress}
            />
          </>
        }
      />

      <VirtualCursor />
    </main>
  );
}

export default App;

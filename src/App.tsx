import { useEffect, useState } from "react";

import { WeatherPanel } from "./components/hud/WeatherPanel";
import { ResponsePanel } from "./components/hud/ResponsePanel";
import { assistantConfig } from "./config/assistantConfig";

import { useAssistantState } from "./hooks/useAssistantState";
import { useAssistantKeyboard } from "./hooks/useAssistantKeyboard";

import {
  getSystemInfo,
  subscribeToSystemTelemetry,
} from "./services/native/tauriBridge";

import type { SystemInfo } from "./services/native/tauriBridge";

import {
  applyTheme,
  toggleTheme,
} from "./config/themeManager";

import { useThemeName } from "./hooks/useTheme";

import { NovaCoreScene } from "./components/core/NovaCoreScene";

import { HudFrame } from "./components/hud/HudFrame";
import { SystemPanel } from "./components/hud/SystemPanel";
import { TaskPanel } from "./components/hud/TaskPanel";
import { VoicePanel } from "./components/hud/VoicePanel";

import { DataStreamPanel } from "./components/hud/DataStreamPanel";
import { NeuralActivityPanel } from "./components/hud/NeuralActivityPanel";
import { SystemLogPanel } from "./components/hud/SystemLogPanel";

import { VirtualCursor } from "./components/interaction/VirtualCursor";

import {
  visualStateProfiles,
} from "./config/visualState";

import "./styles/theme.css";
import "./styles/hud.css";
import "./App.css";

function formatUptime(
  totalSeconds: number,
): string {
  const hours =
    Math.floor(
      totalSeconds / 3600,
    );

  const minutes =
    Math.floor(
      (totalSeconds % 3600) /
        60,
    );

  const seconds =
    totalSeconds % 60;

  return [
    String(hours).padStart(
      2,
      "0",
    ),
    String(minutes).padStart(
      2,
      "0",
    ),
    String(seconds).padStart(
      2,
      "0",
    ),
  ].join(":");
}

function App() {
  const [
    systemInfo,
    setSystemInfo,
  ] = useState<SystemInfo | null>(
    null,
  );

  useEffect(() => {
    let mounted = true;
    let unlisten:
      | (() => void)
      | undefined;

    const updateSystemInfo =
      async () => {
        try {
          const info =
            await getSystemInfo();

          if (mounted) {
            setSystemInfo(info);
          }
        } catch (error) {
          console.error(
            "NAVEEN HOST SYSTEM INFO ERROR:",
            error,
          );
        }
      };

    updateSystemInfo();

    void subscribeToSystemTelemetry(
      (info) => {
        if (mounted) {
          setSystemInfo(info);
        }
      },
    )
      .then((nextUnlisten) => {
        if (mounted) {
          unlisten = nextUnlisten;
          return;
        }

        void nextUnlisten();
      })
      .catch((error) => {
        console.error(
          "NAVEEN HOST TELEMETRY ERROR:",
          error,
        );
      });

    return () => {
      mounted = false;

      void unlisten?.();
    };
  }, []);

  useAssistantKeyboard();

  const themeName =
    useThemeName();

  useEffect(() => {
    applyTheme("blue");
  }, []);

  const assistantState =
    useAssistantState();

  const profile =
    visualStateProfiles[
      assistantState
    ];

  const voiceStatus =
    assistantState === "speaking"
      ? "speaking"
      : assistantState ===
          "thinking" ||
        assistantState ===
          "processing"
        ? "processing"
        : assistantState ===
            "listening"
          ? "listening"
          : "standby";

  const voiceTranscript =
    assistantState ===
      "thinking" ||
    assistantState ===
      "processing"
      ? "Processing..."
      : assistantState ===
          "speaking"
        ? "Responding..."
        : "Say your command";

  return (
    <main className="naveen-app">
      <NovaCoreScene
        intensity={
          profile.intensity
        }
        particleDensity={
          profile.particleDensity
        }
        pulseSpeed={
          profile.pulseSpeed
        }
        rotationSpeed={
          profile.rotationSpeed
        }
      />

      <HudFrame
        title={
          assistantConfig.displayName
        }
        coreName={
          assistantConfig.coreName
        }
        coreState={
          assistantState.toUpperCase()
        }
        statusLabel="ACTIVE MODE"
        statusValue={
          assistantState.toUpperCase()
        }
        themeName={themeName}
        onToggleTheme={
          toggleTheme
        }

        left={
          <>
            <SystemPanel
              cpu={
                systemInfo?.cpu_usage ??
                0
              }

              memory={
                systemInfo
                  ? (
                      systemInfo.memory_used /
                      systemInfo.memory_total
                    ) * 100
                  : 0
              }

              network="0 Mbps"

              uptime={
                systemInfo
                  ? formatUptime(
                      systemInfo.uptime,
                    )
                  : "00:00:00"
              }

              state={
                assistantState.toUpperCase()
              }

              vision="OFFLINE"
            />

            <DataStreamPanel />

            <NeuralActivityPanel />

            <SystemLogPanel />

            <VoicePanel
              status={
                voiceStatus
              }
              micLevel={0}
              transcript={
                voiceTranscript
              }
            />
          </>
        }

        right={
          <>
            <TaskPanel
              tasks={[
                {
                  name:
                    "Data Analysis",
                  progress: 72,
                },
                {
                  name:
                    "Voice Recognition",
                  progress: 91,
                },
                {
                  name:
                    "Image Processing",
                  progress: 63,
                },
                {
                  name:
                    "Natural Language",
                  progress: 87,
                },
              ]}
            />

            <WeatherPanel
              temperature="--°C"
              condition="WAITING"
              wind="-- km/h"
              humidity="--%"
            />

            <ResponsePanel
              status="READY"
              message="Awaiting command..."
              progress={0}
            />
          </>
        }
      />

      <VirtualCursor />
    </main>
  );
}

export default App;

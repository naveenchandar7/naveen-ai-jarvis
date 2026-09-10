import { useEffect, useState } from "react";

import { WeatherPanel } from "./components/hud/WeatherPanel";
import { ResponsePanel } from "./components/hud/ResponsePanel";
import { assistantConfig } from "./config/assistantConfig";

import { useAssistantState } from "./hooks/useAssistantState";
import { useAssistantKeyboard } from "./hooks/useAssistantKeyboard";

import {
  pingJarvisCore,
  getSystemInfo,
  startMicrophone,
  stopMicrophone,
  getMicrophoneLevel,
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

  const [
    micLevel,
    setMicLevel,
  ] = useState(0);

  useEffect(() => {
    let mounted = true;

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
            "JARVIS SYSTEM INFO ERROR:",
            error,
          );
        }
      };

    const updateMicLevel =
      async () => {
        try {
          const level =
            await getMicrophoneLevel();

          if (mounted) {
            setMicLevel(level);
          }
        } catch {
          /*
           * Mic may briefly disappear during
           * development hot reload. Don't spam
           * the console every polling cycle.
           */
        }
      };

    pingJarvisCore()
      .then((response) => {
        console.log(
          "JARVIS CORE:",
          response,
        );
      })
      .catch((error) => {
        console.error(
          "JARVIS CORE ERROR:",
          error,
        );
      });

    updateSystemInfo();
    updateMicLevel();

    const systemInterval =
      window.setInterval(
        updateSystemInfo,
        2000,
      );

    /*
     * 120ms is enough for a responsive
     * voice meter without hammering the
     * Tauri bridge every animation frame.
     */
    const microphoneInterval =
      window.setInterval(
        updateMicLevel,
        120,
      );

    startMicrophone()
      .then((response) => {
        console.log(
          "JARVIS MIC:",
          response,
        );
      })
      .catch((error) => {
        console.error(
          "JARVIS MIC ERROR:",
          error,
        );
      });

    return () => {
      mounted = false;

      window.clearInterval(
        systemInterval,
      );

      window.clearInterval(
        microphoneInterval,
      );

      void stopMicrophone()
        .catch(() => {
          // Ignore cleanup errors.
        });
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

  const micIsActive =
    micLevel > 0.028;

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
          : micIsActive
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
        : micIsActive
          ? "Audio detected..."
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
              micLevel={
                micLevel
              }
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
import {
  useEffect,
  useRef,
  useState,
} from "react";
import { GlassPanel } from "./GlassPanel";

type VoiceStatus =
  | "standby"
  | "listening"
  | "processing"
  | "speaking";

interface VoicePanelProps {
  status?: VoiceStatus;
  transcript?: string;
  micLevel?: number;
}

const statusLabels: Record<
  VoiceStatus,
  string
> = {
  standby: "STANDBY",
  listening: "LISTENING...",
  processing: "PROCESSING...",
  speaking: "SPEAKING...",
};

const BAR_COUNT = 38;

/*
 * Small input changes should not immediately make
 * the waveform jump around.
 *
 * We use a noise gate + smoothing so:
 * - quiet room => mostly flat
 * - voice => visible movement
 * - sudden spikes => softened
 */
const NOISE_GATE = 0.028;
const DISPLAY_GAIN = 10;

export function VoicePanel({
  status = "standby",
  transcript = "Say your command",
  micLevel = 0,
}: VoicePanelProps) {
  const targetLevelRef =
    useRef(0);

  const smoothedLevelRef =
    useRef(0);

  const frameRef =
    useRef<number | null>(null);

  const [displayLevel, setDisplayLevel] =
    useState(0);

  useEffect(() => {
    const gatedLevel =
      micLevel <= NOISE_GATE
        ? 0
        : Math.min(
            1,
            (micLevel - NOISE_GATE) *
              DISPLAY_GAIN,
          );

    targetLevelRef.current =
      gatedLevel;
  }, [micLevel]);

  useEffect(() => {
    const animate = () => {
      const target =
        targetLevelRef.current;

      const current =
        smoothedLevelRef.current;

      const smoothing =
        target > current
          ? 0.22
          : 0.10;

      const next =
        current +
        (target - current) *
          smoothing;

      smoothedLevelRef.current =
        next;

      setDisplayLevel(next);

      frameRef.current =
        window.requestAnimationFrame(
          animate,
        );
    };

    frameRef.current =
      window.requestAnimationFrame(
        animate,
      );

    return () => {
      if (
        frameRef.current !== null
      ) {
        window.cancelAnimationFrame(
          frameRef.current,
        );
      }
    };
  }, []);

  const voiceActive =
    displayLevel > 0.02;

  const effectiveStatus =
    status === "standby" &&
    voiceActive
      ? "listening"
      : status;

  const effectiveTranscript =
    status === "standby" &&
    voiceActive
      ? "Audio detected..."
      : transcript;

  const wave =
    Array.from(
      { length: BAR_COUNT },
      (_, index) => {
        const center =
          Math.abs(
            index -
              (BAR_COUNT - 1) / 2,
          );

        const normalized =
          center /
          ((BAR_COUNT - 1) / 2);

        const profile =
          1 -
          normalized * 0.52;

        /*
         * Deterministic movement instead of
         * Math.random(), which keeps the waveform
         * much more stable and cheaper to render.
         */
        const phase =
          index * 0.42;

        const motion =
          0.78 +
          Math.sin(
            performance.now() *
              0.004 +
              phase,
          ) *
            0.16;

        const quietBase =
          0.07;

        const value =
          quietBase +
          displayLevel *
            profile *
            motion;

        return Math.min(
          1,
          Math.max(
            quietBase,
            value,
          ),
        );
      },
    );

  return (
    <GlassPanel
      title="VOICE COMMAND"
      eyebrow="AUDIO INTERFACE"
    >
      <div className="hud-voice">
        <div className="hud-voice-wave">
          {wave.map(
            (value, index) => (
              <span
                key={index}
                style={{
                  height: `${Math.max(
                    4,
                    value * 42,
                  )}px`,
                }}
              />
            ),
          )}
        </div>

        <div className="hud-voice-line" />

        <div className="hud-voice-status">
          <span
            className={
              effectiveStatus ===
                "standby"
                ? "hud-voice-dot"
                : "hud-voice-dot hud-voice-dot--active"
            }
          />

          <span>
            {
              statusLabels[
                effectiveStatus
              ]
            }
          </span>
        </div>

        <div className="hud-voice-transcript">
          {effectiveTranscript}
        </div>
      </div>
    </GlassPanel>
  );
}
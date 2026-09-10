import { GlassPanel } from "./GlassPanel";

const logs = [
  ["CORE ONLINE", "23:12:08"],
  ["NEURAL SYNC", "23:12:11"],
  ["STATE READY", "23:12:14"],
  ["VOICE STANDBY", "23:12:18"],
];

export function SystemLogPanel() {
  return (
    <GlassPanel
      title="SYSTEM LOG"
      eyebrow="EVENT STREAM"
    >
      <div className="hud-system-log">
        {logs.map(
          ([message, time]) => (
            <div
              className="hud-log-row"
              key={`${message}-${time}`}
            >
              <span className="hud-log-bullet">
                •
              </span>

              <span className="hud-log-message">
                {message}
              </span>

              <time>
                {time}
              </time>
            </div>
          ),
        )}
      </div>
    </GlassPanel>
  );
}
import { GlassPanel } from "./GlassPanel";

interface SystemPanelProps {
  cpu?: number;
  memory?: number;
  network?: string;
  uptime?: string;
  state?: string;
  vision?: string;
}

export function SystemPanel({
  cpu = 0,
  memory = 0,
  network = "0 Mbps",
  uptime = "00:00:00",
  state = "IDLE",
  vision = "OFFLINE",
}: SystemPanelProps) {
  return (
    <GlassPanel
      title="SYSTEM OVERVIEW"
      eyebrow="SYSTEM / TELEMETRY"
    >
      <div className="hud-metrics">
        <MetricRow
          label="CPU USAGE"
          value={`${cpu}%`}
          progress={cpu}
        />

        <MetricRow
          label="MEMORY"
          value={`${memory}%`}
          progress={memory}
        />

        <MetricRow
          label="NETWORK"
          value={network}
        />

        <MetricRow
          label="UPTIME"
          value={uptime}
        />

        <MetricRow
          label="STATE"
          value={state}
          accent
        />

        <MetricRow
          label="VISION"
          value={vision}
        />
      </div>
    </GlassPanel>
  );
}

function MetricRow({
  label,
  value,
  progress,
  accent = false,
}: {
  label: string;
  value: string;
  progress?: number;
  accent?: boolean;
}) {
  return (
    <div className="hud-metric">
      <div className="hud-metric-top">
        <span className="hud-metric-label">
          {label}
        </span>

        <span
          className={
            accent
              ? "hud-metric-value hud-metric-value--active"
              : "hud-metric-value"
          }
        >
          {value}
        </span>
      </div>

      {progress !== undefined && (
        <div className="hud-metric-track">
          <span
            style={{
              width: `${Math.max(
                0,
                Math.min(100, progress),
              )}%`,
            }}
          />
        </div>
      )}
    </div>
  );
}
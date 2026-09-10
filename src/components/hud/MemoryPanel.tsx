import { GlassPanel } from "./GlassPanel";

interface MemoryPanelProps {
  title?: string;
  contextUsed: number;
  entries: number;
  className?: string;
}

export function MemoryPanel({
  title = "MEMORY",
  contextUsed,
  entries,
  className,
}: MemoryPanelProps) {
  const percent = Math.round(Math.min(1, Math.max(0, contextUsed)) * 100);

  return (
    <GlassPanel className={["hud-panel", className].filter(Boolean).join(" ")}>
      <div className="hud-panel-title">{title}</div>

      <div className="hud-memory-bar-track">
        <div className="hud-memory-bar-fill" style={{ width: `${percent}%` }} />
      </div>

      <div className="hud-panel-rows">
        <div className="hud-panel-row">
          <span className="hud-panel-row-label">CONTEXT</span>
          <span className="hud-panel-row-value hud-panel-row-value--default">{percent}%</span>
        </div>
        <div className="hud-panel-row">
          <span className="hud-panel-row-label">ENTRIES</span>
          <span className="hud-panel-row-value hud-panel-row-value--default">{entries}</span>
        </div>
      </div>
    </GlassPanel>
  );
}

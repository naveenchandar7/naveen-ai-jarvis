import { GlassPanel } from "./GlassPanel";

interface ResponsePanelProps {
  message?: string;
  progress?: number;
  status?: string;
}

export function ResponsePanel({
  message = "Processing command...",
  progress = 0,
  status = "READY",
}: ResponsePanelProps) {
  return (
    <GlassPanel
      title="AI RESPONSE"
      eyebrow="RESPONSE CHANNEL"
    >
      <div className="hud-response">
        <div className="hud-response-status">
          <span />
          {status}
        </div>

        <div className="hud-response-message">
          {message}
        </div>

        <div className="hud-response-track">
          <span
            style={{
              width: `${Math.max(
                0,
                Math.min(100, progress),
              )}%`,
            }}
          />
        </div>
      </div>
    </GlassPanel>
  );
}

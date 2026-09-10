import { GlassPanel } from "./GlassPanel";

export function NeuralActivityPanel() {
  return (
    <GlassPanel
      title="NEURAL ACTIVITY"
      eyebrow="COGNITIVE CORE"
    >
      <div className="hud-neural-activity">
        <div className="hud-neural-mini-orb">
          <span />
          <span />
          <span />
        </div>

        <div className="hud-neural-stats">
          <span className="hud-neural-label">
            SYNAPSES ACTIVE
          </span>

          <strong>98.7%</strong>

          <span className="hud-neural-label">
            LEARNING RATE
          </span>

          <strong>
            0.045
          </strong>
        </div>
      </div>
    </GlassPanel>
  );
}
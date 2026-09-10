import { useEffect, useState } from "react";
import { GlassPanel } from "./GlassPanel";

export function DataStreamPanel() {
  const [values, setValues] = useState(() =>
    createWave(),
  );

  useEffect(() => {
    const timer = window.setInterval(() => {
      setValues(createWave());
    }, 120);

    return () => {
      window.clearInterval(timer);
    };
  }, []);

  return (
    <GlassPanel
      title="DATA STREAM"
      eyebrow="NETWORK / SIGNAL"
    >
      <div className="hud-data-stream">
        <div className="hud-data-chart">
          {values.map((value, index) => (
            <span
              key={index}
              style={{
                height: `${8 + value * 34}px`,
              }}
            />
          ))}
        </div>

        <div className="hud-data-divider" />

        <div className="hud-data-row">
          <span>PACKETS</span>
          <strong>2.6M/s</strong>
        </div>

        <div className="hud-data-row">
          <span>LATENCY</span>
          <strong>18ms</strong>
        </div>
      </div>
    </GlassPanel>
  );
}

function createWave() {
  return Array.from(
    { length: 34 },
    (_, index) =>
      0.18 +
      Math.abs(
        Math.sin(
          index * 0.75 +
            Date.now() * 0.002,
        ),
      ) *
        (0.35 +
          Math.random() * 0.65),
  );
}
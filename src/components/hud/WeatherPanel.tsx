import { GlassPanel } from "./GlassPanel";

interface WeatherPanelProps {
  temperature?: string;
  condition?: string;
  wind?: string;
  humidity?: string;
}

export function WeatherPanel({
  temperature = "--°C",
  condition = "UNAVAILABLE",
  wind = "-- km/h",
  humidity = "--%",
}: WeatherPanelProps) {
  return (
    <GlassPanel
      title="WEATHER UPDATE"
      eyebrow="ENVIRONMENT"
    >
      <div className="hud-weather">
        <div className="hud-weather-main">
          <div className="hud-weather-icon">
            <span />
            <span />
          </div>

          <strong>
            {temperature}
          </strong>

          <small>
            {condition}
          </small>
        </div>

        <div className="hud-weather-meta">
          <div>
            <span>WIND</span>
            <strong>{wind}</strong>
          </div>

          <div>
            <span>HUMIDITY</span>
            <strong>{humidity}</strong>
          </div>
        </div>
      </div>
    </GlassPanel>
  );
}
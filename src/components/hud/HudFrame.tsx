import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import type { ThemeName } from "../../config/theme";

interface HudFrameProps {
  title: string;
  coreName: string;
  coreState: string;
  statusLabel: string;
  statusValue: string;
  themeName: ThemeName;
  onToggleTheme: () => void;
  left?: ReactNode;
  right?: ReactNode;
}

export function HudFrame({
  title,
  coreName,
  coreState,
  statusLabel,
  statusValue,
  themeName,
  onToggleTheme,
  left,
  right,
}: HudFrameProps) {
  const [now, setNow] =
    useState(() => new Date());

  const [online, setOnline] =
    useState(() => navigator.onLine);

  useEffect(() => {
    const timer =
      window.setInterval(() => {
        setNow(new Date());
      }, 1000);

    const handleOnline = () => {
      setOnline(true);
    };

    const handleOffline = () => {
      setOnline(false);
    };

    window.addEventListener(
      "online",
      handleOnline,
    );

    window.addEventListener(
      "offline",
      handleOffline,
    );

    return () => {
      window.clearInterval(timer);

      window.removeEventListener(
        "online",
        handleOnline,
      );

      window.removeEventListener(
        "offline",
        handleOffline,
      );
    };
  }, []);

  const time =
    now.toLocaleTimeString(
      "en-GB",
      {
        hour12: false,
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      },
    );

  const isBlue =
    themeName === "blue";

  return (
    <div className="hud-root">

      <div
        className="hud-border"
        aria-hidden="true"
      >
        <span className="hud-border-corner hud-border-corner--tl" />
        <span className="hud-border-corner hud-border-corner--tr" />
        <span className="hud-border-corner hud-border-corner--bl" />
        <span className="hud-border-corner hud-border-corner--br" />

        <span className="hud-border-notch hud-border-notch--tl" />
        <span className="hud-border-notch hud-border-notch--tr" />
        <span className="hud-border-notch hud-border-notch--bl" />
        <span className="hud-border-notch hud-border-notch--br" />
      </div>

      <header className="hud-title-rail">
        <span className="hud-title-bracket hud-title-bracket--left" />

        <div className="hud-title-main">
          <span className="hud-title-name">
            {title}
          </span>

          <span className="hud-title-divider">
            |
          </span>

          <span className="hud-title-subtitle">
            PERSONAL ASSISTANT AI
          </span>
        </div>

        <span className="hud-title-bracket hud-title-bracket--right" />
      </header>

      <div className="hud-top-right">
        <span className="hud-system-status">
          <span className="hud-status-dot" />

          {online
            ? "ONLINE"
            : "OFFLINE"}
        </span>

        <span className="hud-time">
          {time}
        </span>

        <span className="hud-top-icon">
          ◌
        </span>

        <span className="hud-top-icon">
          ◇
        </span>

        <span className="hud-top-icon">
          ○
        </span>
      </div>

      <button
        type="button"
        className="hud-theme"
        onClick={onToggleTheme}
        aria-label="Change theme"
      >
        <span
          className={
            isBlue
              ? "hud-theme-orb hud-theme-orb--blue"
              : "hud-theme-orb hud-theme-orb--orange"
          }
        />

        <span className="hud-theme-text">
          {isBlue
            ? "BLUE"
            : "ORANGE"}
        </span>
      </button>

      <div className="hud-active">
        <div className="hud-active-label">
          {statusLabel}
        </div>

        <div className="hud-active-wave">
          {Array.from(
            { length: 34 },
            (_, index) => (
              <span
                key={index}
                style={{
                  animationDelay:
                    `${index * 30}ms`,
                }}
              />
            ),
          )}
        </div>
      </div>

      <div className="hud-core-state">
        <span />

        <b>
          {statusValue}
        </b>

        <span />
      </div>

      <div className="hud-core-name">
        <strong>
          {coreName}
        </strong>

        <small>
          {coreState}
        </small>
      </div>

      {left && (
        <aside className="hud-left">
          {left}
        </aside>
      )}

      {right && (
        <aside className="hud-right">
          {right}
        </aside>
      )}

      <div className="hud-process">
        <span>THINKING</span>
        <i>•</i>
        <span>ANALYZING</span>
        <i>•</i>
        <span>RESPONDING</span>

        <div className="hud-process-track">
          <span />
        </div>

        <b>83%</b>
      </div>

      <footer className="hud-footer">
        <span>VISION</span>
        <span>VOICE</span>
        <span>NEURAL SYSTEM</span>
      </footer>

    </div>
  );
}
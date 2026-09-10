import type { ReactNode } from "react";

interface GlassPanelProps {
  title?: string;
  eyebrow?: string;
  children: ReactNode;
  className?: string;
  accent?: "primary" | "secondary";
}

export function GlassPanel({
  title,
  eyebrow,
  children,
  className = "",
  accent = "primary",
}: GlassPanelProps) {
  return (
    <section
      className={[
        "hud-glass",
        `hud-glass--${accent}`,
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <span className="hud-glass-corner hud-glass-corner--tl" />
      <span className="hud-glass-corner hud-glass-corner--tr" />
      <span className="hud-glass-corner hud-glass-corner--bl" />
      <span className="hud-glass-corner hud-glass-corner--br" />

      <div className="hud-glass-topline" />

      {(title || eyebrow) && (
        <>
          <header className="hud-glass-header">
            <div>
              {eyebrow && (
                <div className="hud-glass-eyebrow">
                  {eyebrow}
                </div>
              )}

              {title && (
                <h2 className="hud-glass-title">
                  {title}
                </h2>
              )}
            </div>

            <span className="hud-glass-indicator" />
          </header>

          <div className="hud-glass-rule" />
        </>
      )}

      <div className="hud-glass-body">
        {children}
      </div>
    </section>
  );
}
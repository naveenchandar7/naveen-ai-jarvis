import { GlassPanel } from "./GlassPanel";

interface ProjectPanelProps {
  title?: string;
  projectName: string;
  phase: string;
  className?: string;
}

export function ProjectPanel({
  title = "PROJECT",
  projectName,
  phase,
  className,
}: ProjectPanelProps) {
  return (
    <GlassPanel className={["hud-panel", className].filter(Boolean).join(" ")}>
      <div className="hud-panel-title">{title}</div>

      <div className="hud-project-name">{projectName}</div>
      <div className="hud-project-phase">{phase}</div>
    </GlassPanel>
  );
}

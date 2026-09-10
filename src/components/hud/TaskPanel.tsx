import { GlassPanel } from "./GlassPanel";

interface Task {
  name: string;
  progress: number;
}

interface TaskPanelProps {
  tasks?: Task[];
}

const defaultTasks: Task[] = [
  {
    name: "Data Analysis",
    progress: 72,
  },
  {
    name: "Voice Recognition",
    progress: 91,
  },
  {
    name: "Image Processing",
    progress: 63,
  },
  {
    name: "Natural Language",
    progress: 87,
  },
];

export function TaskPanel({
  tasks = defaultTasks,
}: TaskPanelProps) {
  return (
    <GlassPanel
      title="TASK MANAGER"
      eyebrow="ACTIVE PROCESSES"
    >
      <div className="hud-task-list">
        {tasks.map((task) => (
          <div
            className="hud-task"
            key={task.name}
          >
            <div className="hud-task-head">
              <span>
                {task.name}
              </span>

              <strong>
                {task.progress}%
              </strong>
            </div>

            <div className="hud-task-track">
              <span
                style={{
                  width: `${Math.max(
                    0,
                    Math.min(
                      100,
                      task.progress,
                    ),
                  )}%`,
                }}
              />
            </div>
          </div>
        ))}
      </div>
    </GlassPanel>
  );
}
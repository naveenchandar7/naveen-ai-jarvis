import { useState } from "react";
import { GlassPanel } from "./GlassPanel";

interface CommandPanelProps {
  disabled?: boolean;
  onSubmit: (text: string) => Promise<void> | void;
}

export function CommandPanel({
  disabled = false,
  onSubmit,
}: CommandPanelProps) {
  const [value, setValue] = useState("");
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit() {
    const text = value.trim();
    if (!text || disabled || submitting) {
      return;
    }

    setSubmitting(true);
    try {
      await onSubmit(text);
      setValue("");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <GlassPanel title="COMMAND" eyebrow="NAVEEN INPUT">
      <div className="naveen-command">
        <textarea
          value={value}
          disabled={disabled || submitting}
          placeholder={
            disabled
              ? "Waiting for Core connection..."
              : "Ask NAVEEN anything..."
          }
          rows={3}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
              event.preventDefault();
              void handleSubmit();
            }
          }}
        />
        <div className="naveen-command-footer">
          <span>Ctrl / ⌘ + Enter</span>
          <button
            type="button"
            disabled={disabled || submitting || !value.trim()}
            onClick={() => void handleSubmit()}
          >
            {submitting ? "SENDING..." : "SEND"}
          </button>
        </div>
      </div>
    </GlassPanel>
  );
}

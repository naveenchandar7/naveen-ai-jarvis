import { useEffect } from "react";

import {
  setAssistantState,
  type AssistantState,
} from "../state/assistantState";

const KEY_STATE_MAP: Record<
  string,
  AssistantState
> = {
  "1": "idle",
  "2": "listening",
  "3": "thinking",
  "4": "speaking",
};

/**
 * Development-only visual state controls. Privileged/native actions must not
 * originate from keyboard/UI handlers. Production voice lifecycle will be
 * driven by the native host/core pipeline through explicit capability paths.
 */
export function useAssistantKeyboard() {
  useEffect(() => {
    if (!import.meta.env.DEV) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null;

      const isTyping =
        target?.tagName === "INPUT" ||
        target?.tagName === "TEXTAREA" ||
        target?.isContentEditable;

      if (isTyping) {
        return;
      }

      const nextState =
        KEY_STATE_MAP[event.key];

      if (!nextState) {
        return;
      }

      setAssistantState(nextState);
    }

    window.addEventListener(
      "keydown",
      handleKeyDown,
    );

    return () => {
      window.removeEventListener(
        "keydown",
        handleKeyDown,
      );
    };
  }, []);
}

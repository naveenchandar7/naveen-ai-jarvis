import { useSyncExternalStore } from "react";

import {
  getAssistantState,
  subscribeAssistantState,
} from "../state/assistantState";

export function useAssistantState() {
  return useSyncExternalStore(
    subscribeAssistantState,
    getAssistantState,
    getAssistantState,
  );
}
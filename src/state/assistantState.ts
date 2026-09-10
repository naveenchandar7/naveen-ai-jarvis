import {
  visualStateProfiles,
} from "../config/visualState";

export type AssistantState =
  keyof typeof visualStateProfiles;

type StateListener = () => void;

let currentState: AssistantState = "idle";

const listeners = new Set<StateListener>();

export function getAssistantState(): AssistantState {
  return currentState;
}

export function setAssistantState(
  state: AssistantState,
) {
  if (currentState === state) {
    return;
  }

  currentState = state;

  listeners.forEach((listener) => {
    listener();
  });
}

export function subscribeAssistantState(
  listener: StateListener,
) {
  listeners.add(listener);

  return () => {
    listeners.delete(listener);
  };
}
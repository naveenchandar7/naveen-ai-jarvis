import {
  setAssistantState,
} from "../../state/assistantState";

import type {
  VoiceResult,
} from "../voiceTypes";

import {
  createDesktopVoiceEngine,
} from "./desktopVoiceEngine";

export class VoiceController {
  private readonly engine =
    createDesktopVoiceEngine();

  private transcript = "";

  constructor() {
    this.engine.configure({
      onListening: () => {
        setAssistantState(
          "listening",
        );
      },

      onStopped: () => {
        setAssistantState(
          "idle",
        );
      },

      onTranscript: (
        result: VoiceResult,
      ) => {
        this.transcript =
          result.text;

        if (result.final) {
          setAssistantState(
            "thinking",
          );
        }
      },

      onError: (
        message: string,
      ) => {
        console.error(
          "[VOICE]",
          message,
        );

        setAssistantState(
          "idle",
        );
      },
    });
  }

  startListening() {
    this.engine.start();
  }

  stopListening() {
    this.engine.stop();
  }

  abortListening() {
    this.engine.abort();
  }

  getTranscript() {
    return this.transcript;
  }
}
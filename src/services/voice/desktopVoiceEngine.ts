import type {
  VoiceResult,
} from "../voiceTypes";

import type {
  DesktopVoiceEngine,
  VoiceEngineEvents,
} from "./voiceEngine";

import {
  createSttProvider,
} from "./stt/sttProvider";

export function createDesktopVoiceEngine(): DesktopVoiceEngine {
  let events: VoiceEngineEvents = {};

  let running = false;

  const stt =
    createSttProvider();

  stt.onResult = (
    result: VoiceResult,
  ) => {
    events.onTranscript?.(
      result,
    );
  };

  stt.onError = (
    message: string,
  ) => {
    events.onError?.(
      message,
    );
  };

  const engine: DesktopVoiceEngine = {
    configure(nextEvents) {
      events = nextEvents;
    },

    async start() {
      if (running) {
        return;
      }

      try {
        await stt.start();

        running = true;

        events.onListening?.();
      } catch (error) {
        const message =
          error instanceof Error
            ? error.message
            : String(error);

        events.onError?.(
          message,
        );
      }
    },

    async stop() {
      if (!running) {
        return;
      }

      try {
        await stt.stop();
      } finally {
        running = false;

        events.onStopped?.();
      }
    },

    async abort() {
      try {
        await stt.abort();
      } finally {
        running = false;

        events.onStopped?.();
      }
    },
  };

  return engine;
}
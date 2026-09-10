import type { SttProvider } from "./sttTypes";

/**
 * Provider-independent STT factory.
 *
 * The actual speech-to-text implementation will be
 * plugged in here without changing the voice engine.
 */
export function createSttProvider(): SttProvider {
  let running = false;

  return {
    async start() {
      if (running) {
        return;
      }

      running = true;

      /*
       * Actual STT provider will be connected here.
       *
       * IMPORTANT:
       * No browser SpeechRecognition.
       * No provider-specific code in DesktopVoiceEngine.
       */
    },

    async stop() {
      running = false;
    },

    async abort() {
      running = false;
    },
  };
}
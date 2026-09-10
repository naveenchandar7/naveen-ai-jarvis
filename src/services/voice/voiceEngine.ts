import type {
  VoiceEngine,
  VoiceResult,
} from "../voiceTypes";

export interface VoiceEngineEvents {
  onListening?: () => void;

  onStopped?: () => void;

  onTranscript?: (
    result: VoiceResult,
  ) => void;

  onError?: (
    message: string,
  ) => void;
}

export interface DesktopVoiceEngine
  extends VoiceEngine {
  configure(
    events: VoiceEngineEvents,
  ): void;
}
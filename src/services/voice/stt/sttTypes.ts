import type { VoiceResult } from "../../voiceTypes";

export interface SttProvider {
  start(): Promise<void>;

  stop(): Promise<void>;

  abort(): Promise<void>;

  onResult?: (result: VoiceResult) => void;

  onError?: (error: string) => void;
}
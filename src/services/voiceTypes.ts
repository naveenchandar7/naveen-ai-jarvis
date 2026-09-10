export type VoiceStatus =
  | "idle"
  | "listening"
  | "processing"
  | "speaking";

export interface VoiceResult {
  text: string;
  final: boolean;
}

export interface VoiceEngine {
  start(): Promise<void>;
  stop(): Promise<void>;
  abort(): Promise<void>;

  onStart?: () => void;
  onEnd?: () => void;
  onResult?: (
    result: VoiceResult,
  ) => void;
  onError?: (
    error: string,
  ) => void;
}
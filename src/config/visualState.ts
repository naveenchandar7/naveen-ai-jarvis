export type AIVisualState =
  | "idle"
  | "thinking"
  | "listening"
  | "speaking"
  | "processing"
  | "error";

export interface CoreVisualProfile {
  state: AIVisualState;
  intensity: number;
  particleDensity: number;
  orbitDensity: number;
  pulseSpeed: number;
  rotationSpeed: number;
  /**
   * When true, the HUD/App layer should mount the separate SpeakingCore
   * (src/components/speaking) instead of — or layered on top of — NovaCore.
   * NovaCore itself never changes shape for this; see section 9 of the spec.
   */
  usesSpeakingCore: boolean;
}

export const visualStateProfiles: Record<AIVisualState, CoreVisualProfile> = {
  idle: {
    state: "idle",
    intensity: 0.75,
    particleDensity: 0.8,
    orbitDensity: 0.7,
    pulseSpeed: 0.8,
    rotationSpeed: 0.7,
    usesSpeakingCore: false,
  },

  thinking: {
    state: "thinking",
    intensity: 1.05,
    particleDensity: 1.0,
    orbitDensity: 1.0,
    pulseSpeed: 1.4,
    rotationSpeed: 1.3,
    usesSpeakingCore: false,
  },

  listening: {
    state: "listening",
    intensity: 0.9,
    particleDensity: 0.9,
    orbitDensity: 0.85,
    pulseSpeed: 1.1,
    rotationSpeed: 1.0,
    usesSpeakingCore: false,
  },

  speaking: {
    state: "speaking",
    intensity: 1.15,
    particleDensity: 1.0,
    orbitDensity: 1.0,
    pulseSpeed: 1.6,
    rotationSpeed: 1.2,
    usesSpeakingCore: true,
  },

  processing: {
    state: "processing",
    intensity: 1.0,
    particleDensity: 1.0,
    orbitDensity: 1.15,
    pulseSpeed: 1.5,
    rotationSpeed: 1.4,
    usesSpeakingCore: false,
  },

  error: {
    state: "error",
    intensity: 1.2,
    particleDensity: 0.6,
    orbitDensity: 0.4,
    pulseSpeed: 0.5,
    rotationSpeed: 0.3,
    usesSpeakingCore: false,
  },
};

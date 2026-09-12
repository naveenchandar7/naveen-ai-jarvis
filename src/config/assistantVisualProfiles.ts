/**
 * Single source of truth for "what does NAVEEN look like in state X".
 *
 * This replaces the previous split between `visualState.ts` (scene-level
 * multipliers consumed by <NovaCoreScene>) and `visualProfiles.ts`
 * (neural-field multipliers + palette consumed inside <NovaCore>). Both
 * files were keyed by the same conceptual assistant state and were never
 * merged, which meant the palette/activity side never actually reacted to
 * `assistantState` changes from the HUD (NovaCoreScene never received a
 * `visualState` prop). Keeping one profile per state removes that class of
 * drift entirely.
 *
 * Field naming keeps the two multiplier "stages" distinct on purpose:
 *   - scene*   → passed into <NovaCoreScene>/<NovaCore> as top-level props
 *               (intensity, particleDensity, pulseSpeed, rotationSpeed)
 *   - neural*  / activity / density → consumed *inside* <NovaCore> to scale
 *               the neural field, particles, and glow per assistant state
 *   - palette  → tints EnergySphere / NeuralPlexus per state
 *   - usesSpeakingCore → tells the HUD/App layer to mount <SpeakingCoreScene>
 *               instead of (or layered on) <NovaCoreScene>. See
 *               src/components/speaking for why that stays a separate visual.
 */

export type AssistantState =
  | "idle"
  | "listening"
  | "thinking"
  | "speaking"
  | "processing"
  | "error";

export type VisualMode = "neural" | "jarvis" | "minimal";

export interface NeuralPalette {
  primary: string;
  secondary: string;
  accent: string;
  hot: string;
}

export interface AssistantVisualProfile {
  mode: VisualMode;
  state: AssistantState;

  // ---- scene-level multipliers (NovaCoreScene props) ----
  sceneIntensity: number;
  sceneParticleDensity: number;
  scenePulseSpeed: number;
  sceneRotationSpeed: number;

  // ---- neural-field multipliers (used inside NovaCore) ----
  neuralIntensity: number;
  activity: number;
  density: number;
  neuralSpeed: number;

  palette: NeuralPalette;

  /**
   * When true, the HUD/App layer should mount the separate SpeakingCore
   * instead of — or layered on top of — NovaCore. NovaCore itself never
   * changes shape for this; see section 9 of the master architecture spec.
   */
  usesSpeakingCore: boolean;
}

const neuralBase: NeuralPalette = {
  primary: "#00e5ff",
  secondary: "#398cff",
  accent: "#7c5cff",
  hot: "#ff4fd8",
};

export const assistantVisualProfiles: Record<
  AssistantState,
  AssistantVisualProfile
> = {
  idle: {
    mode: "neural",
    state: "idle",

    sceneIntensity: 0.75,
    sceneParticleDensity: 0.8,
    scenePulseSpeed: 0.8,
    sceneRotationSpeed: 0.7,

    neuralIntensity: 0.82,
    activity: 0.32,
    density: 0.92,
    neuralSpeed: 0.72,

    palette: {
      primary: neuralBase.primary,
      secondary: neuralBase.secondary,
      accent: "#735cff",
      hot: "#ef5cff",
    },

    usesSpeakingCore: false,
  },

  listening: {
    mode: "neural",
    state: "listening",

    sceneIntensity: 0.9,
    sceneParticleDensity: 0.9,
    scenePulseSpeed: 1.1,
    sceneRotationSpeed: 1.0,

    neuralIntensity: 1.0,
    activity: 0.64,
    density: 1.0,
    neuralSpeed: 1.05,

    palette: {
      primary: "#00eaff",
      secondary: "#2f8dff",
      accent: "#7d63ff",
      hot: "#ff66d9",
    },

    usesSpeakingCore: false,
  },

  thinking: {
    mode: "neural",
    state: "thinking",

    sceneIntensity: 1.05,
    sceneParticleDensity: 1.0,
    scenePulseSpeed: 1.4,
    sceneRotationSpeed: 1.3,

    neuralIntensity: 1.12,
    activity: 0.88,
    density: 1.08,
    neuralSpeed: 1.28,

    palette: {
      primary: "#00dcff",
      secondary: "#438bff",
      accent: "#9a5cff",
      hot: "#ff43c8",
    },

    usesSpeakingCore: false,
  },

  speaking: {
    mode: "neural",
    state: "speaking",

    sceneIntensity: 1.15,
    sceneParticleDensity: 1.0,
    scenePulseSpeed: 1.6,
    sceneRotationSpeed: 1.2,

    neuralIntensity: 1.2,
    activity: 1.0,
    density: 1.14,
    neuralSpeed: 1.45,

    palette: {
      primary: "#00eaff",
      secondary: "#4e82ff",
      accent: "#ac5cff",
      hot: "#ff3fa8",
    },

    usesSpeakingCore: true,
  },

  processing: {
    mode: "neural",
    state: "processing",

    sceneIntensity: 1.0,
    sceneParticleDensity: 1.0,
    scenePulseSpeed: 1.5,
    sceneRotationSpeed: 1.4,

    neuralIntensity: 1.08,
    activity: 0.76,
    density: 1.02,
    neuralSpeed: 1.16,

    palette: {
      primary: "#12e0ff",
      secondary: "#497fff",
      accent: "#8a62ff",
      hot: "#ff57d5",
    },

    usesSpeakingCore: false,
  },

  error: {
    mode: "neural",
    state: "error",

    sceneIntensity: 1.2,
    sceneParticleDensity: 0.6,
    scenePulseSpeed: 0.5,
    sceneRotationSpeed: 0.3,

    neuralIntensity: 1.0,
    activity: 0.72,
    density: 0.92,
    neuralSpeed: 0.92,

    palette: {
      primary: "#4aa8ff",
      secondary: "#7c68ff",
      accent: "#ff4b9b",
      hot: "#ff3e65",
    },

    usesSpeakingCore: false,
  },
};

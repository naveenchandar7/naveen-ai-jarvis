export type VisualMode =
  | "neural"
  | "jarvis"
  | "minimal";

export type AssistantVisualState =
  | "idle"
  | "listening"
  | "thinking"
  | "speaking"
  | "processing"
  | "error";

export interface NeuralPalette {
  primary: string;
  secondary: string;
  accent: string;
  hot: string;
}

export interface VisualProfile {
  mode: VisualMode;
  state: AssistantVisualState;

  intensity: number;
  activity: number;
  density: number;
  speed: number;

  palette: NeuralPalette;
}

const neuralBase: NeuralPalette = {
  primary: "#00e5ff",
  secondary: "#398cff",
  accent: "#7c5cff",
  hot: "#ff4fd8",
};

export const visualProfiles: Record<
  AssistantVisualState,
  VisualProfile
> = {
  idle: {
    mode: "neural",
    state: "idle",

    intensity: 0.82,
    activity: 0.32,
    density: 0.92,
    speed: 0.72,

    palette: {
      primary: neuralBase.primary,
      secondary: neuralBase.secondary,
      accent: "#735cff",
      hot: "#ef5cff",
    },
  },

  listening: {
    mode: "neural",
    state: "listening",

    intensity: 1.0,
    activity: 0.64,
    density: 1.0,
    speed: 1.05,

    palette: {
      primary: "#00eaff",
      secondary: "#2f8dff",
      accent: "#7d63ff",
      hot: "#ff66d9",
    },
  },

  thinking: {
    mode: "neural",
    state: "thinking",

    intensity: 1.12,
    activity: 0.88,
    density: 1.08,
    speed: 1.28,

    palette: {
      primary: "#00dcff",
      secondary: "#438bff",
      accent: "#9a5cff",
      hot: "#ff43c8",
    },
  },

  speaking: {
    mode: "neural",
    state: "speaking",

    intensity: 1.2,
    activity: 1.0,
    density: 1.14,
    speed: 1.45,

    palette: {
      primary: "#00eaff",
      secondary: "#4e82ff",
      accent: "#ac5cff",
      hot: "#ff3fa8",
    },
  },

  processing: {
    mode: "neural",
    state: "processing",

    intensity: 1.08,
    activity: 0.76,
    density: 1.02,
    speed: 1.16,

    palette: {
      primary: "#12e0ff",
      secondary: "#497fff",
      accent: "#8a62ff",
      hot: "#ff57d5",
    },
  },

  error: {
    mode: "neural",
    state: "error",

    intensity: 1.0,
    activity: 0.72,
    density: 0.92,
    speed: 0.92,

    palette: {
      primary: "#4aa8ff",
      secondary: "#7c68ff",
      accent: "#ff4b9b",
      hot: "#ff3e65",
    },
  },
};
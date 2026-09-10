export type HudPanelId =
  | "system"
  | "data"
  | "neural"
  | "log"
  | "tasks"
  | "voice"
  | "weather"
  | "response";

export interface SystemMetrics {
  cpu: number;
  memory: number;
  network: string;
  uptime: string;
}

export interface DataStreamMetrics {
  packets: string;
  latency: string;
  activity: number[];
}

export interface NeuralMetrics {
  synapses: string;
  learningRate: string;
}

export interface SystemLogEntry {
  message: string;
  time: string;
}

export interface TaskMetric {
  name: string;
  progress: number;
}

export interface VoiceHudState {
  status:
    | "standby"
    | "listening"
    | "processing"
    | "speaking";
  transcript: string;
  waveform: number[];
}

export interface WeatherHudState {
  temperature: string;
  condition: string;
  wind: string;
  humidity: string;
}

export interface ResponseHudState {
  status:
    | "idle"
    | "processing"
    | "complete"
    | "error";
  message: string;
  progress: number;
}

/*
 * This is only the presentation model.
 *
 * Later these values will come from:
 * system service
 * voice service
 * memory service
 * AI service
 * weather service
 *
 * The HUD itself won't know where the data came from.
 */

export interface HudModel {
  system: SystemMetrics;
  data: DataStreamMetrics;
  neural: NeuralMetrics;
  log: SystemLogEntry[];
  tasks: TaskMetric[];
  voice: VoiceHudState;
  weather: WeatherHudState;
  response: ResponseHudState;
}

export const defaultHudModel: HudModel = {
  system: {
    cpu: 0,
    memory: 0,
    network: "0 Mbps",
    uptime: "00:00:00",
  },

  data: {
    packets: "0/s",
    latency: "0 ms",
    activity: [
      0.18,
      0.25,
      0.2,
      0.32,
      0.28,
      0.4,
      0.36,
      0.48,
      0.44,
      0.6,
      0.52,
      0.7,
    ],
  },

  neural: {
    synapses: "0%",
    learningRate: "0.000",
  },

  log: [],

  tasks: [
    {
      name: "Data Analysis",
      progress: 0,
    },
    {
      name: "Voice Recognition",
      progress: 0,
    },
    {
      name: "Image Processing",
      progress: 0,
    },
    {
      name: "Natural Language",
      progress: 0,
    },
  ],

  voice: {
    status: "standby",
    transcript: "",
    waveform: Array.from(
      { length: 32 },
      () => 0,
    ),
  },

  weather: {
    temperature: "--°C",
    condition: "Unavailable",
    wind: "-- km/h",
    humidity: "--%",
  },

  response: {
    status: "idle",
    message: "System ready.",
    progress: 0,
  },
};
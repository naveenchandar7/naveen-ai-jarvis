export type ThemeName = "blue" | "orange";

export interface ThemeTokens {
  // ---- HUD / UI tokens ----
  background: string;
  surface: string;
  surfaceStrong: string;

  primary: string;
  primaryStrong: string;
  secondary: string;
  accent: string;

  text: string;
  textMuted: string;

  border: string;
  shadow: string;

  glow: string;

  // ---- Nova Core engine tokens (consumed as JS values inside R3F, not CSS) ----
  coreColor: string;
  coreGlow: string;
  particleColor: string;
  orbitColor: string;
  nodeColor: string;

  /** Base visual intensity multiplier for this theme (bloom strength, emissive power). */
  intensity: number;
}

export const themes: Record<ThemeName, ThemeTokens> = {
  blue: {
    background: "#050816",
    surface: "rgba(10, 18, 40, 0.72)",
    surfaceStrong: "rgba(14, 28, 58, 0.92)",

    primary: "#00d9ff",
    primaryStrong: "#008cff",
    secondary: "#5b7cff",
    accent: "#8be9ff",

    text: "#f4fbff",
    textMuted: "#8fa7bf",

    border: "rgba(0, 217, 255, 0.28)",
    shadow: "rgba(0, 217, 255, 0.18)",

    glow: "0 0 32px rgba(0, 217, 255, 0.28)",

    coreColor: "#00e5ff",
    coreGlow: "#00e5ff",
    particleColor: "#00d0ff",
    orbitColor: "#00d0ff",
    nodeColor: "#ffffff",

    intensity: 1,
  },

  orange: {
    background: "#0b0602",
    surface: "rgba(38, 18, 6, 0.74)",
    surfaceStrong: "rgba(55, 24, 7, 0.92)",

    primary: "#ff8a00",
    primaryStrong: "#ff5a00",
    secondary: "#ffb347",
    accent: "#ffd08a",

    text: "#fff8ef",
    textMuted: "#bba58e",

    border: "rgba(255, 138, 0, 0.28)",
    shadow: "rgba(255, 106, 0, 0.18)",

    glow: "0 0 32px rgba(255, 106, 0, 0.28)",

    coreColor: "#ff8a00",
    coreGlow: "#ffb347",
    particleColor: "#ff9a33",
    orbitColor: "#ffb347",
    nodeColor: "#ffd08a",

    intensity: 1,
  },
};

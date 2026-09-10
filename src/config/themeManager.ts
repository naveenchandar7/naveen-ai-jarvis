import { themes, type ThemeName, type ThemeTokens } from "./theme";

/*
 * themeManager is a tiny external store.
 *
 * - applyTheme() writes CSS custom properties for the HUD (2D layer)
 *   AND updates the in-memory "current theme" snapshot.
 * - Three.js materials cannot read CSS variables, so R3F components
 *   read theme colors from this JS snapshot via the useTheme() hook
 *   (see src/hooks/useTheme.ts), which stays in sync automatically.
 */

let currentThemeName: ThemeName = "blue";
const listeners = new Set<() => void>();

function writeCssVariables(theme: ThemeTokens): void {
  const root = document.documentElement;

  root.style.setProperty("--background", theme.background);
  root.style.setProperty("--surface", theme.surface);
  root.style.setProperty("--surface-strong", theme.surfaceStrong);

  root.style.setProperty("--primary", theme.primary);
  root.style.setProperty("--primary-strong", theme.primaryStrong);
  root.style.setProperty("--secondary", theme.secondary);
  root.style.setProperty("--accent", theme.accent);

  root.style.setProperty("--text", theme.text);
  root.style.setProperty("--text-muted", theme.textMuted);

  root.style.setProperty("--border", theme.border);
  root.style.setProperty("--shadow", theme.shadow);
  root.style.setProperty("--glow", theme.glow);

  root.style.setProperty("--core-color", theme.coreColor);
  root.style.setProperty("--node-color", theme.nodeColor);
}

export function applyTheme(themeName: ThemeName): void {
  currentThemeName = themeName;

  writeCssVariables(themes[themeName]);
  document.documentElement.dataset.theme = themeName;

  listeners.forEach((listener) => listener());
}

export function toggleTheme(): void {
  applyTheme(currentThemeName === "blue" ? "orange" : "blue");
}

export function getCurrentThemeName(): ThemeName {
  return currentThemeName;
}

export function getCurrentTheme(): ThemeTokens {
  return themes[currentThemeName];
}

export function subscribeToTheme(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function getAvailableThemes(): ThemeName[] {
  return Object.keys(themes) as ThemeName[];
}

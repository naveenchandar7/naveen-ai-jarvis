import { useSyncExternalStore } from "react";
import type { ThemeTokens } from "../config/theme";
import {
  getCurrentTheme,
  getCurrentThemeName,
  subscribeToTheme,
} from "../config/themeManager";
import type { ThemeName } from "../config/theme";

export function useTheme(): ThemeTokens {
  return useSyncExternalStore(subscribeToTheme, getCurrentTheme);
}

export function useThemeName(): ThemeName {
  return useSyncExternalStore(subscribeToTheme, getCurrentThemeName);
}

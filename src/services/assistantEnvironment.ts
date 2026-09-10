export type AssistantEnvironment =
  | "browser"
  | "desktop";

function detectEnvironment(): AssistantEnvironment {
  const userAgent =
    navigator.userAgent.toLowerCase();

  const isDesktopShell =
    userAgent.includes("tauri") ||
    userAgent.includes("electron");

  return isDesktopShell
    ? "desktop"
    : "browser";
}

export const assistantEnvironment = {
  current: detectEnvironment(),

  isDesktop:
    detectEnvironment() === "desktop",

  isBrowser:
    detectEnvironment() === "browser",
};
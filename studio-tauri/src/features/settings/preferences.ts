import type { AppSettings } from "../../backend";

const darkThemeMediaRules = new Set<CSSMediaRule>();
const lightThemeMediaRules = new Set<CSSMediaRule>();

export function applyTheme(theme: AppSettings["theme"]) {
  try {
    localStorage.setItem("resubwinny-theme", theme);
  } catch {
    // Rust settings remain authoritative when storage is unavailable.
  }
  document.documentElement.dataset.theme = theme === "system" ? "" : theme;
  for (const sheet of [...document.styleSheets]) {
    try {
      for (const rule of [...sheet.cssRules]) {
        if (
          rule instanceof CSSMediaRule &&
          rule.conditionText === "(prefers-color-scheme: dark)"
        ) {
          darkThemeMediaRules.add(rule);
        } else if (
          rule instanceof CSSMediaRule &&
          rule.conditionText === "(prefers-color-scheme: light)"
        ) {
          lightThemeMediaRules.add(rule);
        }
      }
    } catch {
      // Cross-origin stylesheets are not owned by this application.
    }
  }
  const mediaText =
    theme === "dark"
      ? "all"
      : theme === "light"
        ? "not all"
        : "(prefers-color-scheme: dark)";
  for (const rule of darkThemeMediaRules) rule.media.mediaText = mediaText;
  const lightMediaText =
    theme === "light"
      ? "all"
      : theme === "dark"
        ? "not all"
        : "(prefers-color-scheme: light)";
  for (const rule of lightThemeMediaRules) rule.media.mediaText = lightMediaText;
}

export function restoreCachedTheme() {
  try {
    const cached = localStorage.getItem("resubwinny-theme");
    if (cached === "system" || cached === "light" || cached === "dark") {
      applyTheme(cached);
    }
  } catch {
    // The persisted Rust setting is applied during application startup.
  }
}

export function resolveLocale(setting: string) {
  if (setting !== "system") return setting;
  const system = navigator.language.toLowerCase();
  if (/^zh-(tw|hk|mo|hant)/.test(system) || system === "zh-hant") return "zh-TW";
  return system.startsWith("zh") ? "zh-CN" : system.startsWith("ja") ? "ja" : "en";
}

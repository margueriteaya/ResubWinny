import type { AppSettings } from "../../backend";

export const CURRENT_ONBOARDING_VERSION = 3;
const CACHE_KEY = "resubwinny-onboarding-version";

export class OnboardingSession {
  constructor(private readonly desktopRuntime: boolean) {}

  shouldShow(settings: AppSettings | null) {
    if (this.desktopRuntime)
      return settings == null || settings.onboardingVersion < CURRENT_ONBOARDING_VERSION;
    try {
      return Number(localStorage.getItem(CACHE_KEY) ?? 0) < CURRENT_ONBOARDING_VERSION;
    } catch {
      return true;
    }
  }

  completed(settings: AppSettings) {
    return { ...settings, onboardingVersion: CURRENT_ONBOARDING_VERSION };
  }

  cacheCompletion() {
    if (this.desktopRuntime) return;
    try { localStorage.setItem(CACHE_KEY, String(CURRENT_ONBOARDING_VERSION)); } catch { /* preview cache is best-effort */ }
  }
}

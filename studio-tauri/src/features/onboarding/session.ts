import type { AppSettings } from "../../backend";

export const CURRENT_ONBOARDING_VERSION = 3;
const CACHE_KEY = "resubwinny-onboarding-version";

type CompletionBindings = {
  required: () => boolean;
  settings: () => AppSettings;
  persist: (settings: AppSettings) => Promise<AppSettings>;
  setSettings: (settings: AppSettings) => void;
  setSaving: (saving: boolean) => void;
  clearError: () => void;
  fail: (reason: unknown) => void;
  close: () => void;
  completed: () => void;
};

export class OnboardingSession {
  private readonly desktopRuntime: boolean;
  constructor(desktopRuntime: boolean) { this.desktopRuntime = desktopRuntime; }

  async finish(userMode: AppSettings["userMode"], bindings: CompletionBindings) {
    if (!bindings.required()) { bindings.close(); return; }
    bindings.setSaving(true);
    bindings.clearError();
    try {
      const next = this.completed({ ...bindings.settings(), userMode });
      bindings.setSettings(this.desktopRuntime ? await bindings.persist(next) : next);
      this.cacheCompletion();
      bindings.completed();
    } catch (reason) {
      bindings.fail(reason);
    } finally {
      bindings.setSaving(false);
    }
  }

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

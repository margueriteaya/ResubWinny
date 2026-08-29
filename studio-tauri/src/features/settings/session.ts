import type { AppSettings } from "../../backend";
import type { LanguagePack } from "../../i18n";
import { applyTheme, resolveLocale } from "./preferences";

type PreferencesHooks = {
  desktopRuntime: boolean;
  getSettings: () => Promise<AppSettings>;
  updateSettings: (settings: AppSettings) => Promise<AppSettings>;
  setCaptionFont: (font: string) => Promise<void>;
  listLanguagePacks: () => Promise<LanguagePack[]>;
  registerLanguagePacks: (packs: LanguagePack[]) => void;
  locale: () => string;
  setLocale: (locale: string) => void;
  onError: (reason: unknown) => void;
};

/** Owns application preference application and persistence without becoming a global store. */
export class PreferencesSession {
  constructor(private readonly hooks: PreferencesHooks) {}

  async apply(settings: AppSettings, refreshLanguagePacks = false) {
    if (this.hooks.desktopRuntime && refreshLanguagePacks)
      this.hooks.registerLanguagePacks(await this.hooks.listLanguagePacks());
    const selected = resolveLocale(settings.locale);
    if (this.hooks.locale() !== selected) this.hooks.setLocale(selected);
    applyTheme(settings.theme);
  }

  async load(refreshLanguagePacks = true) {
    if (!this.hooks.desktopRuntime) return null;
    try {
      const settings = await this.hooks.getSettings();
      await this.apply(settings, refreshLanguagePacks);
      return settings;
    } catch (reason) {
      this.hooks.onError(reason);
      return null;
    }
  }

  persist(settings: AppSettings) {
    if (this.hooks.desktopRuntime)
      void this.hooks.updateSettings(settings).catch(this.hooks.onError);
  }

  async saveCaptionFont(font: string) {
    if (!this.hooks.desktopRuntime) return;
    try {
      await this.hooks.setCaptionFont(font);
    } catch (reason) {
      this.hooks.onError(reason);
    }
  }
}

import type { AppSettings } from "../../backend";
import type { LanguagePack } from "../../i18n";
import { applyTheme, resolveLocale } from "./preferences";
import { copySettings, SettingsPersistenceQueue } from "./persistence-queue";

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
  private readonly hooks: PreferencesHooks;
  private readonly persistence: SettingsPersistenceQueue;

  constructor(hooks: PreferencesHooks) {
    this.hooks = hooks;
    this.persistence = new SettingsPersistenceQueue(hooks.updateSettings, hooks.onError);
  }

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

  persist(settings: AppSettings): Promise<AppSettings | null> {
    return this.hooks.desktopRuntime
      ? this.persistence.persist(settings)
      : Promise.resolve(copySettings(settings));
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

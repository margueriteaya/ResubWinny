import type { AppSettings } from "../../backend";

export function copySettings(settings: AppSettings): AppSettings {
  return {
    ...settings,
    exportPreferences: {
      formats: [...settings.exportPreferences.formats],
      preservation: { ...settings.exportPreferences.preservation },
    },
    workspaceLayout: { ...settings.workspaceLayout },
  };
}

export class SettingsPersistenceQueue {
  private readonly updateSettings: (settings: AppSettings) => Promise<AppSettings>;
  private readonly onError: (reason: unknown) => void;
  private pendingSettings: AppSettings | null = null;
  private persistenceRun: Promise<AppSettings | null> | null = null;

  constructor(
    updateSettings: (settings: AppSettings) => Promise<AppSettings>,
    onError: (reason: unknown) => void,
  ) {
    this.updateSettings = updateSettings;
    this.onError = onError;
  }

  persist(settings: AppSettings): Promise<AppSettings | null> {
    this.pendingSettings = copySettings(settings);
    this.persistenceRun ??= this.drain();
    return this.persistenceRun;
  }

  private async drain(): Promise<AppSettings | null> {
    let persisted: AppSettings | null = null;
    while (this.pendingSettings) {
      const candidate = this.pendingSettings;
      this.pendingSettings = null;
      try {
        persisted = await this.updateSettings(candidate);
      } catch (reason) {
        this.onError(reason);
      }
    }
    this.persistenceRun = null;
    return persisted;
  }
}

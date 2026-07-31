import type { AppSettings, LanguagePack, TaskHistoryRecord } from '../backend'
import { call } from './client'

export const settingsApi = {
  getSettings: () => call<AppSettings>('get_settings'),
  updateSettings: (settings: AppSettings) => call<AppSettings>('update_settings', { settings }),
  listLanguagePacks: () => call<LanguagePack[]>('list_language_packs'),
  openLanguagePackDirectory: () => call<void>('open_language_pack_directory'),
  loadTaskHistory: () => call<TaskHistoryRecord[]>('load_task_history'),
  saveTaskHistory: (records: TaskHistoryRecord[]) => call<void>('save_task_history', { records }),
}

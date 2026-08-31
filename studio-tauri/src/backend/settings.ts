import type { AboutInfo, AppSettings, LanguagePack, TaskHistoryRecord } from '../backend'
import { call } from './client'

export const settingsApi = {
  getAboutInfo: () => call<AboutInfo>('get_about_info'),
  openProjectLink: (target: 'source' | 'releases' | 'issues') => call<void>('open_project_link', { target }),
  getSettings: () => call<AppSettings>('get_settings'),
  updateSettings: (settings: AppSettings) => call<AppSettings>('update_settings', { settings }),
  listLanguagePacks: () => call<LanguagePack[]>('list_language_packs'),
  openLanguagePackDirectory: () => call<void>('open_language_pack_directory'),
  loadTaskHistory: () => call<TaskHistoryRecord[]>('load_task_history'),
  saveTaskHistory: (records: TaskHistoryRecord[]) => call<void>('save_task_history', { records }),
}

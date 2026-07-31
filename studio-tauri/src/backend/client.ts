import { invoke } from '@tauri-apps/api/core'

export const call = <T>(command: string, payload?: Record<string, unknown>) =>
  invoke<T>(command, payload)

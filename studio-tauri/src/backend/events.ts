import { listen } from '@tauri-apps/api/event'
import type { TaskEvent } from '../backend'

export const subscribeTaskEvents = (handler: (event: TaskEvent) => void) =>
  listen<TaskEvent>('task-event', ({ payload }) => handler(payload))

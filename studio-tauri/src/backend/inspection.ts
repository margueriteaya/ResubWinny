import type { Inspection } from '../backend'
import { call } from './client'

export const inspectionApi = {
  inspectSource: (path: string) => call<Inspection>('inspect_source', { path }),
}

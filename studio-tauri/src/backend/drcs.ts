import type { DrcsGlyph, DrcsMapping } from '../backend'
import { call } from './client'

export const drcsApi = {
  loadDrcsReport: (path: string) => call<DrcsGlyph[]>('load_drcs_report', { path }),
  loadDrcsMappings: () => call<DrcsMapping[]>('load_drcs_mappings'),
  saveDrcsMappings: (mappings: DrcsMapping[]) => call<void>('save_drcs_mappings', { mappings }),
}

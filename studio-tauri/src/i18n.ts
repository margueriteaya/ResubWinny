import en from './locales/en.json'
import ja from './locales/ja.json'
import zhCN from './locales/zh-CN.json'
import zhTW from './locales/zh-TW.json'
import { writable } from 'svelte/store'

export type Locale = string
export type LanguagePack = { locale: string; name: string; messages: Record<string, string> }

const builtIn: LanguagePack[] = [en, zhCN, zhTW, ja].map((pack) => ({
  locale: pack.locale,
  name: pack.name,
  messages: pack.messages as Record<string, string>,
}))
const english = builtIn[0]
const builtInPacks = new Map<string, LanguagePack>(builtIn.map((pack) => [pack.locale, pack]))
const packs = new Map<string, LanguagePack>(builtIn.map((pack) => [pack.locale, pack]))
let customLocales = new Set<string>()
const browserLocale = typeof navigator !== 'undefined' ? navigator.language : 'en'
const normalizedBrowserLocale = browserLocale.toLowerCase()
let currentLocale = /^(zh-tw|zh-hk|zh-mo)/.test(normalizedBrowserLocale)
  ? 'zh-TW'
  : normalizedBrowserLocale.startsWith('zh')
    ? 'zh-CN'
    : normalizedBrowserLocale.startsWith('ja') ? 'ja' : 'en'

// Components which predate the language-pack work call `t()` directly.  This
// revision store gives the app shell one explicit reactive invalidation point
// whenever the selected pack or its contents changes.
export const localeRevision = writable(0)
function invalidateLocale() { localeRevision.update((revision) => revision + 1) }

export function availableLocales() { return [...packs.values()].map(({ locale, name }) => ({ locale, name })) }
export function setLocale(locale: Locale) {
  currentLocale = packs.has(locale) ? locale : 'en'
  invalidateLocale()
}
export function locale() { return currentLocale }

// Custom packs are merged over their built-in equivalent. Invalid keys remain
// harmless because English is always the fallback source of truth.
export function registerLanguagePacks(custom: LanguagePack[]) {
  const before = JSON.stringify(packs.get(currentLocale))
  for (const locale of customLocales) {
    const builtIn = builtInPacks.get(locale)
    if (builtIn) packs.set(locale, builtIn)
    else packs.delete(locale)
  }
  customLocales = new Set()
  for (const pack of custom) {
    if (!pack.locale || !pack.name || !pack.messages) continue
    const base = packs.get(pack.locale)
    packs.set(pack.locale, {
      locale: pack.locale,
      name: pack.name,
      messages: { ...(base?.messages ?? english.messages), ...pack.messages },
    })
    customLocales.add(pack.locale)
  }
  if (!packs.has(currentLocale)) currentLocale = 'en'
  if (before !== JSON.stringify(packs.get(currentLocale))) invalidateLocale()
}

export function t(key: string, fallback?: string) {
  return packs.get(currentLocale)?.messages[key] ?? english.messages[key] ?? fallback ?? key
}

export function formatMessage(key: string, parameters: Record<string, unknown> = {}, fallback?: string) {
  return t(key, fallback).replace(/\{([^}]+)\}/g, (match: string, name: string) => {
    const value = parameters[name]
    return value === undefined || value === null ? match : String(value)
  })
}

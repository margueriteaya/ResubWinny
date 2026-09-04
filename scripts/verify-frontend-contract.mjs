import { readFile, readdir } from 'node:fs/promises'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const frontendRoot = join(root, 'studio-tauri', 'src')
const handlerSource = await readFile(join(root, 'studio-tauri', 'src-tauri', 'src', 'main.rs'), 'utf8')
const formatCapabilities = JSON.parse(await readFile(join(root, 'shared', 'format_capabilities.json'), 'utf8'))

async function filesIn(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  const files = await Promise.all(entries.map(async (entry) => {
    const path = join(directory, entry.name)
    return entry.isDirectory() ? filesIn(path) : [path]
  }))
  return files.flat()
}

const sourceFiles = (await filesIn(frontendRoot)).filter((path) => /\.(ts|svelte)$/.test(path))
const allowedTauriImports = new Set([
  'backend/client.ts',
  'backend/events.ts',
  'shell/desktop.ts',
])
const violations = []
const exportFormats = ['ASS', 'TTML', 'SRT', 'WebVTT', 'JSON', 'Raw Data']
const preservationFeatures = ['position', 'color', 'ruby', 'drcs', 'gaiji', 'accessibility']
const capabilityLevels = new Set(['preserved', 'approximated', 'unsupported', 'conditional'])
for (const format of exportFormats) {
  if (!formatCapabilities[format]) {
    violations.push(`format capability contract is missing ${format}`)
    continue
  }
  for (const feature of preservationFeatures) {
    const level = formatCapabilities[format][feature]
    if (!capabilityLevels.has(level)) violations.push(`${format}.${feature} has invalid capability level ${String(level)}`)
  }
}
for (const format of Object.keys(formatCapabilities)) {
  if (!exportFormats.includes(format)) violations.push(`format capability contract has unknown format ${format}`)
}
const localeDirectory = join(frontendRoot, 'locales')
const localeFiles = (await readdir(localeDirectory)).filter((name) => name.endsWith('.json'))
const localeDocuments = await Promise.all(localeFiles.map(async (name) => ({
  name,
  document: JSON.parse(await readFile(join(localeDirectory, name), 'utf8')),
})))
const referenceLocale = localeDocuments.find(({ name }) => name === 'en.json')
if (!referenceLocale) violations.push('locales/en.json is required as the message-key reference')
const referenceKeys = new Set(Object.keys(referenceLocale?.document.messages ?? {}))

for (const { name, document } of localeDocuments) {
  const keys = new Set(Object.keys(document.messages ?? {}))
  for (const key of referenceKeys) {
    if (!keys.has(key)) violations.push(`${name} is missing locale key ${key}`)
  }
  for (const key of keys) {
    if (!referenceKeys.has(key)) violations.push(`${name} has locale key ${key} which is absent from en.json`)
  }
}

for (const path of sourceFiles) {
  const text = await readFile(path, 'utf8')
  const localPath = relative(frontendRoot, path).replaceAll('\\', '/')
  if (text.includes('@tauri-apps/') && !allowedTauriImports.has(localPath)) {
    violations.push(`${localPath} imports a Tauri API outside the backend or desktop shell boundary`)
  }
  if (/\b(?:invoke|listen)\s*\(/.test(text) && !allowedTauriImports.has(localPath)) {
    violations.push(`${localPath} calls invoke/listen outside the backend boundary`)
  }
  for (const match of text.matchAll(/\bt\(\s*['"]([^'"]+)['"]/g)) {
    if (!referenceKeys.has(match[1])) violations.push(`${localPath} uses unknown locale key ${match[1]}`)
  }
}

const backendFiles = sourceFiles.filter((path) => relative(frontendRoot, path).replaceAll('\\', '/').startsWith('backend/'))
const commands = new Set()
for (const path of backendFiles) {
  const text = await readFile(path, 'utf8')
  for (const match of text.matchAll(/call(?:<[^>]+>)?\(['"]([a-z0-9_]+)['"]/g)) commands.add(match[1])
}
for (const command of commands) {
  if (!new RegExp(`::${command}\\b`).test(handlerSource)) violations.push(`Tauri command ${command} is exposed by the frontend but not registered in main.rs`)
}

if (violations.length) {
  console.error('Frontend contract verification failed:')
  for (const violation of violations) console.error(`- ${violation}`)
  process.exit(1)
}

console.log(`Frontend contract verified: ${commands.size} typed commands, ${sourceFiles.length} source files, ${localeFiles.length} complete locales.`)

import { readFile, readdir } from 'node:fs/promises'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import assert from 'node:assert/strict'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const modeToken = /\b(?:userMode|user_mode|UserMode|default_user_mode|normie|nerd)\b/

// Settings persistence is allowed to carry presentation policy. Remove only
// those declarations, never the other request/result types in the same file.
function withoutBlock(source, marker) {
  const start = source.indexOf(marker)
  if (start < 0) return source
  const open = source.indexOf('{', start)
  if (open < 0) throw new Error(`Missing declaration body: ${marker}`)
  let depth = 0
  for (let index = open; index < source.length; index++) {
    if (source[index] === '{') depth++
    if (source[index] === '}' && --depth === 0) return source.slice(0, start) + source.slice(index + 1)
  }
  throw new Error(`Unclosed declaration: ${marker}`)
}

function semanticSource(path, source) {
  if (path === 'studio-tauri/src/backend.ts') {
    source = source.replace(/^export type UserMode = .*$/m, '')
    return withoutBlock(source, 'export type AppSettings =')
  }
  if (path === 'studio-tauri/src-tauri/src/models.rs') {
    for (const marker of ['pub struct AppSettings', 'impl Default for AppSettings', 'fn default_user_mode()']) source = withoutBlock(source, marker)
  }
  return source
}

async function filesIn(path) {
  return (await Promise.all((await readdir(path, { withFileTypes: true })).map((entry) => {
    const next = join(path, entry.name)
    return entry.isDirectory() ? filesIn(next) : [next]
  }))).flat()
}

// Mutation checks ensure the settings exemption cannot hide mode fields in
// neighboring export requests or artifact types.
assert.equal(modeToken.test(semanticSource('studio-tauri/src/backend.ts',
  "export type UserMode = 'normie' | 'nerd'\nexport type AppSettings = { userMode: UserMode }")), false)
assert.equal(modeToken.test(semanticSource('studio-tauri/src-tauri/src/models.rs',
  'pub struct AppSettings { pub user_mode: String }\nimpl Default for AppSettings { fn default() { user_mode: default_user_mode() } }\nfn default_user_mode() { "normie" }')), false)
for (const [path, sample] of [
  ['studio-tauri/src/backend.ts', 'export type AppSettings = { userMode: UserMode }\nexport type ExportRequest = { userMode: UserMode }'],
  ['studio-tauri/src-tauri/src/models.rs', 'pub struct AppSettings { pub user_mode: String }\npub struct Artifact { pub user_mode: String }'],
]) assert.ok(modeToken.test(semanticSource(path, sample)))

const roots = ['crates/arib-caption-worker/src', 'shared', 'studio-tauri/src-tauri/src', 'studio-tauri/src/backend', 'studio-tauri/src/features/tasks']
const paths = [...(await Promise.all(roots.map((path) => filesIn(join(root, path))))).flat(), join(root, 'studio-tauri/src/backend.ts')]
const violations = []
let checked = 0
for (const path of paths) {
  if (!/\.(rs|ts)$/.test(path)) continue
  const local = relative(root, path).replaceAll('\\', '/')
  if (local === 'studio-tauri/src-tauri/src/settings.rs') continue
  const source = semanticSource(local, await readFile(path, 'utf8'))
  checked++
  if (modeToken.test(source)) violations.push(`${local}: presentation mode crossed the semantic boundary`)
}
if (violations.length) throw new Error(violations.join('\n'))
console.log(`Mode boundary verified: ${checked} semantic source files; settings exemptions mutation-checked.`)

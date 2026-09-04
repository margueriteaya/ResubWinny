import assert from 'node:assert/strict'
import test from 'node:test'
import { emptyTaskEventState, featureCountSummary, reduceTaskEvent } from '../studio-tauri/src/features/tasks/event-state.ts'
import { assessExports } from '../studio-tauri/src/features/tasks/export-assessment.ts'

const preservation = { position: true, color: true, ruby: true, drcs: true, gaiji: true, accessibility: true }

test('assessment entries preserve the source-state and user-intent truth table for all formats', () => {
  const expected = {
    ASS: ['preserved', 'preserved', 'approximated', 'conditional', 'preserved', 'preserved'],
    TTML: ['preserved', 'preserved', 'preserved', 'conditional', 'approximated', 'preserved'],
    SRT: ['unsupported', 'unsupported', 'unsupported', 'conditional', 'approximated', 'preserved'],
    WebVTT: ['unsupported', 'unsupported', 'unsupported', 'conditional', 'approximated', 'preserved'],
    JSON: Array(6).fill('preserved'),
    'Raw Data': Array(6).fill('preserved'),
  }
  for (const [format, levels] of Object.entries(expected)) {
    for (const [index, feature] of Object.keys(preservation).entries()) {
      for (const state of ['unknown', 'present', 'absent']) {
        for (const enabled of [true, false]) {
          const knowledge = Object.fromEntries(Object.keys(preservation).map((name) => [name, { state: name === feature ? state : 'absent', complete: state === 'absent' }]))
          const result = assessExports([format], { ...preservation, [feature]: enabled }, knowledge)
          const groups = result.formats[format]
          const entries = Object.values(groups).flat()
          const level = levels[index]
          let bucket
          if (state === 'present') bucket = !enabled ? 'dropped' : level === 'unsupported' ? 'conflicts' : level === 'conditional' ? 'approximated' : level
          if (state === 'unknown' && enabled && ['unsupported', 'conditional'].includes(level)) bucket = 'conditional'
          if (state === 'unknown' && !enabled) bucket = 'conditional'
          assert.equal(entries.length, bucket ? 1 : 0, `${format}/${feature}/${state}/${enabled}`)
          assert.equal(result.hasConflict, bucket === 'conflicts')
          if (!bucket) continue
          const item = groups[bucket][0]
          assert.equal(item.feature, feature)
          assert.deepEqual(item.parameters, { format, feature })
          assert.equal(typeof item.code, 'string')
          assert.ok(Array.isArray(item.actions))
          if (bucket === 'dropped') assert.equal(item.severity, undefined)
          if (state === 'unknown' && !enabled) {
            assert.equal(item.code, 'feature_will_be_dropped_if_present')
            assert.equal(item.severity, undefined)
          }
        }
      }
    }
  }
})

test('one source feature yields independent results for simultaneous target formats', () => {
  const result = assessExports(['ASS', 'SRT', 'TTML'], preservation, { ruby: { state: 'present', complete: false } })
  assert.equal(result.formats.ASS.approximated[0].code, 'format_approximates_feature')
  assert.equal(result.formats.SRT.conflicts[0].code, 'format_cannot_preserve_feature')
  assert.equal(result.formats.TTML.preserved[0].code, 'format_preserves_feature')
  assert.equal(result.hasConflict, true)
})

test('unsupported feature conflicts offer explicit remedies without changing selections', () => {
  for (const feature of ['position', 'color', 'ruby']) {
    const formats = new Set(['ASS', 'SRT'])
    const preferences = { ...preservation }
    const knowledge = { [feature]: { state: 'present', complete: false } }
    const result = assessExports(formats, preferences, knowledge)
    const conflict = result.formats.SRT.conflicts.find((item) => item.feature === feature)
    assert.deepEqual(conflict.parameters, { format: 'SRT', feature })
    assert.deepEqual(conflict.actions, [`disable_preservation:${feature}`, 'remove_format', 'choose_compatible_format'])
    assert.equal(result.hasConflict, true)
    assert.deepEqual([...formats], ['ASS', 'SRT'])
    assert.deepEqual(preferences, preservation)
    assert.equal(assessExports(['ASS'], preferences, knowledge).hasConflict, false)
    assert.equal(assessExports(formats, { ...preferences, [feature]: false }, knowledge).hasConflict, false)
  }
})

const event = (kind, logicalTrack, feature, parameters = {}) => ({
  kind,
  message: kind,
  parameters: { logicalTrack, feature, ...parameters },
})

const reduce = (state, taskEvent, source = 'recording.ts') =>
  reduceTaskEvent(state, taskEvent, 1_000, taskEvent.message, false, source).state

test('feature knowledge is isolated by source and logical track', () => {
  let state = emptyTaskEventState()
  state = reduce(state, event('feature_observed', 'service=1:component=48:lang=jpn', 'ruby', { observedCount: 1 }))
  state = reduce(state, event('feature_summary', 'service=1:component=49:lang=eng', 'ruby', { state: 'absent', complete: true }))
  state = reduce(state, event('feature_observed', 'service=1:component=48:lang=jpn', 'color', { observedCount: 2 }), 'other.ts')

  assert.equal(state.featureKnowledge['recording.ts::service=1:component=48:lang=jpn'].ruby.state, 'present')
  assert.equal(state.featureKnowledge['recording.ts::service=1:component=49:lang=eng'].ruby.state, 'absent')
  assert.equal(state.featureKnowledge['other.ts::service=1:component=48:lang=jpn'].color.state, 'present')
})

test('PID evidence changes do not split one logical track', () => {
  let state = emptyTaskEventState()
  const track = 'service=1:component=48:lang=jpn'
  state = reduce(state, event('feature_observed', track, 'ruby', { observedCount: 1, details: { pid: 256 } }))
  state = reduce(state, event('feature_observed', track, 'ruby', { observedCount: 2, details: { pid: 512 } }))
  assert.deepEqual(Object.keys(state.featureKnowledge), [`recording.ts::${track}`])
  assert.equal(state.featureKnowledge[`recording.ts::${track}`].ruby.observedCount, 2)
})

test('feature state is monotonic and absent requires a complete summary', () => {
  let state = emptyTaskEventState()
  const track = 'logical-track'
  state = reduce(state, event('feature_summary', track, 'ruby', { state: 'absent', complete: false }))
  assert.equal(state.featureKnowledge['recording.ts::logical-track'], undefined)
  state = reduce(state, event('feature_observed', track, 'ruby', { observedCount: 3 }))
  state = reduce(state, event('feature_summary', track, 'ruby', { state: 'absent', complete: true }))
  assert.equal(state.featureKnowledge['recording.ts::logical-track'].ruby.state, 'present')
})

test('count summaries distinguish observations from final totals', () => {
  assert.deepEqual(featureCountSummary({ state: 'present', observedCount: 3, complete: false }), { count: 3, final: false })
  assert.deepEqual(featureCountSummary({ state: 'present', observedCount: 7, complete: true }), { count: 7, final: true })
  assert.equal(featureCountSummary({ state: 'unknown', observedCount: 3, complete: false }), null)
})

test('runtime export conflicts do not contaminate source feature details', () => {
  const state = reduce(emptyTaskEventState(), {
    kind: 'failed',
    code: 'export_conflict',
    message: 'fallback text must not drive semantics',
    parameters: {
      logicalTrack: 'logical-track',
      feature: 'drcs',
      formats: ['SRT'],
      issueCode: 'unresolved_drcs_text_target',
      availableActions: ['open_drcs_mapping'],
    },
  })
  const key = 'recording.ts::logical-track'
  assert.deepEqual(state.featureKnowledge[key].drcs.details, {})
  assert.deepEqual(state.exportConflicts[key].drcs, {
    formats: ['SRT'],
    issueCode: 'unresolved_drcs_text_target',
    availableActions: ['open_drcs_mapping'],
  })
})

test('conditional DRCS converges to approximation or a format-specific runtime conflict', () => {
  const knowledge = { drcs: { state: 'present', observedCount: 1, complete: false } }
  const allowed = assessExports(['ASS', 'SRT'], preservation, knowledge)
  assert.deepEqual(allowed.formats.ASS.approximated.map((item) => item.feature), ['drcs'])
  assert.deepEqual(allowed.formats.SRT.approximated.map((item) => item.feature), ['drcs'])
  assert.equal(allowed.hasConflict, false)

  const conflict = assessExports(['ASS', 'SRT'], preservation, knowledge, {
    drcs: {
      formats: ['SRT'],
      issueCode: 'unresolved_drcs_text_target',
      availableActions: ['open_drcs_mapping'],
    },
  })
  assert.deepEqual(conflict.formats.ASS.approximated.map((item) => item.feature), ['drcs'])
  assert.equal(conflict.formats.SRT.conflicts[0].code, 'unresolved_drcs_text_target')
  assert.equal(conflict.hasConflict, true)
})

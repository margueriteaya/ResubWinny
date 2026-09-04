import assert from 'node:assert/strict'
import test from 'node:test'
import { emptyTaskEventState, featureCountSummary, reduceTaskEvent } from '../studio-tauri/src/features/tasks/event-state.ts'
import { assessExports } from '../studio-tauri/src/features/tasks/export-assessment.ts'

const preservation = { position: true, color: true, ruby: true, drcs: true, gaiji: true, accessibility: true }

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
  assert.deepEqual(allowed.formats.ASS.approximated, ['drcs'])
  assert.deepEqual(allowed.formats.SRT.approximated, ['drcs'])
  assert.equal(allowed.hasConflict, false)

  const conflict = assessExports(['ASS', 'SRT'], preservation, knowledge, {
    drcs: {
      formats: ['SRT'],
      issueCode: 'unresolved_drcs_text_target',
      availableActions: ['open_drcs_mapping'],
    },
  })
  assert.deepEqual(conflict.formats.ASS.approximated, ['drcs'])
  assert.equal(conflict.formats.SRT.conflicts[0].code, 'unresolved_drcs_text_target')
  assert.equal(conflict.hasConflict, true)
})

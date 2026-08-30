# Private Windows Alpha acceptance

This matrix validates the packaged application against legally held Japanese
broadcast recordings. Recordings, decoded captions, programme names, local
paths, and screenshots remain private. A release may publish the result summary,
fixture class, byte size, duration, hashes of generated release artifacts, and
pass/fail counts only.

Public protocol fixtures do not replace this matrix, and no synthetic broadcast
stream should be added merely to imitate a broadcaster-specific recording.
Parser unit fixtures remain useful for bounded malformed-input regression.

## Candidate identity

Record these fields before testing. They must match `RELEASE-MANIFEST.json` and
`SHA256SUMS.txt` from the candidate directory.

| Field | Result |
| --- | --- |
| Release tag / commit | Pending |
| Installer name / SHA-256 | Pending |
| `RELEASE-MANIFEST.json` SHA-256 | Pending |
| libmpv `SOURCE-RECEIPT.json` SHA-256 | Pending |
| Windows edition / build | Pending |
| CPU / GPU / graphics driver | Pending |
| Clean install or upgrade path | Pending |

Use an opaque local fixture ID such as `terrestrial-a` in notes. Do not record a
programme title, broadcaster schedule, source filename, or filesystem path.

## Compatibility matrix

| Category | Required packaged workflow evidence | Pass criteria | Result |
| --- | --- | --- | --- |
| Terrestrial TS | Detect service and B24 track; preview; timeline; ASS and TTML export | Correct selected track, bounded memory, usable preview, non-empty validated exports | Pending |
| BS TS | Detect service and caption track; preview; timeline; export | Same end-to-end result without fixed PID or filename assumptions | Pending |
| Long programme (2–4 h) | Open, index, seek near start/middle/end, preview, export | No unbounded memory growth, stale timeline, seek lockup, or incomplete final artifact | Pending |
| DRCS | Inspect report, map known glyph, leave unknown glyph unresolved, export | Mapping persists; fallback remains visible and diagnosable | Pending |
| Multiple services/tracks | Switch the selected caption track and export each independently | Preview, event count, evidence, and export follow only the selected logical track | Pending |
| Damaged/truncated recording | Inspect and convert a legally held corrupted or truncated capture | Bounded processing, explicit diagnostics, no fabricated service/track, recoverable outputs remain valid | Pending |
| BS4K M2TS/private PES | Run only the already evidenced real-sample route | Result is reported narrowly as M2TS/private-PES TTML support | Pending |
| TLV/MMTP | Run only when a lawful real sample exists | Result remains experimental and evidence-first; it cannot promote general BS4K/8K support | Not gated |

## End-to-end subtitle workflow

Run the installed application, not a development server or Cargo test harness:

1. Verify the installer hash against `SHA256SUMS.txt`, then install while
   acknowledging the documented unknown-publisher warning.
2. Open the recording and confirm detected container, service, programme
   metadata evidence, and available caption tracks.
3. Start native Preview; seek and pause/resume at multiple positions.
4. Inspect timeline events, warnings, DRCS, and diagnostics.
5. Select ASS and TTML, choose the intended preservation options, and export.
6. Open the results in the normal downstream subtitle workflow and record any
   text, timing, layout, Ruby, colour, DRCS, or usability defect.
7. Uninstall the candidate and confirm that user-selected exports remain while
   application files are removed.

Existing opt-in corpus and native-preview checks in `docs/corpus.md` remain
supporting evidence. They do not replace the installed-application workflow.

## Publishable result

Release notes may publish a summary in this form:

```text
Candidate: <tag> @ <commit>
Windows: <edition/build>; GPU class: <vendor/model>
Private fixtures: terrestrial N, BS N, long N, damaged N
Passed: N / N required matrix rows
Skipped: <rows and reason>
Known failures: <issue links or concise descriptions>
No recording bytes, captions, programme metadata, or screenshots published.
```

An Unsigned Windows Alpha may proceed only when every required row is passed or
the release notes explicitly identify a skipped private-corpus gate as allowed
by `release-checklist.md`. A failed required row is a release blocker, not a
reason to weaken the matrix.

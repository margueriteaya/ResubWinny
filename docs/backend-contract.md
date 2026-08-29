# Backend contract

The Tauri/Svelte UI is a client of the Rust backend. It does not parse TS/TLV data, decode ARIB, render high-resolution video, or decide conversion semantics.

The durable `.caption.jsonl` format is specified separately in
[`contracts/archive.md`](contracts/archive.md), including its explicit schema
version and streaming-reader compatibility rules.

The contract is split into focused reading guides: [`contracts/tauri-api.md`](contracts/tauri-api.md),
[`contracts/worker-protocol.md`](contracts/worker-protocol.md),
[`contracts/preview.md`](contracts/preview.md), and
[`contracts/timeline.md`](contracts/timeline.md). This file remains the
compatibility index and detailed reference.

The backend surface is a bounded, stable application contract. During the
current convergence phase, prefer consolidating related queries over adding
new one-off command variants:

| Command | Responsibility |
| --- | --- |
| `inspect_source` | bounded probe of a recording and caption-track discovery |
| `start_export` | starts the streaming worker and emits `task-event` progress; accepts an optional validated `trackId` |
| `cancel_export` | stops the current worker process |
| `pause_export` / `resume_export` | sends cooperative control messages to the worker |
| `create_job` / `list_jobs` / `get_job` / `remove_job` | persists task summaries without media payloads |
| `start_job` / `pause_job` / `resume_job` / `cancel_job` | controls a persisted job through the worker supervisor |
| `get_job_diagnostics` | returns bounded, structured diagnostics collected for a persisted job |
| `get_job_diagnostics_window` | returns a bounded diagnostic page using offset/limit |
| `list_jobs_window` | returns a bounded page of recent task summaries |
| `get_job_artifacts` | returns the task artifact manifest and `.part` paths |
| `get_job_checkpoint` | returns the latest bounded progress checkpoint for a task |
| `pause_queue` / `resume_queue` / `queue_is_paused` | controls the supervisor queue and cooperatively pauses/resumes its active Worker |
| `load_drcs_report` | reads a worker-produced DRCS report and returns displayable glyph images |
| `get_settings` / `update_settings` | reads or atomically updates validated UI and export defaults in app-data `settings.json` |
| `list_language_packs` | rescans bounded JSON language files from the fixed app-data `language-packs/` directory; arbitrary browser-provided directories are not accepted |
| `open_language_pack_directory` | creates that fixed directory when needed and opens it with the platform file manager |
| `start_preview` / `resize_preview` / `stop_preview` | controls the current in-process libmpv video surface |
| `preview_command` | forwards seek/pause commands to libmpv |
| `get_preview_capabilities` | reports declared video/caption-composition routes and only the currently usable routes |
| `get_preview_runtime` | reports the discovered libmpv runtime plus render-API symbol availability without claiming a render surface exists |
| `get_preview_render_diagnostics` | reports the active native route and bounded render-thread counters/errors; absence of a worker returns a stable inactive result |
| `render_at` | returns a bounded caption-plane snapshot for a requested archive time without sending video frames through WebView |
| `sync_preview_overlay` | reads the embedded libmpv time, renders a bounded native plane, and applies, clears, or deduplicates the Windows overlay without WebView timing or layout |
| `get_playback_time_mapping` / `update_playback_time_mapping` | gets or replaces the validated media-time → project-time segment mapping used by native caption preview |
| `get_timeline_window` / `get_timeline_window_filtered` | streams a bounded archive page for completed-task browsing |
| `get_timeline_recent_window_filtered` | incrementally tails complete JSONL records and returns only the latest bounded live event page |
| `get_timeline_time_window` | returns a bounded prefetched time range for the editor timeline and incrementally reads appended records |

`render_at` is exposed in the task workspace after an archive export completes. The UI keeps the time query explicit and bounded and displays a real RGBA-derived PNG when the archive contains a B24 render frame. The backend returns `planeWidth`, `planeHeight`, `composedPngBase64`, and `activeLayerCount`; the composed image is produced by the bounded native caption-plane compositor, not by CSS or WebView text layout. A TTML interval with bounded layout fields can return a backend-rasterised 1920×1080 RGBA plane using the bundled Rounded M+ 1m for ARIB font. A valid declared display extent normalises source geometry and pixel lengths onto that logical plane; absent extent defaults to logical 2K and infers only canonical 4K/8K from complete pixel region geometry that exceeds logical 2K on at least one axis and fits that plane. Equivalent 2K/4K/8K layouts retain the same viewer-relative size without guessing ambiguous sources. The bounded rich-body parser retains text outside span/ruby tags and maps explicit span colour, size, spacing, and opacity. The native horizontal path retains explicit line breaks and applies resolved `textAlign`, `displayAlign`, and `lineHeight`. A simple horizontal `tts:ruby` base/text pair is rasterised at 0.5 scale and centred over its base span. An explicitly associated vertical ruby is likewise rasterised at 0.5 scale beside its base cells, including bounded continuation when automatic column wrapping occurs; both report `captionPlaneMode=ttml-vertical-ruby-basic-native` and `renderedRubyCount`. This continuation does not implement general B62 ruby grouping or source-specific placement. The vertical renderer uses Unicode vertical-presentation punctuation only when the bundled ARIB font contains the mapped glyph; it never approximates Latin rotation or tate-chu-yoko. Direct `tts:textOutline` accepts only `none`, TTML named colours, or full `#RRGGBB[AA]` plus a `px` width, then applies a bounded native outline; `arib-tt:border` is deliberately not converted. Complete B62 glyph orientation, standard B62 stroke behaviour, non-PNG resources, and unrenderable/missing glyphs remain explicit limitations; unsupported records remain structural previews rather than fabricated images.

TLV archive exports may also contain bounded `asset_evidence` and
`resource_evidence` records. Each `resource_evidence` record retains a
lossless base64 payload, format validation, and the exact
`packet_id + mpu_sequence_number + subsample_number` record key used by a
matching `subt://` reference. The archive-time preview reader keeps at most 64
such records, attaches only same-MPU matches to active captions, and exposes a
small verified PNG `preview_data_uri` as `resourcePreviews`. Font resources,
non-PNG resources, missing resources, and incomplete maps remain evidence only
and are not claimed as rendered caption text.

Separate bounded `asset_evidence` records identify only MPT signalling already
observed in the input
(`packet_id`, source TLV offset, `asset_type`, descriptor tags, and advertised MPU NTP values).
They are evidence for future `subt://` resource joining, not decoded image or
font bytes. `resource_reference` records carry the originating
`packet_id + mpu_sequence_number` scope. A numeric `subt://` index is never
treated as a global MPT packet ID: if a bounded same-MPU subsample is present,
the association is `same-mpu-evidence` and points to its raw-resource record;
otherwise it remains explicitly `unresolved`. `dump-tlv` additionally emits complete bounded non-`stpp`
MPU/MFU payloads as `mmt_asset_payload` raw evidence with a deterministic
scope key. Such records may include a `format_hint`, but it is only a bounded
binary-signature or bounded-header observation (not a decode or rendering
claim), while unknown asset semantics remain unresolved. PNG dimensions and
font table counts, when present, are structural metadata only. Small,
structurally complete PNG resources may also carry a capped `data:` preview
value for a future native preview surface; the backend still does not decode
or trust arbitrary resource URLs.

The snapshot also carries a `renderProfile`. Its contract is deliberately
libaribcaption-compatible: use the bundled `Rounded M+ 1m for ARIB` family,
preserve character-cell geometry, keep ruby at a 0.5 relative scale, and take
background alpha and stroke colour from the decoded source character data.
The published libaribcaption screenshot is the viewer-facing visual reference;
its pinned local baseline and review rules are in `docs/visual-reference.md`.
The B24 portion of this profile is decoder-backed. The current native TTML path
uses the bundled font, source foreground/background RGBA, span style runs,
simple horizontal ruby, and explicitly associated vertical ruby, including a
bounded continuation across automatic columns. Complex ruby grouping, complete
vertical orientation, and standard stroke behaviour remain declarative metadata until their native
implementations are tested; the UI must not imitate them with arbitrary CSS
shadows or fixed black boxes.
`captionOverlayModes` is an array of structured backend route capabilities:
`id`, `available`, `experimental`, and `unavailableReasonCode`. On Windows,
`libmpv-render` becomes available when the discovered runtime exports the full
render API; the backend selects it by default and falls back per source to
`libmpv-client-overlay` if render-worker startup fails. The UI presents the
backend's actual route and never selects a renderer itself.

## Worker event envelope

Worker JSONL events use `protocolVersion`, `jobId`, `sequence`, and `payload` fields. For compatibility, the legacy top-level event fields remain present during the migration. The Tauri layer must validate the version and sequence before forwarding events to Svelte.

The worker emits `hello` first, followed by bounded `stage-changed`, `track-discovered`, progress, `diagnostic`, `drcs-discovered`, pause/resume, cancellation, `artifact-created`, completion, or `failed` events as applicable. Every successfully published artifact is reported with its stable kind and final path before completion; Tauri consumes that event to update the atomic `app-data/jobs/{job-id}/artifacts.json` manifest rather than inferring final artifacts from UI options. Checkpoint persistence belongs to Tauri: only after `checkpoint.json` is atomically published does it forward `checkpoint-written`. Tauri forwards a stable `code` and `parameters` shape on every task event. Timeline and diagnostic pages stream their JSONL source and retain only the requested window; the desktop does not cache the complete archive or diagnostic history in memory. The live time-window API keeps one bounded prefetched window and advances a byte cursor over newly completed JSONL lines, rebuilding from disk only when the requested time leaves that window or the artifact is replaced. Protocol-version and sequence violations retain their raw message as evidence but also carry named parameters such as `expected`, `actual`, `previous`, and `current`; Svelte localizes the code without parsing that message. Worker-provided diagnostic parameters are preserved verbatim when they are JSON objects. On cancellation or failure, artifact status is reconciled from Worker events and file evidence: `completed` means the Worker published it, `preserved` means a pre-existing target remains untouched, and `incomplete` means a `.part` file remains. `failed` or `cancelled` means no stronger artifact evidence exists. On application startup, persisted active states become `Interrupted` and a persisted `Queued` task becomes `Ready`; the in-memory queue never resumes itself. `resume_job` only replays `Interrupted`, `Failed`, or `Cancelled` jobs after verifying job ID, source, output, track, source size, progress bound, and a bounded head/tail source fingerprint. A timestamp-only change is reported but accepted when size and fingerprint still match. Native decoder and partial-artifact state are not serialized, so recovery currently performs a full replay from the trusted recording origin rather than claiming byte-exact resume.

The worker is independently executable and must be tested before UI integration:

```text
arib-caption-worker inspect recording.ts
arib-caption-worker convert recording.ts output.ass --overwrite --drcs-report
arib-caption-worker convert recording.m2ts output.ttml --ttml --overwrite
arib-caption-worker dump-tlv recording.tlv output.caption.mmtp.jsonl --overwrite
arib-caption-worker render-at output.caption.archive.jsonl 90000
```

Known limitations are product constraints, not hidden fallbacks:

- SRT is not a formal lossless target.
- Unrecognized TLV/MMTP assets are kept as raw evidence and are not guessed.
- `inspect_source` returns a stable `routeCode`: `mpeg_ts_b24_verified` is
  immediately verified by a B24 component descriptor. `mpeg_ts_ttml_candidate`
  means that private PES PIDs were found in either 188-byte TS or 192-byte M2TS
  and still requires strict ARIB-TTML XML validation during conversion.
  `mpeg_ts_192_ttml_verified` names the release-gated, successfully validated
  192-byte M2TS/TTML conversion route; a bounded initial inspection must not
  claim it before it has seen a valid TTML document.
  `tlv_mmtp_experimental` is intentionally evidence-first and must not be
  presented as general BS4K/8K support without a real corpus.
- Checkpoints currently perform a source-identity-verified full replay from the trusted recording origin because native B24 and partial-artifact state are not serializable.
- The current Windows video surface is owned by in-process `libmpv`; no `mpv.exe` sidecar or JSON named pipe is used. Where the runtime exports the complete render API, the backend selects `libmpv-render`, owns the WGL context and BGRA texture blend path, and falls back to client overlay only if that specific startup fails. It requests `hwdec=auto-safe`, allowing compatible copy-back acceleration but not promising zero-copy D3D/ANGLE interoperability. `get_preview_render_diagnostics` returns the selected route, live surface dimensions, presents-per-second, texture operation counts, aspect, requested decoder policy, and libmpv's actual `hwdec-current` when the loaded source reports it. Long 2K/4K/8K profiling remains a release-quality gate rather than an implied capability.
- `get_preview_capabilities` reports each route as `{ id, available, experimental,
  unavailableReasonCode }`. It is a presentation contract only: the WebView
  cannot submit caption bitmaps. `render_preview_at` and
  `sync_preview_overlay` compose the bounded native caption plane inside the
  backend, then apply it to libmpv. Non-Windows builds report
  `preview.platform_not_implemented`; they do not imply a native preview route.
- `sync_preview_overlay` reports both `mediaTimeMs` and `projectTimeMs`. It queries captions using `projectTimeMs`; the default mapping is identity, but PTS repair, programme boundaries and user offsets must update the backend mapping rather than teaching the WebView a second clock.
- `trackId` is passed as a validated PID selector for all discovered MPEG-TS
  B24 or M2TS data tracks. For B24, the selected PID resolves to a logical
  `service_id + component_tag` track; sequential decoding follows current
  PAT/PMT updates and may continue on a replacement PID for that same logical
  track. Inspection reports the representative `caption_pid`, every bounded-
  discovery `caption_pids`, the component tag, PAT/PMT service IDs, SDT service
  names and ISO-639 caption languages. Its `broadcast` object additionally
  reports optional NIT network name, current-service EIT present-event name and
  description, and TDT/TOT UTC broadcast time. This SI pass is content-based,
  streams at most 64 MiB with a one-packet working buffer, and never substitutes
  another service's programme when the selected service has no EIT. Missing
  fields mean that the recording did not provide the information in the bounded
  window; they are not parser guesses. Broad EPG history, CAS and recorder
  metadata remain outside the product contract. The queue supervisor owns pause
  state and sends cooperative pause/resume controls to its active Worker; idle
  pause still prevents the next queued job from starting.

Private-PES track discovery reports `pids`, `caption_pids`, and
`superimpose_pids`. Component tags `0x30..0x37` and `0x38..0x3f` classify the
two services, but do not by themselves prove B24 or TTML: B24 still requires
its data-component descriptor, while TTML still requires a complete,
strictly-decoded XML document. With no explicit `trackId`, conversion and
preview select declared caption components and keep superimpose components
independent. If PMT descriptors do not classify a private stream, it remains a
candidate rather than being guessed from its PID.

Namespace-conformant TTML is parsed by XML local name and ancestry, including
default or prefixed TTML namespaces. Sequential ARIB-TTML documents may omit
`begin`, `end`, and `dur`; the next complete document on the same PID closes
the previous document, and an empty `<tt>` is a clear operation. A 192-byte
M2TS route derives this document clock from the 30-bit arrival timestamp with
wrap handling when PES PTS marker/prefix validation fails. It never accepts a
zero-filled private PES field merely because `PTS_DTS_flags` was set, and it
never closes one PID from a document arriving on another PID.

# Tauri API contract

The Svelte application is a client of the Rust application layer. It does not
parse TS/TLV, decode ARIB, render video, or decide conversion semantics.

The public command surface is listed in [`../backend-contract.md`](../backend-contract.md).
This page groups those commands by responsibility:

- inspection and export: `inspect_source`, `start_export`, `cancel_export`,
  `pause_export`, `resume_export`;
- persisted jobs and recovery: `create_job`, `list_jobs`, `get_job`, job
  control, diagnostics, artifacts, checkpoints, and queue control;
- preferences and DRCS: settings, language packs, and DRCS report loading;
- preview and timeline: native preview control, archive rendering, playback
  mapping, and bounded timeline windows.

Commands must return bounded data and stable error codes. The UI must not infer
artifacts from options or fabricate capabilities unavailable from the backend.

## Surface freeze

The current command surface is in a convergence phase. New one-off variants of
existing queries should not be added while the underlying model is settling.
When timeline queries next need consolidation, prefer one parameterized
`query_timeline` request (with an explicit mode/time range/filter) over adding
more `get_timeline_*` commands. Any such migration must preserve bounded
responses, archive cursor semantics, stable error codes, and a coordinated
frontend contract update.

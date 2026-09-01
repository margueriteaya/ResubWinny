[简体中文](worker-protocol.md) · [繁體中文](worker-protocol.zh-TW.md) · [日本語](worker-protocol.ja.md) · [English](worker-protocol.en.md)

> This is a translation. The Simplified Chinese version is the sole authoritative source.

# Worker protocol contract

Worker messages use `protocolVersion`, `jobId`, `sequence`, and `payload`.
Legacy top-level fields remain during migration. The worker emits `hello`
first, then bounded stage, track, progress, diagnostic, artifact, completion,
or failure events as applicable.

Tauri validates protocol version and sequence before forwarding events. Raw
messages are retained as evidence when validation fails, alongside structured
`expected`, `actual`, `previous`, or `current` parameters. Artifact status is
derived from worker events and file evidence; the UI never guesses completion.

The worker owns probe/demux/decode, Caption IR, export, archive, and evidence.
Job history, queue state, checkpoints, settings, and window lifetime remain in
the Tauri application layer.

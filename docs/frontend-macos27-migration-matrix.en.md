# macOS 27 Frontend Migration Matrix

[简体中文（唯一权威）](frontend-macos27-migration-matrix.md) | [English](frontend-macos27-migration-matrix.en.md) | [日本語](frontend-macos27-migration-matrix.ja.md) | [繁體中文（台灣）](frontend-macos27-migration-matrix.zh-TW.md)

> This is a translation of `frontend-macos27-migration-matrix.md`. The Simplified Chinese document is the sole authoritative specification; the English, Japanese, and Traditional Chinese (Taiwan) versions are translations only. Any ambiguity or conflict is resolved exclusively by the Simplified Chinese document.

This document is the delivery contract for the ResubWinny frontend rewrite. It
freezes the product surface while the visual implementation is migrated. A
screen is complete only when its existing behavior, copy, states, accessibility,
and performance gates all pass in the real Tauri application.

## Source Priority

1. Apple Human Interface Guidelines for behavior and material placement.
2. The official Apple macOS 27 UI Kit for control measurements and states.
3. The approved Figma frames, including `Task State / Timeline Indexing / 1280x820`.
4. The existing application for features, copy, data, and information structure.
5. External references for optical implementation research only.

No unapproved logo, brand color, content feature, or alternate information
architecture is part of this migration.

## Platform Rules

- The titlebar has no title text. The titlebar and sidebar read as one material.
- Window controls use `macos-traffic-lights` and retain their minimum layout area.
- Production controls map to official UI Kit variants; no visual-only variants are
  added without a matching product need.
- UI text references installed SF Pro first and uses region-appropriate system
  fallbacks. Fonts embedded in the Apple UI Kit are not redistributed.
- Apple-restricted symbols are not redistributed in the Windows package. Windows
  assets preserve the approved semantic, stroke, and optical-size mapping.
- Liquid Glass is limited to navigation and control layers. Video, timelines,
  tables, forms, logs, and other content surfaces remain solid.
- The native mpv surface and its ancestors never receive `filter`,
  `backdrop-filter`, `transform`, or WebView transparency.
- Visual acceptance uses the packaged Tauri/WebView2 application, not a browser
  development page.

## Application Shell

| Surface | Existing behavior that must remain | Required states | Migration gate |
| --- | --- | --- | --- |
| Window chrome | drag, eight resize directions, close, minimize, maximize/restore | normal, hover, active, narrow window | Traffic lights and toolbar controls remain centered and uncompressed |
| Sidebar | Home, Tasks, Batch, DRCS, Settings, current task, busy state, task count | expanded, collapsed, responsive collapse, light, dark | No seam with titlebar; responsive state does not overwrite saved layout |
| Navigation | lazy route loading and preview-safe route changes | idle, loading, selected, keyboard focus | Native preview is stopped or rebound according to the existing lifecycle |
| Status | application and job progress information | idle, running, paused, error, complete | Copy and live-region behavior remain intact |

## Home

| Feature | Existing behavior that must remain | Required states |
| --- | --- | --- |
| Open source | file picker and drag/drop entry | idle, drag over, inspecting, error |
| Recent tasks | task name, route/container metadata, state, reopen action | empty, populated, long names, compact width |
| Supported formats | current output formats and descriptions | complete locale copy, long labels |
| Decoded caption types | current ARIB/TTML capability presentation | available and unavailable capability information |

## Task Workspace

| Feature | Existing behavior that must remain | Required states |
| --- | --- | --- |
| Source inspector | file/container/packet metadata, broadcast metadata, caption tracks, track selection | no task, inspecting, no tracks, selected, selection disabled |
| Workspace layout | source width 220-320, output width 280-380, pointer and keyboard resizing, saved collapse state | desktop, compact overlay, source/output collapsed |
| Preview tabs | Preview, Events, Diagnostics | selected, keyboard focus, counts, loading |
| Native preview | start/stop, resize geometry, unavailable fallback, current subtitle overlay | stopped, starting, playing, paused, unavailable, error |
| Playback | play/pause, seek, volume, current/duration time | indexing, ready, playing, paused, end of media |
| Time mapping | existing mapping modes and save command | loading, selected, saving, error |
| Timeline | paged event loading, current filter semantics, zoom, independent scrolling, draggable playhead, keyboard seek | empty, indexing, populated, filtered, loading more, live updates |
| ARIB event visuals | Ruby, ARIB gaiji, DRCS, color, accessibility and other decoded traits | single trait, multiple traits, unknown text, long text |
| Diagnostics | paged diagnostic records with stable keys | empty, loading, populated, exhausted, error |
| Output formats | all current `ExportFormat` choices and format descriptions | unchecked, checked, disabled, lossy warning |
| Preservation | position, color, ruby, DRCS, gaiji, accessibility and combined toggle semantics | checked, unchecked, disabled |
| Output location | editable path and directory picker | empty, selected, invalid/error |
| Export | start, progress, pause/resume/cancel where exposed by the controller, completion and failure | idle, indexing, exporting, paused, cancelled, complete, error |
| Checkpoint | detect and resume persisted job checkpoint | unavailable, available, resuming, error |
| Log | current live log entries and automatic scroll | empty, streaming, long lines |

## Batch

| Feature | Existing behavior that must remain | Required states |
| --- | --- | --- |
| Queue input | add files and preserve file order | empty, populated, duplicate/error |
| Queue management | selection, removal and per-item state | pending, running, complete, failed, cancelled |
| Batch configuration | preset, output directory, formats and preservation options | default, edited, invalid |
| Batch execution | start, aggregate progress, pause/resume/cancel where exposed | idle, running, paused, complete, partial failure |

## DRCS

| Feature | Existing behavior that must remain | Required states |
| --- | --- | --- |
| Dictionary tabs | automatic and user mappings | selected, keyboard focus |
| Filtering | all, mapped and review states | empty, filtered, populated |
| Mapping editor | character, image and font-glyph actions | unselected, selected, validation error, saved |
| Dictionary persistence | load and update existing mappings | loading, saving, error, complete |

## Settings

| Feature | Existing behavior that must remain | Required states |
| --- | --- | --- |
| Appearance | installed language packs, open language directory, system/light/dark theme | loading packs, selected, missing pack, error |
| Typography | interface fallback profile and caption font preview | system, CJK fallback, ARIB caption font |
| Output defaults | default export format | default and edited |
| Player | runtime availability, render API status and detail | ready, unavailable, partial capability |
| Persistence | immediate preview, automatic persistence and category reset | saving, saved, reset, error, rapid consecutive edits |
| About | product identity, running version, build provenance, distribution tier and signing declaration | development build, unsigned Alpha, missing optional provenance |
| Licenses | project license, distribution notice, bundled component licenses and generated dependency inventory | loading, searchable list, selected document, unavailable resource, offline |

Settings category navigation is a navigation layer and may use Liquid Glass.
Setting groups, previews, runtime details, About metadata, license search,
license lists, notices, and full license text are content surfaces and remain
opaque. A page must not add glass simply to make a card visually prominent.
Interactive controls use the shared Liquid Glass control treatment; content
containers use the existing solid surface and border tokens.

## Shared Control Inventory

| Production component | Official reference | Required behavior |
| --- | --- | --- |
| Toolbar icon button | Titlebars and Toolbars | 36x36 toolbar control, centered optical icon, tooltip, visible focus |
| Button group | Titlebars and Toolbars | 28x28 units where the official toolbar variant is used |
| Pop-up button and menu | Pop-up and Pull-down Buttons; Button and Menu | keyboard navigation, outside click, animated menu, no native Windows selector |
| Segmented control | Segmented Controls; Toolbars | toolbar and regular variants, sliding selection, complete ARIA state |
| Slider | Sliders; Toolbars | keyboard adjustment, pointer capture, glass thumb, numeric ARIA |
| Switch | Toggles - Switches | checked, unchecked, disabled, focus, reduced motion |
| Checkbox | Buttons/Toggles reference | sharp checkmark, mixed/checked/unchecked semantics |
| Split pane | App workspace adaptation | separator role, ARIA values, pointer and arrow-key resize |
| Tooltip/popover | Toolbars; Button and Menu | correct anchor, focus handling, escape/outside dismissal |

## Promotion Gates

Every vertical slice must pass all four gates before work expands:

1. **Contract:** existing commands, copy, states, and four locales remain present.
2. **Visual:** same-viewport comparison against the official Kit and approved Figma.
3. **Interaction:** keyboard, pointer, focus, reduced motion, and compact layout pass.
4. **Runtime:** real Tauri/WebView2 validation with no regression to native preview.

For Settings, About, and Licenses, visual review additionally verifies that
glass appears only on category navigation, menus, and controls; readable content
never receives `filter` or `backdrop-filter`; category labels remain visible at
regular widths; and compact layouts do not turn the category list into a button
wall. Forced-colors, static-glass, and reduced-motion fallbacks are release
requirements, not optional polish.

Final performance targets are panel and glass interaction p95 below 20 ms, no
frontend long task above 50 ms during interaction, no idle animation frame loop,
initial gzip JavaScript growth below 10 percent, and CSS gzip at or below 15 KB.

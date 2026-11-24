# fennec-ui

Shared [gpui-kit](https://crates.io/crates/gpui-kit) pieces for the Fennec shells: theme,
markdown, timeline grouping, composer controls, the inspector model, and the dialogs both
native surfaces open.

Working version **1.0.0**. This repository has no prior published release or tag; nothing at 1.0.0 is published yet.

## Why this crate exists

The desktop shell and the coding-agent shell draw the same conversation. Before this crate
each one had its own copy of the timeline grouping, the risk colours and the markdown
splitter, and they drifted: the PWA folded a tool row at a different width than the native
app, and the two shells counted a run's reads and writes differently. Everything here is a
piece more than one repo needs, so it lives once. App state, backend dispatch and
persistence stay in the app — this crate renders, it does not own a session.

The web PWA in `fennec-coding-agent/web` is plain HTML, CSS and JavaScript and cannot import
Rust, so it mirrors this vocabulary in its own code; the parity is asserted by the PWA lane
rather than by a shared type.

## What lives here

| Module | Provides |
|---|---|
| `timeline.rs` | groups tool calls into one block per stretch between answers, folds long rows to a basename or seven words, prices the block header (`Ran n tools · read n files · wrote n files`), progress lines and the turn clock |
| `attention.rs` | the My Attention model: approval decisions, risk parsing and colours, tool-name display, the call blob a row expands to |
| `widgets.rs` | message cards, the header row, queue strip, action/attachment/edit cards, the composer dock and toolbar, TTS speak, the brand fox, shared truncation |
| `markdown.rs` | the rich-block splitter: fenced code, inline spans that must not break into columns, artifact badges, attachment subtitles, diff counts |
| `controls.rs` | the permission and model dropdowns, reasoning-effort levels, and the WAV encode + POST that hands dictation samples to core |
| `inspector.rs` | right-panel sections, the default-tool set hidden from the Skills list, the command tools that get an approval card, link extraction |
| `repository.rs` | the "Working in" GitHub panel and the repository-attach dialog, with inline repo creation |
| `theme.rs` | `Palette` and `ThemePreset`, the preset list and index lookup both surfaces read |
| `icons.rs` | `FennecAssets` — the PWA's line icons carried as embedded SVG so a shell packs no icon files, and `ui_icon` to tint one |
| `plugin.rs` | the `Plugin` record and the registry every capability is declared in |
| `confirm.rs` | the one confirmation dialog shape, used for deletes and destructive writes |
| `workspace_dialog.rs` | the workspace-creation dialog: name, source folders, emoji icon grid, colour grid, index toggle |
| `lib.rs` | module exports |

Theme presets: 7 palettes in `theme.rs`, each carrying its own background, muted and border colours, so the desktop and the PWA offer the same list. Plugin registry: 7 built-in capabilities in `plugin.rs`, gated by id rather than by layout code.

## Constraints the shells depend on

- Dropdowns are rendered at the root, not inside the card that opens them: gpui-kit's
  `overflow_hidden` clips an absolutely-positioned child, so a dropdown inside a scrolled
  panel is cut off.
- Rows built in a loop carry an explicit `.id()`. An id-less `Label` takes its call site as
  its id, and two identical call sites in one loop is a duplicate accessibility id — a debug
  build aborts on it.
- Nothing here truncates text to make a layout fit; the callers pass a width and the shared
  widget shortens with an ellipsis only where the row is folded by design.

## Tests

The lane measures 44 tests written inline in 13 modules — 28 in `timeline.rs`, 9 in `attention.rs`, 4 in `inspector.rs`, 2 in `controls.rs`, 1 in `icons.rs`. `fennec-desktop` compiles them but never runs them, so the crate needs its own lane.

```
cargo test --offline
```

Debug, not release: `fennec-ui` links gpui-kit, whose release HLSL pass needs `fxc.exe`,
which is absent on this machine.

## Used by

- `fennec-desktop` — the native gpui shell

## Part of the Fennec suite

[environment-setup](https://github.com/Liso7150/fennec-environment-setup) installs and keeps
every part of the suite current, including this crate.

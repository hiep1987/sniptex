---
phase: 2
title: "Isolated Gemini Workspace Staging"
status: complete
priority: P1
effort: "0.5d"
---

# Phase 02: Isolated Gemini Workspace Staging

## Context Links

- [OCR dispatcher](../../src-tauri/src/ocr/dispatcher.rs)
- [History rerun command](../../src-tauri/src/commands.rs)
- [Gemini CLI rerun note](../../260523-gemini-cli-rerun-fix.md)

## Overview

Replace `current_dir($HOME)` for Gemini CLI with an isolated temporary workspace that contains only the staged image for this OCR call.

## Key Insights

- `$HOME` fixes "image path outside workspace" but gives Gemini too much contextual surface.
- A clean temp workspace reduces accidental memory/context discovery and makes failures easier to reason about.
- The image path passed to Gemini should be inside its current directory.

## Requirements

- Create a per-call temp directory under `std::env::temp_dir()/sniptex/gemini-workspaces/{uuid}`.
- Copy the source image into that workspace, preserving extension when possible.
- Run Gemini with `current_dir(workspace)`.
- Pass the staged image path to Gemini, not the original app data path.
- Clean up the staging directory on every exit path.

## Architecture

```text
history image path
  -> copy to temp/sniptex/gemini-workspaces/{uuid}/image.png
  -> gemini cwd = temp/sniptex/gemini-workspaces/{uuid}
  -> prompt references @"image.png" or absolute staged path
  -> cleanup workspace
```

## Related Code Files

- Modify: `src-tauri/src/ocr/dispatcher.rs`
- Optional create: `src-tauri/src/ocr/gemini_workspace.rs`
- Add tests under: `src-tauri/tests/rust/`

## Implementation Steps

1. Add an RAII `TempDir` guard similar to the existing `TempFile` guard.
2. For Gemini CLI only, create workspace before building args.
3. Copy image into workspace as `input.<ext>`.
4. Build Gemini args using staged path.
5. Set `cmd.current_dir(&workspace)`.
6. Remove the old `$HOME` current-dir workaround.

## Todo List

- [x] Add temp-dir guard.
- [x] Stage image for Gemini only.
- [x] Set Gemini cwd to staging dir.
- [x] Remove `$HOME` cwd workaround.
- [x] Add tests for staged path naming and cleanup behavior where feasible.

## Completion Notes

- Implemented 2026-05-24.
- Added `ocr/gemini_workspace.rs` for per-call Gemini temp workspaces under `temp/sniptex/gemini-workspaces/{uuid}`.
- Gemini CLI now receives a workspace-relative `input.<ext>` image reference while its process `current_dir` is the isolated workspace.
- Removed the previous `$HOME` current-dir workaround.
- Added cleanup on normal drop and on image-copy failure before the guard is constructed.
- Codex path remains unchanged.
- Reviewer/debugger re-check found no remaining Phase 02 blockers.

## Validation

- `cargo test --manifest-path src-tauri/Cargo.toml ocr::gemini_workspace --lib` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --test agent_registry_argv --test gemini_cli_output` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- `pnpm exec tsc --noEmit` passed.
- `cargo fmt` was not run because `rustfmt` is not installed for the active toolchain.

## Success Criteria

- Gemini can read rerun images from history without using `$HOME` as workspace.
- Tool-loop path errors are reduced because workspace is intentionally small.
- Temp staging does not leak files after timeout or failure.

## Risk Assessment

- **Risk:** Copying images adds overhead.
  **Mitigation:** Snip images are small; overhead is negligible versus 15-60s CLI latency.
- **Risk:** Gemini still tries unrelated tools.
  **Mitigation:** Phase 03 rejects tool-call/error outputs.

## Security Considerations

- Do not place user secrets in staging workspace.
- Use a unique directory per call to avoid cross-request data exposure.

## Next Steps

- Phase 03 adds explicit guards so unsafe Gemini outputs cannot update history records.

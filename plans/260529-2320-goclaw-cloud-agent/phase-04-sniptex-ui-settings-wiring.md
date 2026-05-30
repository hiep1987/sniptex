---
phase: 4
title: "SnipTeX UI + settings wiring"
status: pending
priority: P1
effort: "2h"
dependencies: [3]
---

# Phase 4: SnipTeX UI + Settings Wiring

## Overview

Surface the Phase 3 adapter as a user-visible agent: Settings → Agents shows a "Goclaw" row with API key entry, fallback chain includes `cloud-goclaw`, PDF dispatcher recognizes it, and the per-page PDF timeout selection learns one CLI-class exception so gpt-5.4 doesn't time out at the cloud-default 30s. After this phase the user pastes the Phase 2 key once and Goclaw appears in priority list, ready for snip/PDF flows.

## Requirements

- Functional: Settings → Agents tab shows "Goclaw OCR Agent" with Set/Update/Remove key affordances, identical to existing cloud-gemini / cloud-mistral rows.
- Functional: `set_api_key` / `has_api_key` / `delete_api_key` commands accept provider id `"goclaw"`.
- Functional: PDF dispatch routes `cloud-goclaw` through `run_per_page_pdf_ocr` (same as cloud-gemini, since each page is sent as one image — Goclaw doesn't accept multi-page PDFs in `chat.send.media`).
- **Functional (NEW from Phase 3 dependency)**: per-page PDF budget for `cloud-goclaw` is 120s (matches `PDF_CLI_PAGE_TIMEOUT`), not the 30s cloud default. gpt-5.4's OCR latency is CLI-class (~80s/page on full-resolution PDF pages) — same provider as local codex CLI, just routed differently.
- Functional: cloud-goclaw appears in default fallback chain (placement decided in step 3 below).
- Non-functional: link to user's Goclaw admin UI for getting / rotating the API key.

## Architecture

Four surfaces touched:

1. **`commands.rs::set/has/delete_api_key`** — three small match-arm additions for provider id `"goclaw"`. Mirror the gemini/mistral arms exactly.

2. **`commands.rs::run_per_page_pdf_ocr`** — the per-page timeout selection is currently:
   ```rust
   let per_page = match agent.spec.kind {
       AgentKind::CliBin   => ocr::PDF_CLI_PAGE_TIMEOUT,         // 120s
       AgentKind::CloudApi => Duration::from_secs(30),           // 30s
   };
   ```
   Extend to recognise the agent **id** as a second axis — `cloud-goclaw` is `CloudApi` kind but CLI-class latency:
   ```rust
   use crate::agents::registry::CLOUD_GOCLAW_ID;
   let per_page = match (agent.spec.kind, agent.spec.id) {
       (AgentKind::CliBin, _)             => ocr::PDF_CLI_PAGE_TIMEOUT,    // 120s — codex / gemini-cli
       (AgentKind::CloudApi, CLOUD_GOCLAW_ID) => ocr::PDF_CLI_PAGE_TIMEOUT, // 120s — gpt-5.4 over Goclaw
       (AgentKind::CloudApi, _)           => Duration::from_secs(30),     // 30s — gemini / mistral cloud
   };
   ```
   This is the ONLY place the carve-out lives — `dispatch_pdf_ocr` routing stays untouched (cloud-goclaw falls into the default `run_per_page_pdf_ocr` branch alongside cloud-gemini and CLI agents). cloud-mistral keeps its native multi-page PDF endpoint via the explicit `if agent.spec.id == CLOUD_MISTRAL_ID` early-out.

3. **`agents-tab.tsx`** — add `cloud-goclaw` entry to `CLOUD_PROVIDERS` map and to `ALL_KNOWN`. Re-introduce a `providerKeyFor(id)` helper that maps `cloud-goclaw → "goclaw"`. Extend the `keyStates` scan loop to include `"goclaw"`.

4. **Onboarding (deferred, NOT in this phase)** — leave the onboarding cloud-key step unchanged. Goclaw is power-user territory; users discover it via Settings, not first-run.

## Related Code Files

**Work context: `/Users/hieplequoc/Projects/sniptex`**

- Modify: `src-tauri/src/commands.rs`
   - Three match arms in `set_api_key` / `has_api_key` / `delete_api_key` (provider `"goclaw"`).
   - One `match` rewrite in `run_per_page_pdf_ocr` per Architecture point 2.
- Modify: `src/windows/settings/agents-tab.tsx` — provider map + `providerKeyFor` helper + scan loop + `ALL_KNOWN`.
- (No change needed) `src-tauri/src/agents/registry.rs::DEFAULT_FALLBACK_CHAIN` — already includes `CLOUD_GOCLAW_ID` from Phase 3.

## Implementation Steps

1. **Backend — provider id wiring (`commands.rs`):** add `"goclaw" => keychain::set_cloud_goclaw_api_key(&key)…` (and matching `has` / `delete` arms calling the accessors added in Phase 3). Mirror gemini/mistral arms exactly.
2. **Backend — per-page timeout carve-out (`commands.rs::run_per_page_pdf_ocr`):**
   - Import `CLOUD_GOCLAW_ID` from `crate::agents::registry`.
   - Replace the `let per_page = match agent.spec.kind { … }` block with the 3-arm `match (kind, id)` shown in Architecture point 2.
   - Re-check the budget-exceeded error message — it currently reads `"PDF OCR exceeded budget of Ns (M pages × {per_page}s)"`. Already parameterised on `per_page` so no further edit needed.
3. **Frontend (`agents-tab.tsx`):**
   - Add to `CLOUD_PROVIDERS`:
     ```ts
     "cloud-goclaw": {
       keyLabel: "Goclaw API key",
       getKeyUrl: "https://goclaw.tikz2svg.com/dashboard/api-keys"  // confirm exact admin path during impl
     }
     ```
   - Add `cloud-goclaw` to `ALL_KNOWN` between `cloud-mistral` and `gemini-cli`.
   - Re-introduce a `providerKeyFor(id)` helper handling the three cloud providers (gemini / mistral / goclaw). Reuse the pattern from the reverted cloud-vision attempt (`agents-tab.tsx` history in commit `e8771d6` then `3a4f3b6`).
   - Add `"goclaw"` to the `keyStates` scan loop.
   - Update the `ApiKeyInput` placeholder to hint the expected `goclaw_` prefix so users don't paste an `AIza...` Gemini key by mistake.
4. **Decide fallback priority placement.** Default chain after Phase 3 will be:
   ```
   [codex, cloud-gemini, cloud-mistral, cloud-goclaw, gemini-cli]
   ```
   Goclaw lives after directly-billed cloud providers — fine for v1 since users explicitly opt in. Users who prefer Goclaw first reorder via Settings drag.
5. **TypeScript** — `npx tsc --noEmit` clean.
6. **Cargo check + test** — full suite, no regressions. Special attention: confirm existing PDF tests still pass with the new 3-arm timeout match.
7. **Manual smoke** — paste API key in Settings, click "Test agent" (existing button in agents-tab), confirm a snip on a small math screenshot returns LaTeX. Then test the PDF flow with `test-1.pdf` (the same fixture used to validate prior cloud-gemini PDF fix). Expect ~80-120s/page latency.

## Success Criteria

- [ ] Settings → Agents shows the Goclaw row with proper status badge ("Key set" / "No key").
- [ ] Setting / updating / removing the key persists across app restart (verified via keychain JSON inspection).
- [ ] Cloud-goclaw appears in priority list and is reorderable.
- [ ] PDF flow with cloud-goclaw produces multi-page LaTeX output (verified against `test-1.pdf`), completing under `pages × 120s`.
- [ ] Rerun-from-history with cloud-goclaw works on both image and PDF records.
- [ ] Existing PDF flows with cloud-gemini and cloud-mistral are unaffected (regression check — the 3-arm `match` must not accidentally widen their budget or narrow Goclaw's).

## Risk Assessment

- **Risk**: The `keyStates` map mixes provider keys (`gemini`, `mistral`, `goclaw`) and might collide with future providers using full ids.
  **Mitigation**: `providerKeyFor` is the single source of truth for the mapping. Any future provider id added to it stays consistent.
- **Risk**: User pastes a Gemini key (`AIza…`) into the Goclaw slot by mistake.
  **Mitigation**: front-end placeholder shows the expected `goclaw_` prefix. Backend `AuthFailed` maps cleanly to a user-facing "auth failed" toast — same UX as the gemini-vs-vision mix-up earlier in this project.
- **Risk (NEW)**: 3-arm `(kind, id)` match accidentally regresses cloud-gemini / cloud-mistral timeout (e.g., wildcard order wrong, wrong arm matches first).
  **Mitigation**: order arms specific-id BEFORE generic-kind, and add a unit test asserting the timeout selected for each `(kind, id)` combo: codex → 120s, gemini-cli → 120s, cloud-gemini → 30s, cloud-mistral → 30s (but mistral takes the native-PDF branch so this is moot for PDFs), cloud-goclaw → 120s.

## Security Considerations

- Reuse existing `ApiKeyInput` component (masked input + show-toggle) — no new attack surface.
- `goclaw_xxx` keys redacted from logs by the Phase 3 adapter's `redact_key`.
- Settings UI never displays the key after save (only "Key set" badge + Update / Remove buttons).

## Next Steps

After merge: short `docs/cloud-goclaw.md` (or update `docs/codebase-summary.md`) explaining the agent, the keychain account name, how to rotate the key, and the 120s/page latency expectation. Bump `docs/project-changelog.md` with the integration. Out of scope for this phase but listed for the post-implementation `/ck:project-management` sweep.

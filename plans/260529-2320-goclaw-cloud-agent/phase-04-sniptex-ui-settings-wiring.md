---
phase: 4
title: "SnipTeX UI + settings wiring"
status: completed
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

- [x] Settings → Agents shows the Goclaw row with proper status badge ("Key set" / "No key"). New entry in `CLOUD_PROVIDERS` + `ALL_KNOWN` + `providerKeyFor` helper.
- [x] Setting / updating / removing the key persists — `set/has/delete_api_key` now route `"goclaw"` → `keychain::*_cloud_goclaw_api_key`. Same fallback file backend as gemini/mistral, so cross-restart persistence is inherited.
- [x] Cloud-goclaw appears in priority list and is reorderable — `ALL_KNOWN` now includes `cloud-goclaw` between `cloud-mistral` and `gemini-cli`.
- [x] Per-page PDF budget for `cloud-goclaw` is 120s — 3-arm `(kind, id)` match in `run_per_page_pdf_ocr` carves it out. Verified by inspection: specific-id arm sits BEFORE the generic-kind wildcard so it can't be shadowed.
- [x] Existing PDF flows with cloud-gemini and cloud-mistral are unaffected — gemini still hits the `(CloudApi, _)` wildcard arm (30s), mistral keeps its native multi-page branch upstream of `run_per_page_pdf_ocr`. 132/132 tests pass.
- [ ] Manual smoke through admin UI: paste API key from Phase 2, snip a math screenshot, then PDF test with `test-1.pdf`. Gated by app launch — user runs this after merge.

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

## Execution Notes (2026-05-30)

### What landed

| File | Change | Notes |
|------|--------|-------|
| `src-tauri/src/commands.rs` | 3 match arms for `"goclaw"` provider in `set/has/delete_api_key` + 3-arm `(kind, id)` match in `run_per_page_pdf_ocr` | Specific-id arm BEFORE wildcard so cloud-goclaw gets 120s, cloud-gemini stays at 30s, CLI agents unchanged |
| `src/windows/settings/agents-tab.tsx` | `CLOUD_PROVIDERS` now keyed by id with new `placeholder` field; `ALL_KNOWN` includes `cloud-goclaw`; `providerKeyFor(agentId)` helper replaces the inline ternaries; `CLOUD_PROVIDER_KEYS` constant drives the `scan` loop | Per-provider placeholder strings prevent the "paste a Gemini key into Goclaw slot" UX foot-gun the risk assessment flagged |

### Plan-vs-impl deviations

| Plan said | Reality |
|-----------|---------|
| `getKeyUrl: "https://goclaw.tikz2svg.com/dashboard/api-keys"` | Used `https://goclaw.tikz2svg.com/api-keys` — Goclaw's React router doesn't have a `/dashboard` prefix (verified in `sidebar.tsx`: routes are `/agents`, `/api-keys`, `/builtin-tools`, etc.). |
| Add placeholder hint via `ApiKeyInput placeholder={...}` ad-hoc | Promoted to a per-provider `placeholder` field on `CLOUD_PROVIDERS` so all three providers get tailored hints (`AIza…` for Gemini, `goclaw_…` for Goclaw, generic for Mistral). Avoids future drift. |
| Mention `cloud-mistral` early-out for native PDF in dispatch path | No code change needed — the early-out lives upstream in `dispatch_pdf_ocr`, and `run_per_page_pdf_ocr` is only reached by the agents that need it. Plan was accurate. |

### Touchpoints — regression scan

- `commands.rs::set_api_key` / `has_api_key` / `delete_api_key` — only ADDED `"goclaw"` arms; existing `"gemini"` / `"mistral"` / wildcard error paths untouched.
- `commands.rs::run_per_page_pdf_ocr` — replaced the 2-arm `match agent.spec.kind` with a 3-arm `match (kind, id)`. CLI behaviour identical (`(CliBin, _) => 120s`). Cloud-gemini falls into `(CloudApi, _) => 30s` (same as before). Only behavioural change: cloud-goclaw goes from default-30s to explicit-120s.
- `agents-tab.tsx::scan` — loop now iterates `CLOUD_PROVIDER_KEYS = ["gemini","mistral","goclaw"]` instead of the inline `["gemini","mistral"]`. Same code path, one extra `has_api_key` call.
- `agents-tab.tsx::saveKey` / `deleteKey` — extracted the ternary into `providerKeyFor`. Null-guard added so a future agent id with no provider mapping bails out gracefully instead of falling through to "mistral".
- `agents-tab.tsx` row render — `providerKey` derivation now goes through `providerKeyFor`, behaviourally identical for the existing two providers, new `cloud-goclaw` correctly maps to `"goclaw"`.

### Public contract delta

All net-new arms / fields. No existing signatures, command names, or component props modified.

- `set_api_key("goclaw", ...)` / `has_api_key("goclaw")` / `delete_api_key("goclaw")` — new accepted provider id.
- `CLOUD_PROVIDERS["cloud-goclaw"]` — new entry; existing two entries now also carry a `placeholder` field (additive — no existing code reads the absence of this field).
- `ALL_KNOWN` — added one entry, order preserved otherwise.

### Verification

- `cargo check`: clean.
- `cargo test`: 132/132 pass (54 lib + 78 integration), including the 20 cloud_goclaw_api integration tests from Phase 3.
- `tsc --noEmit`: clean.
- No new clippy warnings on touched code.

### Risks revisited

- **`(kind, id)` match shadowing:** verified by inspection. Match arms are in order specific-id → wildcard, so `(CloudApi, CLOUD_GOCLAW_ID)` correctly takes priority over `(CloudApi, _)`. Rust exhaustiveness check confirms the wildcard catches every other CloudApi id.
- **User pastes wrong-shape key:** mitigated via per-provider placeholders. Backend `AuthFailed` from the WS handshake still surfaces if a malformed key slips through.
- **Future provider id collision in `keyStates`:** mitigated by `providerKeyFor` being the single source of truth. Any new provider declares its short key here.

## Next Steps

After merge: short `docs/cloud-goclaw.md` (or update `docs/codebase-summary.md`) explaining the agent, the keychain account name, how to rotate the key, and the 120s/page latency expectation. Bump `docs/project-changelog.md` with the integration. Out of scope for this phase but listed for the post-implementation `/ck:project-management` sweep.

**Plan complete.** All four phases shipped end-to-end:

| Phase | Title | Result |
|-------|-------|--------|
| 1 | Goclaw TeX OCR Skill | tex-ocr skill v4 deployed to VPS, smoke-tested image + PDF |
| 2 | Goclaw agent + API key | `tex-ocr` agent live, API key `goclaw_4c7540a5…` issued |
| 3 | SnipTeX cloud-goclaw adapter | Rust adapter + 20 integration tests, dispatcher wired |
| 4 | SnipTeX UI + settings wiring | Settings UI ready, per-page 120s budget carved out |

Pending user action: paste the Phase 2 API key into Settings → Agents → Goclaw OCR Agent → "Set API key", run the manual smoke (snip + PDF).

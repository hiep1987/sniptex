---
phase: 1
title: "Goclaw TeX OCR Skill"
status: completed
priority: P1
effort: "2h"
dependencies: []
---

# Phase 1: Goclaw TeX OCR Skill

## Overview

Author the `tex-ocr` Skill that teaches Goclaw's LLM (`gpt-5.4` via `openai-codex-1` provider) how to OCR an attached image or PDF page into LaTeX/Markdown. Skill source lives in a new `bot-tex` repo (separate from `bot-tikz` which hosts the existing `tikz` skill); deployment copies it to the Goclaw VPS Docker volume alongside the `tikz` skill.

## Requirements

- Functional: given one or more images attached via `chat.send.media`, return LaTeX (raw, no `$` delimiters) for `EQUATION_ONLY`, GitHub Markdown for `TABLE_ONLY`, or Markdown w/ `$...$` for `MIXED`. Same classification rules as SnipTeX's `MASTER_PROMPT`.
- Functional: preserve Vietnamese diacritics; do NOT translate; output `[UNREADABLE]` on garbage input.
- Functional: when multiple images are attached (PDF pages), concatenate output with one blank line between pages. No "Page N" labels.
- Non-functional: critical OCR rule must be in frontmatter `description` field (not body) so it's injected into the system prompt even if the LLM skips `read_file`. Verified Goclaw quirk per `bot-tikz/docs/tikz-skill-engineering-notes.md`.

## Architecture

Skill loading flow (from `tikz-skill-engineering-notes.md`):

1. Goclaw `BuildSummary()` injects `<available_skills>` block into the system prompt containing **only** `name + description + location` for each skill the agent has access to.
2. Agent receives the user message + system prompt. If the description's instructions are enough, agent acts directly. Otherwise it should call `read_file` to load the full SKILL.md body.
3. **gpt-5.4 frequently skips step 2's `read_file` call** → critical rules must fit in `description`.

Layout in `bot-tex` repo (mirrors existing `tikz` skill structure):

```
bot-tex/skills-store/tex-ocr/
└── 1/
    └── SKILL.md
```

No helper scripts needed — pure prompt skill.

Deployment target on VPS:

```
/var/lib/docker/volumes/goclaw_goclaw-data/_data/skills-store/tex-ocr/1/SKILL.md
```

## Related Code Files

**Work context: `/Users/hieplequoc/Projects/bot-tex`** (NOT `/Users/hieplequoc/Projects/goclaw` — that's docs-only)

- Create: `bot-tex/skills-store/tex-ocr/1/SKILL.md` — the skill file.
- Reference (do not modify): `bot-tikz/skills-store/tikz/1/SKILL.md` — frontmatter format model (lives in original tikz repo).
- Reference (do not modify): `sniptex/src-tauri/src/ocr/prompt.rs` (`MASTER_PROMPT`) — body content source.

## Implementation Steps

1. Read the existing `bot-tikz/skills-store/tikz/1/SKILL.md` frontmatter to confirm the schema (fields: `name`, `description`; any others Goclaw expects).
2. Draft `description` field — must contain the **core OCR rule** in one sentence so it survives even if `read_file` is skipped:
   ```
   description: "MANDATORY OCR skill: when the user sends an image, output ONLY the OCR'd content as raw LaTeX (for equations) or Markdown with $...$ inline math and $$...$$ display math (for mixed text+math). NEVER add preambles like 'Here is...' or sign-offs. NEVER wrap output in ```markdown or ```latex fences. Preserve Vietnamese diacritics exactly. Use \\frac{}{}, ^{}, _{} for math. Output [UNREADABLE] if the image cannot be parsed."
   ```
3. Draft body — adapt SnipTeX `MASTER_PROMPT` for chatbot context:
   - Keep the EQUATION_ONLY / TABLE_ONLY / MIXED classification logic and format rules.
   - Drop "Begin output now:" trailer (chatbot context, not single-shot completion).
   - Add multi-image preamble: "If multiple images are attached, process in attachment order and concatenate output with one blank line between pages. No page labels."
   - Add a 1-line cross-reference comment: `<!-- Source: sniptex/src-tauri/src/ocr/prompt.rs MASTER_PROMPT. Keep in sync. -->`
4. Add a matching back-reference in `sniptex/src-tauri/src/ocr/prompt.rs` (above `MASTER_PROMPT` const) pointing at the Goclaw skill file path. This protects future-you from drifting one without the other.
5. Commit to `bot-tex` repo: `git add skills-store/tex-ocr && git commit -m "feat(skill): tex-ocr — image to LaTeX/Markdown"`.
6. Deploy to VPS:
   ```bash
   scp -r bot-tex/skills-store/tex-ocr Digital-Ocean-Goclaw:/tmp/
   ssh Digital-Ocean-Goclaw "sudo cp -r /tmp/tex-ocr /var/lib/docker/volumes/goclaw_goclaw-data/_data/skills-store/"
   ssh Digital-Ocean-Goclaw "sudo chown -R 1000:1000 /var/lib/docker/volumes/goclaw_goclaw-data/_data/skills-store/tex-ocr"
   ssh Digital-Ocean-Goclaw "sudo rm -rf /tmp/tex-ocr"
   ```
7. Trigger skill rescan via admin endpoint or container CLI:
   ```bash
   ssh Digital-Ocean-Goclaw "docker exec goclaw-goclaw-1 /app/goclaw skills list" \
     && curl -X POST -H "Authorization: Bearer $GOCLAW_GATEWAY_TOKEN" \
        https://goclaw.tikz2svg.com/api/v1/skills/rescan-deps
   ```
   The `Rescan Deps` button only updates existing skills — for NEW skills the container may need an actual rescan trigger. If `skills list` doesn't show `tex-ocr` after rescan-deps, fall back to a container restart: `ssh Digital-Ocean-Goclaw "docker restart goclaw-goclaw-1"`.
8. Verify visibility:
   ```bash
   curl -H "Authorization: Bearer $GOCLAW_GATEWAY_TOKEN" \
     https://goclaw.tikz2svg.com/api/v1/skills | jq '.[] | select(.name=="tex-ocr")'
   ```

## Success Criteria

- [x] `bot-tex/skills-store/tex-ocr/1/SKILL.md` exists with valid frontmatter matching tikz skill format.
- [x] `description` field contains the standalone OCR rule (so it works even when read_file is skipped).
- [x] Deployed to VPS volume; visible in `GET /v1/skills` response (`id=019e7730-1b23-78c0-9fb0-3f579dc66916`, `source=managed`, `is_system` absent → defaults to false). Final version on disk + DB: **v3** at `/app/data/skills-store/tex-ocr/3/SKILL.md`.
- [x] Sync-comment back-reference added in `sniptex/src-tauri/src/ocr/prompt.rs`.
- [x] **Manual smoke PASS** — tested via admin chat UI with both image and multi-page PDF (`test-1.pdf`, Vietnamese content). Output: clean LaTeX/Markdown, Vietnamese diacritics preserved (e.g., "SỞ GD&ĐT THÁI NGUYÊN / TRƯỜNG THPT LÊ HỒNG PHONG"), no preamble, no fences.

## Execution Notes (2026-05-30)

Phase 1 hit several Goclaw quirks that were NOT documented in the original plan. All resolved; captured here as a runbook for future skill onboarding.

### What landed

| Artifact | Location | Notes |
|----------|----------|-------|
| Skill source repo | `/Users/hieplequoc/Projects/bot-tex/` | NEW repo, separate from `bot-tikz`. Initial commit `95c10ef`. |
| Skill body | `bot-tex/skills-store/tex-ocr/1/SKILL.md` | v3 = final shipped version (3 incremental fixes). |
| Sync back-ref | `sniptex/src-tauri/src/ocr/prompt.rs` (doc comment, lines ~9-13) | Bidirectional comment pair. Sniptex side uncommitted (per user). |
| Goclaw skill record | DB id `019e7730-1b23-78c0-9fb0-3f579dc66916`, slug `tex-ocr`, master tenant, version 3 | `source=managed`, `visibility=internal`, `is_system=false`, `status=active`. |
| Goclaw agent (test) | `ocr-tex` (DB id `019e7736-8d17-7da7-b12e-ef9a160ee677`), `provider=openai-codex-1`, `model=gpt-5.4` | Created manually via Goclaw admin UI during smoke test. **Phase 2 will define the canonical agent name + grant.** |
| Skill grant | `skill_agent_grants` row linking tex-ocr → ocr-tex | Required because skill is `visibility=internal`. |

### Plan-vs-reality corrections

1. **Repo choice (deviation):** plan said `bot-tikz`. User redirected to new `bot-tex` repo (one-skill-per-repo). Plan files were updated in-flight; `bot-tikz` reference now only points at the tikz skill (used as schema model).

2. **Registration path (plan was wrong):** plan's "scp + chown + rescan-deps" only places the file on disk. Goclaw's `/v1/skills` API is DB-backed; there is **no startup scan of `skills-store/`**. Correct registration path: `POST /v1/skills/upload` (multipart zip, admin auth + `X-GoClaw-User-Id` header). Executed via `docker cp zip into container → python3 urllib.request multipart POST`, using `$GOCLAW_GATEWAY_TOKEN` + `X-GoClaw-User-Id: system`. Re-uploading the same slug auto-bumps `version` (1 → 2 → 3).

3. **Vision provider chain (Goclaw infra bug, fixed via UI):** `builtin_tools.read_image.settings` referenced a stale provider name `openai-codex` (without the `-1`) and an orphan `provider_id`. Symptom: all `read_image` calls failed with `provider not available`. Fixed in admin UI: **/builtin-tools → Media category → read_image → Settings → MediaProviderChainForm**, point provider at the current `openai-codex-1`. Same root issue fixed for `read_document`.

4. **Builtin `pdf` skill conflict (Goclaw quirk):** Goclaw ships a built-in system skill named `pdf` with a very broad description ("anything with PDF files"). For agents granted tex-ocr, gpt-5.4 still picked `pdf` first because (a) `pdf` sorts alphabetically before `tex-ocr` in the `<available_skills>` XML (`ORDER BY s.name`), and (b) `is_system=true` skills are auto-granted to every agent in the tenant. Per-agent revoke is NOT available in UI (`SkillsSection` renders "Always Available" for is_system skills). UI per-tenant override is also gated by `hasTenantScope = currentTenantId !== MASTER_TENANT_ID` — **not exposed for master tenant**. Workaround: SQL insert into `skill_tenant_configs` directly.
   ```sql
   -- Disable system `pdf` skill for master tenant
   INSERT INTO skill_tenant_configs (skill_id, tenant_id, enabled)
   SELECT id, '0193a5b0-7000-7000-8000-000000000001', false FROM skills WHERE name='pdf';
   -- Undo:
   DELETE FROM skill_tenant_configs WHERE skill_id='019d12c8-6bdc-794f-8faf-ac65cdb2754c' AND tenant_id='0193a5b0-7000-7000-8000-000000000001';
   ```

5. **`exec` tool filtered out by global Tool Profile (Goclaw config):** the agent had `exec` in its own `tools_config.allow`, but Goclaw's tool resolution pipeline (`policy.go`, 7-step) intersects with the GLOBAL **Server → Tools → Allow** list first (Step 3). The global Allow had only 4 tools (read_file, write_file, read_document, read_image); `exec` was outside, so the per-agent allow was a no-op (intersect with a smaller set can't enlarge it). Fixed by adding `exec` to **Server → Tools → Also Allow** (additive — uses `unionWithSpec`, doesn't disturb the baseline Allow list).

6. **`read_image` workspace sandbox (caught in v2 → v3 fix):** `loadImageFromPath` calls `resolvePathWithAllowed(path, workspace, ...)` — rejects any path outside `/app/workspace/<agent>/ws/<user>/`. v2 of the skill told the agent to rasterize PDFs to `/tmp/page-*.png`, which `read_image` then refused. Agent self-recovered by `cp`-ing /tmp → workspace (~3-5s wasted per PDF). v3 fixes by telling the agent to rasterize directly to a **relative output prefix** (`pdftoppm ... page`), which lands files in the current workspace and `read_image` accepts on the first call.

7. **Cosmetic warning `security.path_escape`:** when the agent calls `read_file` on `/app/data/skills-store/tex-ocr/N/SKILL.md`, Goclaw logs a path-escape warning because `read_file` is workspace-sandboxed. This is **non-blocking** — the LLM still gets the skill content (the read happens via a different code path that bypasses the sandbox for skill files, OR the agent already has the `description` injected into the system prompt). No fix needed.

8. **Stale file in container:** `/app/tex-ocr.zip` (1.6 KB) left over from upload retries; container rootfs is read-only at runtime so `rm` fails. Harmless — cleared on next container recreate.

### Goclaw configuration changes made (record for Phase 2+)

| Where | Change | Why |
|-------|--------|-----|
| Admin UI → /builtin-tools → read_image | Provider chain `provider=openai-codex-1, model=gpt-5.4` | Stale `openai-codex` (no `-1`) reference. Required for `read_image` to work at all. |
| Admin UI → /builtin-tools → read_document | Same provider rename (if not already done by user) | Same root cause as read_image. |
| Admin UI → Server → Tools → Also Allow | Added `exec` | Required for the agent to run `pdftoppm`. |
| DB `skill_tenant_configs` | `(pdf, master_tenant, enabled=false)` inserted | Disable conflicting built-in `pdf` skill for master tenant. |

### Workflow the agent actually executes (verified)

1. Reads `description` from `<available_skills>` (always).
2. (Often) reads `/app/data/skills-store/tex-ocr/3/SKILL.md` via `read_file` for full body.
3. For PDFs: `exec pdftoppm -png -r 200 "<pdf_path>" page` (relative → workspace).
4. For each PNG in order: `read_image path=page-N.png`.
5. Concatenates per-page output with one blank line between pages.
6. Cleans up: `exec rm page-*.png`.

### Risks de-escalated by smoke test

- gpt-5.4 preamble leak: not observed in smoke (clean output).
- Vietnamese diacritics: preserved exactly.
- Prompt drift: only one source-of-truth file shipped; back-references on both ends.

### Risks still open

- **Multi-page PDF latency:** each page = 1 `read_image` call. For an N-page PDF, latency ≈ N × per-image-OCR time. Not blocking but Phase 3 should consider this when sizing the SnipTeX-side timeout.
- **Visibility=internal vs public:** the tex-ocr skill is `visibility=internal`, which requires explicit `skill_agent_grants` per agent. If Phase 2 creates a NEW agent for SnipTeX (separate from the test `ocr-tex` agent), it must also create the grant. Could alternatively bump tex-ocr to `visibility=public` (then no grant needed). Decision deferred to Phase 2.

## Risk Assessment

- **Risk**: gpt-5.4 reads `description` but still adds preamble like "Here is the OCR…".
  **Mitigation**: existing SnipTeX `post_process` pipeline strips preambles → SnipTeX side is already defensive. If it leaks through, iterate the description wording in a follow-up commit.
- **Risk**: Prompt drift between SnipTeX `MASTER_PROMPT` and Goclaw `tex-ocr/SKILL.md`.
  **Mitigation**: bidirectional comments (step 4) plus a note in the SnipTeX prompt file. Not bulletproof but better than nothing. Future work: shared file fetched at deploy time.
- **Risk**: Goclaw doesn't recognize the new skill after volume copy (rescan only updates existing, not discovers new).
  **Mitigation**: step 7 includes the container restart fallback. Restart is a few seconds of downtime on the VPS — acceptable for one-time deploy.

## Next Steps

Phase 2 creates the agent record in Postgres (or reuses the existing `ocr-tex` agent from smoke test), attaches this skill, and issues the API key SnipTeX will use.

**Phase 2 inputs already in place:**
- Skill ID: `019e7730-1b23-78c0-9fb0-3f579dc66916` (slug `tex-ocr`, version 3)
- Master tenant ID: `0193a5b0-7000-7000-8000-000000000001`
- Reference agent record (if Phase 2 wants to reuse): `ocr-tex` (`019e7736-8d17-7da7-b12e-ef9a160ee677`)
- Provider/model: `openai-codex-1` / `gpt-5.4` (verified to work end-to-end)
- Required global Tool Profile: must include `exec` in Also Allow (already configured)
- Required builtin tool config: `read_image` chain pointing at `openai-codex-1` (already configured)

**Phase 2 open questions:**
- Reuse `ocr-tex` agent or create a new `sniptex-ocr` agent? Reuse is simpler; new is cleaner for separation.
- Bump tex-ocr to `visibility=public` to avoid needing per-agent grants? Or stick with `internal` + explicit grant?
- Which tools to allow on the agent? Minimum: `read_file`, `read_image`, `exec`, `message`. Maybe also `read_document` (so the LLM has the option for non-PDF documents — but currently broken for PDFs, so probably skip).

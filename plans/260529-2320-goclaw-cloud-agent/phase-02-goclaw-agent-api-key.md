---
phase: 2
title: "Goclaw agent + API key"
status: completed
priority: P1
effort: "1.5h"
dependencies: [1]
---

# Phase 2: Goclaw Agent + API Key

## Overview

Insert the `tex-ocr` agent row into Goclaw's Postgres, grant it access to the Phase 1 Skill, then issue an API key SnipTeX will use. No file commits — all Goclaw-side state lives in `goclaw-postgres-1`.

## Requirements

- Functional: agent reachable as `agentId: "tex-ocr"` via `wss://goclaw.tikz2svg.com/ws`.
- Functional: agent uses provider `openai-codex-1` and model `gpt-5.4` (matches user's existing tikz-assistant, reuses ChatGPT Plus subscription).
- Functional: `tools_config` allows the minimum tools needed — `read_image`, `read_document`, `read_file`, `use_skill`, `message`. NO `exec`, `browser`, or `web_*` tools (OCR doesn't need them).
- Functional: `tex-ocr` skill (Phase 1) granted to this agent via `skill_agent_grants`.
- Functional: API key with scopes `operator.read` + `operator.write`, expires in 1 year (31,536,000 s).
- Non-functional: agent has dedicated `workspace` dir (`/app/workspace/tex-ocr`) to keep its uploaded media isolated from other agents.

## Architecture

Two writes against `goclaw-postgres-1` and one HTTPS POST against the gateway:

1. INSERT into `agents`:
   - `agent_key = 'tex-ocr'`
   - `display_name = 'SnipTeX OCR Agent'`
   - `provider = 'openai-codex-1'`
   - `model = 'gpt-5.4'`
   - `workspace = '/app/workspace/tex-ocr'`
   - `tools_config = '{"allow": ["read_image", "read_document", "read_file", "use_skill", "message"], "alsoAllow": ["message"]}'`
   - `other_config = '{"temperature": 0, "max_output_tokens": 8192}'` (best-effort — exact schema TBD by reading existing tikz row)
   - `agent_type = 'open'`
   - `status = 'active'`
   - `owner_id` and `tenant_id` — copy from existing tikz-assistant row.
2. INSERT into `skill_agent_grants` linking the new agent UUID to the `tex-ocr` skill UUID.
3. `POST /v1/api-keys` to issue the SnipTeX-facing key.

Two execution paths to choose between for steps 1–2:

| Path | Pros | Cons |
|------|------|------|
| **A. Admin Web UI** (`https://goclaw.tikz2svg.com/dashboard/agents/new`) | Validates inputs, no SQL injection risk, audit trail | Manual click-through; some fields may not be exposed (e.g., `tools_config` may be UI-restricted) |
| **B. Direct SQL via psql** | Full control over every column, scriptable, reproducible | Higher risk of typo / FK violation; bypasses UI validation |

**Recommendation: Path A** for the agent insert, **Path B** for the skill grant if the admin UI doesn't expose per-agent skill assignment. Mix-and-match is fine — Goclaw doesn't lock either path.

## Related Code Files

**Work context: VPS (`ssh Digital-Ocean-Goclaw`)** + browser if using Path A.

- Modify (DB only): `agents` table — INSERT one row.
- Modify (DB only): `skill_agent_grants` table — INSERT one row linking agent ↔ skill.
- No local repo commits.

## Implementation Steps

### Step 0: Sanity scout (read-only)

```bash
# Confirm tikz-assistant row to copy owner_id / tenant_id from.
ssh Digital-Ocean-Goclaw "docker exec goclaw-postgres-1 psql -U goclaw -d goclaw -c \
  \"SELECT id, agent_key, owner_id, tenant_id, workspace, other_config FROM agents WHERE agent_key='tikz-assistant';\""

# Confirm tex-ocr skill is registered (depends on Phase 1 deploy).
ssh Digital-Ocean-Goclaw "docker exec goclaw-postgres-1 psql -U goclaw -d goclaw -c \
  \"SELECT id, name, is_system, enabled FROM skills WHERE name='tex-ocr';\""
```

### Step 1: Create the agent

**Path A — admin UI:** Log in to `https://goclaw.tikz2svg.com/dashboard`. New agent with the field values from the Architecture section above. If `tools_config` is not user-editable in the UI, accept the UI defaults and finalize via Step 1b SQL patch.

**Step 1b (optional, only if Path A didn't expose `tools_config`):**
```sql
UPDATE agents
SET tools_config = '{"allow": ["read_image", "read_document", "read_file", "use_skill", "message"], "alsoAllow": ["message"]}'::jsonb,
    other_config = jsonb_set(coalesce(other_config, '{}'::jsonb), '{temperature}', '0'::jsonb)
WHERE agent_key = 'tex-ocr';
```

### Step 2: Grant the Skill to the agent

```bash
ssh Digital-Ocean-Goclaw "docker exec goclaw-postgres-1 psql -U goclaw -d goclaw -c \
  \"INSERT INTO skill_agent_grants (agent_id, skill_id) \
    SELECT a.id, s.id FROM agents a, skills s WHERE a.agent_key='tex-ocr' AND s.name='tex-ocr';\""
```

If the table has additional columns (e.g., `tenant_id`, `granted_at`), the scout step should have surfaced the schema — adjust the INSERT accordingly.

### Step 3: Smoke test the agent

Use the admin chat UI: open `https://goclaw.tikz2svg.com/dashboard/chats`, pick `tex-ocr` agent, attach a test image (small equation screenshot), send empty message. Confirm response is clean LaTeX, no preamble.

If the response is empty or has preamble: iterate the SKILL.md `description` (Phase 1 follow-up) until output is clean.

### Step 4: Issue the API key

```bash
# Replace <GATEWAY_TOKEN> with the value from VPS env (do NOT paste shell history).
read -s -p "Gateway token: " GW
echo
curl -X POST "https://goclaw.tikz2svg.com/api/v1/api-keys" \
  -H "Authorization: Bearer $GW" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "sniptex-desktop",
    "scopes": ["operator.read", "operator.write"],
    "expires_in": 31536000
  }' | jq
unset GW
```

Response contains `{ "key": "goclaw_xxx…" }` — **copy once, store in password manager + paste into SnipTeX Settings → Agents in Phase 4**. Goclaw does not display the value again.

### Step 5: End-to-end probe (no SnipTeX yet)

Use `wscat` or a Python one-liner to send a test `chat.send` with a previously-uploaded media path, confirm `content` comes back:

```bash
# Upload a test image first via the same endpoint SnipTeX will use.
curl -X POST "https://goclaw.tikz2svg.com/api/v1/media/upload" \
  -H "Authorization: Bearer goclaw_xxx" \
  -F "file=@/path/to/test-image.png" | jq

# Then chat.send with the returned path via wscat (or any WS tool).
# (Full WS handshake script in Phase 3's `cloud_goclaw_api.rs` will validate this end-to-end too.)
```

## Success Criteria

- [x] `SELECT agent_key FROM agents WHERE agent_key='tex-ocr'` returns one row. Agent UUID `019e7736-8d17-7da7-b12e-ef9a160ee677`, display_name `SnipTeX OCR Agent`, workspace `/app/workspace/tex-ocr`.
- [x] `skill_agent_grants` row links agent `019e7736-…` ↔ skill `019e7730-…` (`tex-ocr` slug v4).
- [x] API key authenticates: `GET /v1/skills` with `Authorization: Bearer goclaw_4c7540a5…` returns the full skills list (HTTP 200, tex-ocr v4 visible).
- [x] Smoke chat passes (carried over from Phase 1 end-to-end test on the same agent: image + multi-page Vietnamese PDF → clean LaTeX/Markdown).
- [x] API key issued — `id=019e791b-6cf4-7611-a858-7dcc490b6462`, prefix `4c7540a5`, expires `2027-05-30`. **Full key value disclosed once in execution session — must be saved to password manager + pasted into SnipTeX Settings → Agents in Phase 4.**

## Risk Assessment

- **Risk**: gateway token leaks via shell history when running step 4 curl.
  **Mitigation**: `read -s` (no echo), `unset` after, never `set -x` near this step. Don't commit anywhere.
- **Risk**: admin UI doesn't expose `tools_config` JSON, agent ends up with default tools that include `exec` / `browser` — wider attack surface than needed.
  **Mitigation**: step 1b SQL patch nails the exact JSON regardless of UI defaults. Verify with `SELECT tools_config FROM agents WHERE agent_key='tex-ocr'` before issuing the key.
- **Risk**: `skill_agent_grants` schema has more columns than naive INSERT covers (e.g., `tenant_id`).
  **Mitigation**: scout step 0 should surface `\d skill_agent_grants`. If unsure during impl, add a `\d` probe before the INSERT.
- **Risk**: gpt-5.4 via `openai-codex-1` is slow for full-page OCR (~80s/page on PDF pages, matches local codex CLI measurement). User may expect faster than they'll get.
  **Mitigation**: Document the latency expectation in `plan.md` (already done). Per-page budget in Phase 3 uses 120s (PDF_CLI_PAGE_TIMEOUT). Users with cloud-gemini get faster results — cloud-goclaw is for users who prefer routing through their ChatGPT subscription.

## Unresolved Questions

- ~~Exact schema of `skill_agent_grants`~~ → resolved: `(id, skill_id, agent_id, pinned_version, granted_by, created_at, tenant_id)` — INSERT must supply `tenant_id` (master) and `granted_by` (`'system'`).
- ~~Whether `other_config` is the right JSON field for `temperature` / `max_output_tokens`~~ → resolved (partial):
  - `other_config.max_tokens` IS read by Goclaw (`AgentData.ParseMaxTokens` in `internal/store/agent_store.go:228`). Plan's `max_output_tokens` key was wrong — actual key is `max_tokens`.
  - `temperature` is NOT read from `other_config` — Goclaw hardcodes `config.DefaultTemperature = 0.7` at `internal/agent/loop.go:269`. Per-agent override would need a code change. We stored `temperature: 0` in other_config anyway for forward-compat if Goclaw adds support, but currently no-op.

## Execution Notes (2026-05-30)

Phase 2 was largely **already done implicitly during Phase 1 smoke test**. Most of the work this phase: reconciling the smoke-test agent (`ocr-tex`) with the canonical name the plan specified (`tex-ocr`), tightening tools/config, and issuing the API key.

### Changes applied

| Where | Change | Notes |
|-------|--------|-------|
| `agents` (DB) UPDATE | `agent_key`: `ocr-tex` → `tex-ocr`; `display_name`: `OCR-tex` → `SnipTeX OCR Agent`; `workspace`: `/app/workspace/ocr-tex` → `/app/workspace/tex-ocr` | Single UPDATE statement on row UUID `019e7736-…`. |
| `agents.tools_config` | Refined to minimum: `{"allow": ["read_image", "read_document", "read_file", "exec", "message", "use_skill"]}` | Dropped: `edit`, `write_file`, `sessions_send` (unnecessary for OCR). Kept `exec` (required for PDF rasterization, per Phase 1 finding). Added `use_skill` (cleaner skill loading than raw `read_file`). |
| `agents.other_config` | Merged in `{"max_tokens": 8192, "temperature": 0}` via `\|\|` jsonb concat | `max_tokens` is honored. `temperature` is a no-op until Goclaw adds per-agent support. `description`, `self_evolve`, `skill_evolve` preserved. |
| Filesystem | `mv /var/lib/docker/volumes/goclaw_goclaw-workspace/_data/ocr-tex /var/lib/docker/volumes/goclaw_goclaw-workspace/_data/tex-ocr` | Preserves the 10+ test uploads inside `ws/system/.uploads/`. |
| API key | Issued via `POST /v1/api-keys` with name `sniptex-desktop`, scopes `[operator.read, operator.write]`, expires_in 1y | Sent from inside container with `$GOCLAW_GATEWAY_TOKEN` + `X-GoClaw-User-Id: system`. |

### Diverged from plan

| Plan said | Reality |
|-----------|---------|
| Brand new agent INSERT | UPDATE on existing `ocr-tex` row from Phase 1 smoke test (preserves skill grant + smoke test continuity) |
| `tools_config` without `exec` | INCLUDED `exec` — Phase 1 proved it's required for PDF rasterization via `pdftoppm` |
| Path A admin UI for agent creation | Direct SQL UPDATE (UI would create a new row, lose the existing skill grant) |
| `other_config.max_output_tokens` | Used correct key `max_tokens` (Goclaw code reads this key per `AgentData.ParseMaxTokens`) |
| `other_config.temperature: 0` | Stored but ineffective — Goclaw hardcodes 0.7. Documented as a no-op. |

### Discoveries (for future plan revision)

1. **Goclaw hardcodes temperature** at `internal/agent/loop.go:269 → config.DefaultTemperature (=0.7)`. Per-agent override not supported in current Goclaw build. For deterministic OCR, would need either a Goclaw patch OR rely on gpt-5.4's inherent behavior at 0.7 (which has been good enough so far per smoke test).
2. **`/v1/agents` filters by `ListAccessible(userID)`** for non-owner API keys → empty list for a fresh `X-GoClaw-User-Id`. Not a bug, just the access model. SnipTeX should use `agentId` directly in `chat.send`, not enumerate via `/v1/agents`.
3. **`skill_agent_grants` requires `tenant_id` + `granted_by`** in addition to `(skill_id, agent_id)`. Plan's naive INSERT would have failed. The Phase 1 grant (done via admin UI) handled this automatically.

### API key disclosure record

| Field | Value |
|-------|-------|
| Name | `sniptex-desktop` |
| ID | `019e791b-6cf4-7611-a858-7dcc490b6462` |
| Prefix | `4c7540a5` |
| Scopes | `operator.read`, `operator.write` |
| Created | 2026-05-30T13:38:21Z |
| Expires | 2027-05-30T13:38:21Z |
| Full value | Disclosed in session output of the POST response — copy to password manager. Goclaw does not display the value again. |

If lost: revoke via `POST /v1/api-keys/019e791b-…/revoke` and re-issue.

## Next Steps

Phase 3 builds the SnipTeX Rust adapter that uploads via `/v1/media/upload` then talks to this agent over WebSocket.

**Phase 3 inputs ready:**
- Agent: `agentId = "tex-ocr"` (in WS `chat.send.params.agentId`)
- API key: `goclaw_4c7540a5…` (paste into SnipTeX Settings → Agents → cloud-goclaw → api_key)
- WS endpoint: `wss://goclaw.tikz2svg.com/ws`
- Media upload: `POST https://goclaw.tikz2svg.com/api/v1/media/upload` (multipart)
- Skill version live: v4 (parallel read_image calls)

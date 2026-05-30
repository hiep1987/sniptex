---
phase: 2
title: "Goclaw agent + API key"
status: pending
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

- [ ] `SELECT agent_key FROM agents WHERE agent_key='tex-ocr'` returns one row.
- [ ] `SELECT * FROM skill_agent_grants` shows the link between the new agent UUID and the `tex-ocr` skill UUID.
- [ ] `GET https://goclaw.tikz2svg.com/api/v1/agents` (with Bearer auth) lists `tex-ocr`.
- [ ] Smoke chat through admin UI returns clean LaTeX for a test image.
- [ ] API key value captured in password manager + ready to paste into SnipTeX in Phase 4.

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

- Exact schema of `skill_agent_grants` (likely just `(agent_id, skill_id)` but could have tenant scoping).
- Whether `other_config` is the right JSON field for `temperature` / `max_output_tokens`, or if Goclaw reads those from a different column / provider-specific config.

## Next Steps

Phase 3 builds the SnipTeX Rust adapter that uploads via `/v1/media/upload` then talks to this agent over WebSocket.

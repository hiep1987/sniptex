---
title: "Goclaw Cloud Agent Integration"
description: "Add cloud-goclaw agent to SnipTeX that delegates OCR (image + PDF → LaTeX/Markdown) to a Skill-based Goclaw agent on goclaw.tikz2svg.com. Two-step protocol: HTTPS multipart upload to /v1/media/upload, then WS chat.send referencing the returned path. Auth via goclaw_xxx API key."
status: pending
priority: P2
branch: "main"
tags: ["goclaw", "agent", "websocket", "ocr", "tex"]
blockedBy: []
blocks: []
created: "2026-05-29"
createdBy: "ck:plan"
source: skill
---

# Goclaw Cloud Agent Integration

## Overview

Add `cloud-goclaw` as a 5th SnipTeX agent. Backed by a new Skill-based agent on the user's existing Goclaw VPS (`https://goclaw.tikz2svg.com`) — same infra hosting the `tikz-assistant` agent. SnipTeX uploads each rendered image to `/v1/media/upload` (HTTPS multipart), then sends `chat.send` over WebSocket referencing the returned path. The agent runs the Phase 1 OCR Skill and returns LaTeX/Markdown.

Value vs existing cloud agents: a single endpoint the user already operates, freedom to swap LLM backends on Goclaw without touching SnipTeX, and reuse of the user's ChatGPT Plus subscription via Goclaw's `openai-codex-1` provider (no separate OpenAI API spend).

## Phases

| Phase | Name | Status |
|-------|------|--------|
| 1 | [Goclaw TeX OCR Skill](./phase-01-goclaw-tex-ocr-skill.md) | Completed |
| 2 | [Goclaw agent + API key](./phase-02-goclaw-agent-api-key.md) | Completed |
| 3 | [SnipTeX cloud-goclaw adapter](./phase-03-sniptex-cloud-goclaw-adapter.md) | Completed |
| 4 | [SnipTeX UI + settings wiring](./phase-04-sniptex-ui-settings-wiring.md) | Pending |

## Architecture

```
SnipTeX (Tauri, Rust)
    │
    │ STEP 1 — upload image (HTTPS multipart)
    │   POST https://goclaw.tikz2svg.com/api/v1/media/upload
    │   Authorization: Bearer goclaw_xxx
    │   Content-Type: multipart/form-data
    │   Body: file=<png bytes>
    │   ← Response: { path: "/temp/xxx.png", mime_type: "image/png" }
    │
    │ STEP 2 — chat.send (WebSocket)
    │   wss://goclaw.tikz2svg.com/ws
    │   req connect { token: "goclaw_xxx", user_id }
    │   req chat.send {
    │     agentId: "tex-ocr",
    │     sessionKey,
    │     message: "",
    │     media: [{ path: "<step1 path>", filename: "page-001.png" }]
    │   }
    │   ← res { content: "<latex/markdown>", usage: { input_tokens, output_tokens } }
    │
Goclaw Gateway (Docker container goclaw-goclaw-1, port 18790 → Caddy → 443)
    │
    │ load tex-ocr agent record from Postgres (agents table, agent_key=tex-ocr)
    │ inject skill metadata into system prompt (name + description + location)
    │ LLM (provider=openai-codex-1, model=gpt-5.4) processes attached media + prompt
    │
TeX OCR Skill at /app/data/skills-store/tex-ocr/1/SKILL.md
    │
    │ frontmatter description carries the OCR rule (gpt-5.4 often skips read_file)
    │ body has detailed format spec for read_file fallback
```

## Goclaw Infra Facts (verified via VPS SSH 2026-05-30)

| Component | Value |
|-----------|-------|
| VPS | `Digital-Ocean-Goclaw` (68.183.187.144), DigitalOcean droplet |
| Goclaw working dir | `/opt/goclaw/` (Go source w/ custom patches) |
| Containers | `goclaw-goclaw-1` (port 18790), `goclaw-postgres-1`, `goclaw-goclaw-ui-1` |
| Public domain | `https://goclaw.tikz2svg.com` (Caddy reverse proxy) |
| Public API base | `https://goclaw.tikz2svg.com/api` (strips `/api` prefix) |
| WebSocket | `wss://goclaw.tikz2svg.com/ws` |
| Skill volume (host) | `/var/lib/docker/volumes/goclaw_goclaw-data/_data/skills-store/` |
| Skill path format | `{name}/{version}/SKILL.md` (version dir mandatory) |
| Agent storage | Postgres `agents` table — `agent_key` unique per tenant, JSON `tools_config` |
| Reference agent | `tikz-assistant`: provider=`openai-codex-1`, model=`gpt-5.4` |
| Media upload endpoint | `POST /v1/media/upload` (multipart/form-data, Bearer auth) |
| chat.send.media format | `[{ "path": "<absolute>", "filename": "<display>" }]` (parsed by `chat.go::parseMedia`) |
| API key creation | `POST /v1/api-keys` with gateway token Bearer auth |

## Critical Goclaw Quirks (must respect in design)

1. **Only `name + description + location` of skills are injected into the system prompt.** The body requires the agent to call `read_file` to load. `gpt-5.4` (openai-codex-1 provider) frequently skips this step. → Phase 1 puts the OCR rule into the `description` frontmatter.

2. **`media` is path-based, never inline.** Goclaw's `chat.go::parseMedia` rejects base64 — only accepts `[{path, filename}]` or legacy `[path]`. → Phase 3 always uploads first.

3. **gpt-5.4 latency is comparable to local codex CLI** (~80s/page on full PDF pages, per SnipTeX measurement). Goclaw doesn't make the model faster, only avoids the local subprocess spawn. Per-page budget in Phase 3 should match codex (PDF_CLI_PAGE_TIMEOUT = 120s).

4. **Per-key rate limits in Goclaw are unclear** — production tikz agent uses `RATE_LIMITED` error code, but exact thresholds aren't documented. Phase 3 maps `RATE_LIMITED` to `DispatchError::RateLimited` and lets the dispatcher's fallback chain handle it.

## Key Dependencies

- Goclaw VPS at `goclaw.tikz2svg.com` already operational.
- SnipTeX existing dispatcher (`ocr::run_ocr`, `run_per_page_pdf_ocr`, `dispatch_pdf_ocr`) — new adapter slots into `CloudApi` branch like cloud-gemini.
- Rust crates already in deps: `reqwest` (rustls-tls, multipart enabled in Phase 3), `tokio` (full), `base64`, `serde_json`, `uuid`. NEW deps in Phase 3: `tokio-tungstenite` (rustls-tls-native-roots).
- `bot-tex` repo (new, separate from `bot-tikz`) as the source-of-truth location for the `tex-ocr` Goclaw skill (mirrors how the `tikz` skill is structured inside `bot-tikz`).

## Cross-Repo Boundary

- **Phase 1**: skill source committed to `bot-tex` repo (new, dedicated to the `tex-ocr` skill), deployed to VPS Docker volume via `scp`.
- **Phase 2**: Goclaw side — admin UI or SQL on `goclaw-postgres-1` for agent insert + `POST /v1/api-keys` HTTP call for key. No file commit to any local repo.
- **Phases 3–4**: SnipTeX repo (current).

Each phase declares its work-context path explicitly. Plan lives in SnipTeX because SnipTeX drives the integration.

## Out of Scope

- Streaming responses (`stream: true`) — initial version uses non-streaming.
- Multi-Goclaw-instance support — one endpoint URL for v1.
- Account/billing/quota UI for Goclaw — user manages keys via admin panel directly.
- Hot-swapping the Goclaw agent's model from SnipTeX side — model lives in DB, user changes it via admin.
- Reusing one persistent WebSocket across multiple OCR calls — per-call WS lifecycle (matches `chatbot_goclaw.py` reference impl in `claudekit_tikz2svg_api`).

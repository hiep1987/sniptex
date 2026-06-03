---
title: "Phase 03 - Registry, Settings UI, and Config"
status: complete
priority: P2
effort: 2h
created: 2026-06-03
---

# Phase 03 - Registry, Settings UI, and Config

## Context Links

- [Scout report](./reports/scout-report.md)
- Registry: `src-tauri/src/agents/registry.rs`
- Settings UI: `src/windows/settings/agents-tab.tsx`

## Overview

Register `cloud-novita-hybrid` as a separate selectable cloud agent using the same Novita API key.

## Key Insights

- Existing Settings UI maps `cloud-novita` to provider `"novita"`.
- Hybrid can reuse the same key; no new keychain account needed.
- Availability should require the existing Novita key only.

## Requirements

- Functional:
  - Agent appears as "Novita OCR + GPT OSS".
  - It is visible in Settings.
  - It is installed/available only when key + endpoint exist.
  - Manual rerun menu can select it.
- Non-functional:
  - Do not place it ahead of cheaper/stable default agents yet.
  - Show clear no-endpoint state.

## Architecture

```text
detect_installed_agents
  -> if id == CLOUD_NOVITA_HYBRID_ID
  -> has_novita_api_key
  -> AgentInfo(version: "deepseek-ocr-2 + gpt-oss-120b")
```

## Related Code Files

- Modify: `src-tauri/src/agents/registry.rs`.
- Modify: `src-tauri/src/agents/mod.rs`.
- Modify: `src-tauri/src/ocr/dispatcher.rs`.
- Modify: `src/windows/settings/agents-tab.tsx`.
- Modify: `src/stores/settings-store.ts`.
- Optional modify: `src-tauri/src/settings.rs`.

## Implementation Steps

1. Add `CLOUD_NOVITA_HYBRID_ID`.
2. Add `AgentSpec` with `kind: CloudApi`.
3. Add detection branch:
   - Existing Novita key.
   - Endpoint configured.
4. Add dispatcher branch to call `cloud_novita_hybrid_api`.
5. Add `ALL_KNOWN` entry in Settings UI.
6. Update provider mapping so both Novita agents use provider `"novita"`.
7. Keep default priority unchanged at first, or append hybrid after `cloud-novita`.

## Todo List

- [x] Add registry constant and spec.
- [x] Add detection branch.
- [x] Add dispatcher branch.
- [x] Add Settings UI entry.
- [x] Decide endpoint config UI vs env-only MVP.

## Success Criteria

- Agent appears when configured.
- Agent does not appear as available with missing endpoint.
- Rerun menu can select hybrid when available.
- Existing `cloud-novita` remains unchanged.

## Risk Assessment

- Risk: Two Novita agents confuse users.
  - Mitigation: display clear labels: "DeepSeek-OCR 2" vs "Novita OCR + GPT OSS".
- Risk: Endpoint config hidden in env var is hard for non-dev users.
  - Mitigation: start env-only for MVP; add UI after benchmark.

## Security Considerations

- No duplicate key storage.
- Do not store endpoint URL if it contains a secret token.
- If endpoint auth differs from Novita API key, add separate keychain account.

## Next Steps

- Phase 04 validates behavior with tests and smoke benchmark.

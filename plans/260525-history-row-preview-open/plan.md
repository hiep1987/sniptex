---
title: "History row click opens Preview window"
description: "Click a History row to open Preview window with that item's OCR content"
status: implementation-complete-gui-smoke-pending
priority: P2
branch: "main"
tags: [history, preview, ux]
blockedBy: []
blocks: []
created: "2026-05-25T06:54:22.386Z"
createdBy: "ck:plan"
source: skill
---

# History row click opens Preview window

## Overview

Clicking a History row currently does nothing. This plan adds a click handler that emits a Tauri event (`history-preview-open`) carrying the row's data. PreviewWindow listens for this event alongside `snip-complete` and reuses its existing render/copy/autohide flow.

## Architecture

```
HistoryRow click
  → emit("history-preview-open", SnipResult-shaped payload)
  → Tauri IPC bridges to PreviewWindow webview
  → useSnipResult hook catches it (same as snip-complete)
  → PreviewWindow renders + auto-copy + auto-hide
```

No Rust changes. Pure frontend event wiring.

## Phases

| Phase | Name | Status |
|-------|------|--------|
| 1 | [Implement event emission and listener](./phase-01-implement-event-emission-and-listener.md) | Implementation complete, GUI smoke pending |

## Dependencies

None.

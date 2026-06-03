# Novita OCR + GPT OSS Agent

SnipTeX includes an experimental `cloud-novita-hybrid` agent:

```text
image -> DeepSeek-OCR 2 -> markdown -> GPT OSS 120B -> final LaTeX/Markdown
```

It is separate from `cloud-novita`, which still uses DeepSeek-OCR 2 directly without the GPT OSS cleanup step.

## Requirements

A Novita API key saved in Settings under Novita.ai.

## Cost Controls

- GPT OSS 120B receives only DeepSeek-OCR markdown, not the original image.
- Intermediate markdown is capped before cleanup.
- GPT output is capped to 4096 tokens.
- The agent is not in the default fallback chain until live benchmarks prove quality and cost.

## Smoke Test

```bash
cd src-tauri
NOVITA_API_KEY=sk_... cargo run --bin novita_hybrid_smoke -- /path/to/sample.png
```

Add `--show-output` only for non-sensitive samples.

## Rollout Status

Current status: opt-in/manual. Live quality, latency, and token cost still need validation before this agent should become a default fallback.

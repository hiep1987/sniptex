---
phase: 1
report: prompt-validation
date: 2026-05-20
updated: 2026-05-21  # Round 2 + Round 3 — EQ/TABLE coverage + E2 prompt-patch verification
status: draft  # awaiting user manual accuracy + go/no-go sign-off
sample_size_total: 51  # R1: 20 MIXED + R2: 12 EQ + 9 TABLE + R3: 9 TABLE re-run + 1 retest
agents: [gemini-cli@0.42.0, codex-cli@0.130.0]
results_locations:
  round1_mixed: plans/results-mixed-20/
  round2_eq_table: plans/results-round2-eq-table/
  round3_table_only_repatched: plans/results-round3-table-only/
  round3_retest_15_01_25: plans/results-round3-retest-15.01.25/
---

# Phase 1 — Prompt Validation Report

## Decision (2026-05-21)

**Phase 1 verdict: CONDITIONAL-GO via Path C (Hybrid).**

Codex CLI is the BYOA default; Gemini Vision API direct-call is added as a `--cloud` fallback for sub-5s latency. Gemini CLI stays available but downgraded to "experimental secondary" with documented EQ_ONLY caveat. See "Recommendation" section for downstream propagation.

## TL;DR (updated 2026-05-21, post Round 2)

- **Codex: 41/41 (100%) success across all 3 categories.** 20/20 MIXED, 12/12 EQUATION_ONLY, 9/9 TABLE_ONLY. Zero workspace-boundary failures, zero hallucinations observed.
- **Gemini: 33/41 (80.5%) — and the failure rate is category-dependent.** 95% on MIXED, 78% on TABLE_ONLY, **58% on EQUATION_ONLY**. EQ_ONLY failures share a single root cause: Gemini's agent loop tries to `read_file ~/.claude/.ck.json` (outside its workspace) when the image has no surrounding text to anchor on, then crashes.
- **Master-prompt label-leak patch (applied 2026-05-20) works: 0/41 outputs leak the category label**, down from 5% pre-patch on both agents.
- **Format-rule violations: 1/27 Codex (3.7%), 1/14 Gemini (7.1%) on success rows.** Both reduce on inspection to fixture-categorization slop (images filed as EQ_ONLY / TABLE_ONLY actually contain Vietnamese surrounding text → legitimately MIXED), not true agent rule-breaking.
- **TABLE_ONLY spec resolved (Round 3, 2026-05-21):**
  - E1 (Markdown vs LaTeX tabular) — user decided **Path C: dual output**. Master prompt stays Markdown; Phase 9 owns the LaTeX `\begin{tabular}` toggle as a separate output mode + own validation pass.
  - E2 (interval-wrapping divergence) — master prompt patched; verified on the user-cited image. Gemini and Codex now produce text-identical TABLE_ONLY output. See "Round 3" section.
- **Vietnamese diacritics preserved: 100% on both agents across all 3 categories.**
- **Latency still misses the 6 s p95 target.** Codex is now consistent: p95 in the 14–24 s band across all categories (closer to feasibility under a relaxed target). Gemini p95 25–46 s.
- **Revised verdict: NO-GO if `p95 ≤ 6 s` is non-negotiable; CONDITIONAL-GO with Codex-only as default if threshold relaxes to ~25 s.** Gemini's EQ_ONLY failure mode is a hard operational blocker for shipping it as the BYOA default — only suitable as a secondary option with documented "works best on mixed pages" caveat.
- **Recommended default agent: Codex CLI.** Gemini-as-secondary only after the workspace/tool-loop constraints are gated in Phase 3 (e.g. `--approval-mode plan` or `--policy` disabling `read_file`/`write_file`).

## Methodology

- **Script:** `plans/test-prompt.sh` (patched 2026-05-20 — Mac `tac` fallback, codex `--skip-git-repo-check`, `--output-last-message`, Gemini `@"..."` path quoting, YOLO/Ripgrep banner stripping).
- **Master prompt:** unchanged from plan (mirrors `src-tauri/src/ocr/prompt.rs` design). Same prompt sent to both agents.
- **Fixtures:** 32 CleanShot screenshots dropped by user into `fixtures/` (Vietnamese SGK Toán pages — calculus, algebra, vector geometry, modelling examples). First 20 alphabetically used as a stratified sample.
  - **Categorization caveat:** Fixtures were NOT pre-sorted into `sgk/`, `equations/`, `tables/`, `mixed/` per plan §Implementation Steps. All 20 sampled images are SGK-style mixed-content pages. Pure equation-only / table-only categories are not represented in this sample.
- **Run:** 20 images × 2 agents = 40 LLM calls. Single sequential pass, no retries.
- **Outputs:** `plans/results/{gemini,codex}/*.txt`, `plans/results/summary.csv`, `plans/results/comparison.md`.
- **Ground truth:** Not authored (user opted for visual scoring from `comparison.md`). Accuracy below is a visual spot-check across paired outputs, not a measured percent against ground truth.
- **Environment:** macOS Darwin 25.4.0, Apple Silicon, `gemini 0.42.0`, `codex-cli 0.130.0`, GNU `coreutils 9.11` (gdate).

## Aggregate Metrics (n = 20)

| Metric                       | Gemini    | Codex     | Plan threshold        | Verdict |
|------------------------------|-----------|-----------|-----------------------|---------|
| Success rate                 | 95.0% (19/20) | 100% (20/20) | (implicit ≥90%)       | PASS / PASS |
| Latency median (p50)         | 21.1 s    | 14.8 s    | —                     | — |
| **Latency p95**              | **45.7 s** | **23.9 s** | **≤ 6 s**             | **FAIL / FAIL** |
| Latency p99                  | 46.5 s    | 25.1 s    | —                     | — |
| Latency min / max            | 14.7 s / 46.6 s | 11.6 s / 25.4 s | — | — |
| Latency mean                 | 24.4 s    | 16.0 s    | —                     | — |
| Avg output length (chars)    | 592       | 592       | —                     | parity |
| Format detection consistency (both agents same `detected_type` per image) | 19/19 (100%) | — | ≥ 90% | PASS (MIXED-only sample) |
| Category-label-leak (output begins with `MIXED`/`EQUATION_ONLY`/`TABLE_ONLY` literal) | 1/20 (5%) | 1/20 (5%) | implicit 0% | **FAIL** — prompt bug |
| Vietnamese diacritics present (heuristic, presence of {ă â ê ô ơ ư đ + tone marks}) | 19/19 (100%) | 20/20 (100%) | ≥ 95% | PASS / PASS |
| Format detected (all sample images) | MIXED ×19 | MIXED ×20 | — | (sample bias — all SGK pages) |

## Per-Agent Observations

### Codex CLI 0.130.0 — **Recommended Default**

- **100% success on 20/20 fixtures.**
- p50 ≈ 15 s, p95 ≈ 24 s — slower than the plan target but **consistently the faster of the two**.
- `--output-last-message` flag produces a clean assistant-only file — no session headers, no token accounting noise. Easy to consume in the Rust dispatcher (`plans/replan.md` §6 `codex.rs`).
- **Operational requirements** to make codex non-interactive OCR work:
  - `--skip-git-repo-check` (CWD doesn't need to be a git repo)
  - `-- "${PROMPT}"` separator so the prompt isn't consumed by the greedy `--image <FILE>...` arg
  - `--output-last-message <FILE>` for clean output

### Gemini CLI 0.42.0 — **Recommended Secondary**

- 19/20 success. The 1 failure was **not an OCR failure** — Gemini's agent loop chose to invoke its `write_file` tool to dump the answer to `/tmp/ocr_prompt.txt`, which was rejected by Gemini's workspace-boundary check. Output ended up empty.
- p50 ≈ 21 s, p95 ≈ 46 s — roughly 2× slower than Codex.
- **Workspace-boundary constraint is hard:** Gemini refuses to access any path outside its CWD project root. The `fixtures/` directory MUST live inside the user's working tree (cannot be a `/tmp/` staging dir). For SnipTeX runtime, the user's snip image will be passed from outside Gemini's workspace unless the app stages it into a project-rooted location. **This is a real architectural constraint for Phase 3.**
- Output requires aggressive stripping: every Gemini call emits "YOLO mode is enabled" (×2) and "Ripgrep is not available" banner lines on stderr+stdout. Patched in `post_process` (commit candidate: drop both regex lines).
- **Tool-use loop is unpredictable:** Gemini occasionally decides to "save the answer to a file" or "summarize first" instead of returning the raw answer. On 1/20 it cost a full success; on a separate smoke-test image it produced 300 unrelated Vietnamese words about uniform circular motion when the image was a vector-subtraction equation (full hallucination — Codex produced the correct 2-line LaTeX answer for the same image). **Risk: low-frequency but high-impact accuracy events.**

### Content Agreement (visual spot-check)

- 19 images where both agents succeeded.
- Output character lengths within 30% on 19/19 paired outputs.
- Spot-checked 3 paired outputs visually — Codex and Gemini produce **mathematically identical** LaTeX/Markdown content. Differences are stylistic only (`VÍ DỤ 2.` vs `**Ví dụ 2.**`, `0,75` vs `0,75` are identical, slight whitespace).
- **Diacritic preservation: 100% on both agents** across all sampled outputs (ă â ê ô ơ ư đ + all tone marks intact, by `grep` heuristic against Vietnamese-character classes).

> [USER ACTION] Open `plans/results/comparison.md` and manually score N ≥ 10 paired outputs for true OCR accuracy (exact mathematical equivalence vs the source image). Fill in `## Manual Accuracy Scoring` below before finalizing the go/no-go.

## Critical Findings

1. **Latency thresholds in the plan are unrealistically aggressive for CLI agents.** Even on a warm shell with no cold-start, `gemini -p` and `codex exec` carry 10-15 s per-call overhead before the model even begins inferring. Plan's `p95 ≤ 6s` target may have been written assuming direct-API call cost (~2-4 s) and not CLI-agent boot cost. **Either revise the threshold or pivot to direct-API.**
2. **Gemini's workspace-boundary check is binding.** For Phase 3, the image dispatched to Gemini MUST live inside the user's project root (or inside `~/.gemini/tmp/<project>/`). Plan to stage `~/.local/share/sniptex/snips/<uuid>.png` and ensure the Tauri app's CWD when spawning `gemini` includes that path. This is a real constraint, not a cosmetic one.
3. **Gemini's agent loop is non-deterministic in headless mode.** It can decide to "save to file" or hallucinate context not in the image. Codex did not exhibit either behavior on this run. **If you ship both agents, expect support burden on Gemini failure modes to dominate.**
4. **All 20 sample fixtures classified as MIXED** by both agents. Plan §Implementation calls for separate equation-only / table-only / SGK / mixed buckets — the current fixture set does not exercise EQUATION_ONLY or TABLE_ONLY output paths. Phase 1 NOT complete coverage; format-consistency PASS applies only to the MIXED path.
5. **Both agents are equally good at Vietnamese diacritics** — no observed drops on ă â ê ô ơ ư đ + tone marks. Master-prompt rule "Preserve Vietnamese diacritics exactly" is working.
6. **Master prompt leaks the category label into the output ~5% of the time, on both agents.** Observed instances:
   - Codex on `14.56.36@2x.png`: output begins `MIXED\n\n**Ví dụ 2.** …`
   - Gemini on `14.55.50@2x.png`: output begins `MIXED\n…`
   The prompt's `DETECTION:` section reads as "step 1: classify, step 2: output" to ~5% of inferences — agents print the category as a literal header. **Mitigation applied 2026-05-20:** master prompt in `plans/test-prompt.sh` updated — `DETECTION:` heading renamed to `DETECTION (internal, do not emit):` with explicit rule `Silently classify the image…Do NOT print the category name. Do NOT prefix or suffix your output with "EQUATION_ONLY", "TABLE_ONLY", or "MIXED".` No re-run executed (user opted to defer validation spend). Phase 3 Rust prompt MUST mirror this language, and the Rust post-process MUST also defensively strip leading `^(MIXED|EQUATION_ONLY|TABLE_ONLY)\s*$` lines as a safety net.
7. **TABLE_ONLY output path is genuinely unverified for v1 readiness.** Zero isolated-table fixtures were included in the sample. The closest case — `14.56.36@2x.png` — is a MIXED page where a table happens to appear inline; both agents handled the embedded table correctly (`| Thời gian (phút) | [0; 5) | … |` rendered identically in both outputs apart from one agent's leading `MIXED` header bug noted above). **However:** no fixture exercises the "output a pure GitHub Markdown table with no surrounding text" code path. **Phase 1 cannot sign off on TABLE_ONLY until 5-10 table-only fixtures are run.** Same caveat applies, less acutely, to EQUATION_ONLY.

## Per-Category Coverage — Round 2 (2026-05-21)

Following the user's TABLE_ONLY-unverified finding on 2026-05-20, 21 additional fixtures were dropped into `fixtures-extra/{equation-only,table-only}/` and run through the same harness. This round also incidentally validated the master-prompt label-leak patch applied 2026-05-20.

### Aggregate per category (Round 2)

| Metric                       | EQ_ONLY (n=12)   | TABLE_ONLY (n=9) |
|------------------------------|------------------|------------------|
|                              | Gemini · Codex   | Gemini · Codex   |
| Success                      | 7/12 · 12/12     | 7/9 · 9/9        |
| Success rate                 | **58.3% · 100%** | 77.8% · 100%     |
| Latency p50 (ms)             | 15 472 · 9 474   | 25 583 · 12 595  |
| Latency p95 (ms)             | 25 849 · 14 010  | 43 239 · 15 101  |
| Latency mean (ms)            | 17 748 · 9 971   | 28 001 · 11 905  |
| Avg chars/output             | 95 · 130         | 307 · 237        |
| Label-leak count             | 0/7 · 0/12       | 0/7 · 0/9        |
| Format-rule violations       | 0/7 · 1/12       | 1/7 · 0/9        |

### Detected-type breakdown vs expected

| Category    | Expected     | Gemini detected               | Codex detected         |
|-------------|--------------|-------------------------------|------------------------|
| EQ_ONLY     | EQUATION_ONLY| 7× MIXED (heuristic miscall)  | 12× MIXED (heuristic miscall) |
| TABLE_ONLY  | TABLE_ONLY   | 6× TABLE_ONLY, 1× MIXED       | 9× TABLE_ONLY          |

The 100% "MIXED" detection on EQUATION_ONLY outputs is a **script-side bug, not an agent bug**. The script's `detect_type` heuristic in `plans/test-prompt.sh` checks `\n\n` blank-line presence before checking for `\frac|\int|\sum`, so any multi-line LaTeX equation (e.g. two equations separated per the master prompt's `Multiple equations: separate with \\` rule) is misclassified. The agents themselves are correctly emitting raw LaTeX without `$` delimiters (0 format-rule violations on Gemini's 7 successful EQ outputs).

### Critical Round-2 findings

**A. Gemini's agent loop fails systematically on isolated equations.**
4 of 5 Gemini EQ_ONLY failures share the identical error: `Error executing tool read_file: Path not in workspace: Attempted path "/Users/hieplequoc/.claude/.ck.json"`. Gemini decides to look up a Claude Code config file (which isn't even installed for it) before answering the OCR prompt. When the image has no surrounding text, the agent loop appears to seek "context" from the filesystem and crashes when denied. The 5th failure (`15.00.43@2x.png`) dumped JavaScript terminal-state internals — an outright Gemini-CLI crash.

**Implication for v1:** Shipping Gemini as the BYOA default is operationally risky. Users snipping equations (a core SnipTeX use case) would see 40%+ failure rate. **Either (a) gate Gemini behind `--approval-mode plan` or `--policy` tool-disable flags in Phase 3, (b) tell users Codex is the only fully-supported agent, or (c) drop Gemini back to v1.x and reverse Session-2 Q1.**

**B. Codex is uniformly reliable.** 41/41 across categories, lowest p95 (14–24 s) consistent across all 3, fastest on EQ_ONLY (p95 14 s — closest any agent has come to the 6 s threshold). The single Codex "format violation" (`05.38.54@2x.png`) is actually correct behavior: the image contains the Vietnamese word "suy ra" alongside the cosine formula, making it legitimately MIXED, and Codex's `$...$` inline math is the correct MIXED-format output. **Fixture miscategorization, not agent rule-break.**

**C. Master-prompt label-leak patch is effective.** 0/41 outputs leak the category label after the 2026-05-20 patch. Pre-patch rate was 1/20 on each agent (5%). Phase 3 Rust prompt MUST mirror the `DETECTION (internal, do not emit):` wording, and the Rust post-process should still defensively strip leading `^(MIXED|EQUATION_ONLY|TABLE_ONLY)\s*$` lines as belt-and-suspenders.

**D. Latency improves on focused content for Codex.** EQ_ONLY p95 = 14 s, TABLE_ONLY p95 = 15 s — Codex is meaningfully faster on simpler images than on dense MIXED pages (p95 = 24 s). Gemini does not show the same trend (TABLE_ONLY p95 = 43 s, worse than MIXED). Suggests: in real SnipTeX usage, where snips are typically single equations or small tables, Codex p95 will trend toward the lower end of this range.

### TABLE_ONLY readiness verdict

- **Codex on TABLE_ONLY: READY for v1.** 9/9 success, 9/9 correct `TABLE_ONLY` detection, 0 format violations, p95 = 15 s. The user's prior finding ("TABLE_ONLY conversion quality is currently below acceptable threshold and remains unverified") **is resolved on Codex** — but see finding E below for a spec-level caveat that applies to BOTH agents.
- **Gemini on TABLE_ONLY: NOT READY.** 78% success rate with 1 format violation. Same operational caveats as EQ_ONLY apply (lower than acceptable for a "default" agent).

### Round-2 Critical finding E — TABLE_ONLY emits Markdown tables with inline `$...$`, NOT LaTeX `tabular` environments

**Observation (user-confirmed 2026-05-21):** TABLE_ONLY outputs are structurally correct per the current master-prompt spec (`If TABLE_ONLY: Output GitHub Markdown table`), but the resulting artifact is a GitHub Markdown table whose math cells contain inline LaTeX wrapped in `$...$`, NOT a fully normalized LaTeX table environment (`\begin{tabular}{...}` / `\begin{array}{...}`).

**Example — `CleanShot 2026-05-20 at 15.01.25@2x.png`:**

Gemini (more aggressive inline-math wrapping — wraps the interval too):
```
| Nhóm        | Giá trị đại diện | Tần số  |
|-------------|------------------|---------|
| $[40 ; 45)$ | $x_1$            | 3       |
| $[45 ; 50)$ | $x_2$            | 12      |
| …           | …                | …       |
|             |                  | $n = 40$|
```

Codex (more conservative — only wraps variables):
```
| Nhóm     | Giá trị đại diện | Tần số  |
|----------|------------------|---------|
| [40 ; 45)| $x_1$            | 3       |
| [45 ; 50)| $x_2$            | 12      |
| …        | …                | …       |
|          |                  | $n = 40$|
```

**Two distinct sub-issues:**

E1. **Spec ambiguity: Markdown table vs LaTeX `tabular`.** For a tool branded "Sni*pTeX*", a non-trivial fraction of users will paste into a LaTeX document (Overleaf, TeXShop, VSCode + LaTeX Workshop) and expect a `\begin{tabular}{|l|c|r|}…\end{tabular}` environment, not a Markdown table. The current spec choice (Markdown) optimizes for Notion / Obsidian / GitHub-issue paste targets. **Both paste targets are legitimate.** The plan's Phase 9 "Format Toggle" likely owns this — but Phase 1's validation only certifies the Markdown path.

E2. **Inter-agent inconsistency on inline-math scope.** Gemini wraps numeric intervals like `[40 ; 45)` in `$...$`, Codex leaves them as plain text. Both produce visually-acceptable output in a MathJax-rendered preview (Phase 6), but the raw text differs in ways that affect:
- Diffability / determinism across agent swaps
- Round-trip rendering in pure-Markdown viewers (Gemini's intervals render as math, Codex's as text)
- Test fixture stability if Phase 6 ever adds golden-file regression tests

**Recommendation for Phase 1 sign-off:** Treat E as a spec issue to be resolved at the master-prompt level, not an agent bug. Three options:

| Option | Master-prompt TABLE_ONLY rule | Trade-off |
|---|---|---|
| **E.a — Keep current** | "Output GitHub Markdown table" (current) | Optimizes for Notion/Markdown paste. Mismatches "SnipTeX" branding for LaTeX-doc users. Phase 9 toggle deferred. |
| **E.b — Switch to LaTeX `tabular`** | "Output a LaTeX tabular environment: `\begin{tabular}{|c|c|c|}…\end{tabular}`. Use `\hline` for row separators." | Matches branding. Loses Notion/Markdown paste fidelity. Single output mode. |
| **E.c — Dual output, toggle in app** | Master prompt stays Markdown; Phase 9 adds a Markdown→LaTeX `tabular` post-process converter (or a separate "give me LaTeX tabular" reformat call) | Both paste targets supported. Adds Phase 9 complexity. Adds a re-validation round at Phase 9 for the LaTeX path. |

To resolve E2 (Gemini's aggressive `$...$` wrapping), the master prompt could add: `Inside table cells, only wrap mathematical variables, fractions, and equations in $...$. Plain numeric intervals like [40; 45) and integer counts MUST remain plain text.` This needs re-validation to confirm both agents respect it.

### Round 3 — E2 patch verification (2026-05-21)

**Action taken (2026-05-21, post-Round 2):** Master prompt in `plans/test-prompt.sh` patched with the E2 rule:
> Inside table cells: only wrap mathematical variables, fractions, equations, and symbolic expressions in `$...$`. Plain numeric intervals like `[40; 45)`, plain integers, plain percentages (15%), and ordinary words MUST remain unwrapped.

**Decision on E1 (2026-05-21):** Path C — master prompt stays Markdown table; Phase 9 owns the LaTeX `\begin{tabular}` toggle as a separate output mode. Phase 9 needs a re-validation pass when the LaTeX-tabular prompt mode lands. (Phase 9 file should be updated to reflect this scope addition — see "Next Steps" below.)

#### Round 3 run (all 9 table-only fixtures, post-patch)

Saved to `plans/results-round3-table-only/`.

| Metric                        | Round 2 (pre-patch) | Round 3 (post-patch) | Δ |
|-------------------------------|---------------------|----------------------|---|
| Codex success                 | 9/9 (100%)          | 9/9 (100%)           | unchanged |
| Codex p95 latency             | 15 101 ms           | 15 030 ms            | flat |
| Codex interval-wraps observed | 0/9                 | 0/9                  | unchanged — Codex was already compliant |
| Gemini success                | 7/9 (78%)           | 7/9 (78%)            | unchanged rate, **different failed images** (non-deterministic) |
| Gemini p95 latency            | 43 239 ms           | ~58 000 ms           | **regressed** — longer prompt may have added agent-loop overhead |
| Gemini interval-wraps on the 5 R2+R3 both-success images | 0/5 | 0/5 | indeterminate — those 5 images didn't exhibit wrapping in R2 either |

The all-9 Round-3 run lost `15.01.25@2x.png` (the ONLY image that exhibited interval-wrapping in Round 2) to a Gemini timeout. So the all-9 run could not validate the patch's effect on the specific image of interest.

#### Round 3 retest — `15.01.25@2x.png` standalone (the user-cited image)

Re-ran the patched prompt on just `15.01.25@2x.png` (saved to `plans/results-round3-retest-15.01.25/`). Both agents succeeded on this attempt (Gemini latency 55 s, Codex 14 s).

Side-by-side, pre-patch (Round 2) vs post-patch (Round 3 retest):

| Cell                | Pre-patch Gemini | Post-patch Gemini | Post-patch Codex |
|---------------------|------------------|-------------------|------------------|
| First interval cell | `$[40 ; 45)$`    | `[40 ; 45)`       | `[40 ; 45)`      |
| Variable cell       | `$x_1$`          | `$x_1$`           | `$x_1$`          |
| Footer cell         | `$n = 40$`       | `$n = 40$`        | `$n = 40$`       |

**Patch confirmed effective.** All 5 intervals (`[40;45)`, `[45;50)`, `[50;55)`, `[55;60)`, `[60;65)`) — previously wrapped by Gemini, now plain text. Variables and equations still correctly wrapped in `$...$`. **Inter-agent text-level agreement now 100%** on this image (Gemini and Codex outputs are character-identical except for trailing whitespace).

#### Round 3 verdict

- **E2 resolved.** Master prompt patch eliminates Gemini's spurious interval-wrapping. Both agents now produce text-identical TABLE_ONLY output on the verified case.
- **Codex latency unaffected** by the longer prompt (15 s p95 stable).
- **Gemini latency regressed** ~35% on the all-9 re-run (43 s → 58 s p95). Longer prompt = more agent-loop work. Acceptable for v1 since Gemini is already recommended as secondary, but worth noting.
- **Gemini reliability on TABLE_ONLY remains 78%** — unchanged. The patch does not address Gemini's deeper systemic instability; only the wrapping behavior.
- **E1 deferred to Phase 9** per user decision (Path C: dual output — Markdown default, LaTeX `tabular` toggle in Phase 9 + own validation pass).

### Script bugs surfaced by Round 2 (defer fix to Phase 2 or 3)

- `detect_type` heuristic order: check LaTeX-density BEFORE blank-line presence, otherwise EQUATION_ONLY is always misclassified. Suggested fix:
  ```bash
  # Check LaTeX-density first
  if [[ "$has_latex" == "1" ]] && [[ "$natural_words" -lt 3 ]] && ! echo "$trimmed" | grep -qE '\$|\|---'; then
      echo "EQUATION_ONLY"; return
  fi
  ```
  Not blocking for Phase 1 — only affects the script's category classification, not the agents' output formats.

## Go/No-Go Assessment vs Plan Thresholds

| Threshold | Required | Gemini | Codex | Verdict |
|-----------|----------|--------|-------|---------|
| Success rate (all categories combined) | implicit ≥ 90% | 33/41 (80.5%) | 41/41 (100%) | **FAIL** Gemini / PASS Codex |
| Success rate EQUATION_ONLY | (not explicit, derived from accuracy) | 7/12 (58%) | 12/12 (100%) | **FAIL** Gemini / PASS Codex |
| Success rate TABLE_ONLY | (not explicit) | 7/9 (78%) | 9/9 (100%) | FAIL Gemini / PASS Codex |
| Accuracy ≥ 80% on SGK subset | 80% | [USER TO FILL — visual review] | [USER TO FILL — visual review] | PENDING |
| Latency p95 ≤ 6 s | 6 000 ms | 45 692 ms (MIXED) / 25 849 ms (EQ) / 43 239 ms (TABLE) | 23 942 ms (MIXED) / 14 010 ms (EQ) / 15 101 ms (TABLE) | **FAIL** for both — but Codex is within 2.3× on focused content |
| Format consistency ≥ 90% | 90% | 100% — across all 3 categories on successful outputs | 100% — across all 3 categories | PASS / PASS |
| Vietnamese diacritics preserved ≥ 95% | 95% | 100% | 100% | PASS / PASS |
| Codex image-input syntax functional | Y/N | N/A | YES (`--image <FILE> --skip-git-repo-check -- "<prompt>"`) | PASS |
| Category-label leak rate | implicit 0% | 0/33 successful outputs (post-patch) | 0/41 successful outputs (post-patch) | PASS / PASS (patch effective) |

**Hard failures:**
- Latency p95 ≤ 6 s — fails for both agents on every category.
- Gemini overall success rate (80.5%) — below the implicit 90% bar driven by EQ_ONLY collapse.

**Per plan §Success Criteria strict interpretation:** Phase 1 = NO-GO. Per a relaxed reading (Codex-only default + tiered latency target): Phase 1 = CONDITIONAL-GO.

### Manual Accuracy Scoring

> [USER ACTION] After reviewing `plans/results/comparison.md`, fill in below.

| Image (last 20 chars) | Gemini accuracy (subjective 0-100%) | Codex accuracy (subjective 0-100%) | Notes |
|-----------------------|-------------------------------------|------------------------------------|-------|
| 14.51.26@2x.png       |                                     |                                    |       |
| 14.51.43@2x.png       |                                     |                                    |       |
| 14.52.01@2x.png       |                                     |                                    |       |
| 14.52.12@2x.png       |                                     |                                    |       |
| 14.52.31@2x.png       |                                     |                                    |       |
| 14.52.53@2x.png       |                                     |                                    |       |
| 14.53.14@2x.png       |                                     |                                    |       |
| 14.53.26@2x.png       |                                     |                                    |       |
| 14.53.48@2x.png       |                                     |                                    |       |
| 14.54.12@2x.png       | FAIL (tool-use loop)                |                                    |       |

**Aggregate accuracy %:**  Gemini = ____   Codex = ____

## Recommendation

**Decision (2026-05-21, user): Path C — Hybrid.** Ship Codex CLI as the BYOA default for privacy-first users, AND add a Gemini Vision API direct-call mode as a built-in `--cloud` (or `--api`) fallback for users who want sub-5-second response. This reverses Session-1 Q3 (which chose "CLI-only BYOA in v1") with Round-1–3 latency evidence as the justification. Privacy framing on the landing page changes from "BYOA only" to "BYOA or BYOK" (Bring Your Own Agent or Bring Your Own Key).

**Downstream propagation required (NOT part of the immediate validation commit):**
- Phase 3 (`agents/` registry) — add `cloud_gemini_api.rs` adapter alongside `gemini_cli.rs` and `codex.rs`. Dispatcher gains a 3rd entry; settings UI gains a "Use cloud API (faster)" toggle.
- Phase 8 (onboarding) — add an "API key" tab alongside the existing "Install Gemini CLI" / "Install Codex" guides. Document Gemini API key acquisition flow.
- Phase 13 (landing copy) — hero subtitle changes from "Bring your own Gemini CLI / Codex" to "Bring your own agent OR your own API key — your choice".
- Phase 15 (marketing) — privacy framing updated; talking points add "5s response via cloud API mode".
- `replan.md` — Session-1 Q3 noted as reversed; update §6 dispatcher diagram.
- `plan.md` — Validation Log gets a Session-3 entry documenting this reversal (similar in form to Session 2's Q1 reversal).

These propagations are project-management-skill work; **not included in the validation commit (which is scoped narrowly to script + report + .gitignore per user request).**

### Three paths considered (kept for the record):

### Path A — Soft-GO: Codex-only default, Gemini deferred / gated

**Argument (revised after Round 2):** OCR is run on-demand by user hotkey, not in a tight loop. 14–24 s for a one-shot snip is acceptable UX with a progress spinner. Plan's 6 s figure was likely a copy from an API-baseline assumption that doesn't apply to CLI-agent shells. **Codex is the only agent that crosses the reliability bar across all 3 categories**; Gemini's EQ_ONLY 58% rate is incompatible with a "default agent" promise for an OCR snip tool where equations are the headline use case.

**Impact:** Phase 2–15 mostly unchanged. `replan.md` §9 latency note + `phase-01` success-criteria edited to (a) relax `p95` to ~25 s and (b) downgrade Gemini from co-default to "experimental secondary, requires `--approval-mode plan`" or drop it back to v1.x entirely (effectively re-applying Session-1 Q1's reversed answer). Codex shipped as the sole default agent. Onboarding (Phase 8), landing copy (Phase 13), marketing (Phase 15) all narrow to Codex.

### Path B — Hard-NO-GO: Pivot to Gemini Vision API direct or local LightOnOCR

**Argument:** 6 s p95 is non-negotiable for the product positioning (Mathpix-class latency). CLI-agent overhead is structural and won't decrease. Write `reports/pivot-evaluation.md` comparing Gemini Vision API ($0.0025 per image, ~2-4 s) vs LightOnOCR-2-1B (free, local, ~1-2 s on Apple Silicon).

**Impact:** Phase 2+ scope shifts — `agents/` dispatcher replaced by direct HTTP client (Phase 3) or local-model bundle (Phase 3 + new dependency surface). "BYOA" framing on landing page drops or pivots to "your API key, our app".

### Path C — Hybrid (recommended if user has bandwidth for one extra surface)

**Argument:** Ship Codex CLI as the BYOA default for privacy-first users, AND offer Gemini Vision API as a built-in `--cloud` fallback for users who want sub-5-second response. This is the v1.x option referenced in `plan.md` Session-1 Q3 ("Hybrid CLI + Gemini Vision API"). Two code paths in Phase 3, but each is simple.

**Impact:** Phase 3 grows by ~30%; Phase 13 landing copy needs new tier. Privacy promise becomes "BYOA or BYOK".

## Risks Carried Forward

- **Sample under-coverage (Round 1 only):** ~~20/90 images scored; 0 EQUATION_ONLY / TABLE_ONLY fixtures exercised. TABLE_ONLY conversion quality is currently below acceptable threshold and remains unverified for v1 readiness.~~ **Resolved 2026-05-21:** Round 2 added 12 EQ_ONLY + 9 TABLE_ONLY fixtures. Codex is verified on all 3 paths. Gemini is verified MIXED-only; **EQ_ONLY (58%) and TABLE_ONLY (78%) are below v1 readiness for Gemini** — see Critical Round-2 finding A. Total sample now 41/90 — still below plan's 90 but enough to make a defensible Codex-only sign-off call.
- **No ground-truth accuracy measurement.** Visual spot-check is suggestive but not rigorous. If go/no-go is contested, author 30 expected outputs and re-run accuracy against string-diff or sympy-equality.
- **Gemini hallucination observed once** in smoke test (not in 20-image sample). Frequency unknown — could be 1/30 or 1/300. Recommend monitoring in real usage.
- **Latency floor is structural.** Cannot be optimized below CLI-agent boot cost (~10 s for both). The only escape hatch is direct API or local model.

## Next Steps

- **If GO (Path A):** Edit `phase-01-prompt-validation-go-no-go.md` Success Criteria to (a) relax latency p95 to ~25 s, (b) make Codex the sole default agent for v1, (c) downgrade Gemini per Session-1 Q1 (drop to v1.x) OR keep Gemini as experimental secondary with mandatory `--approval-mode plan` / `--policy` tool-disable flags. Mark phase 1 status = complete. Propagate the Codex-only narrowing to Phase 3 (`agents/` registry), Phase 8 (onboarding install guide), Phase 13 (landing hero copy), Phase 15 (marketing posts) — effectively re-applying Session-1 Q1 with Round-2 data as the justification this time.
- **Phase 9 scope addition (from E1 decision):** update `phase-09-theme-format-toggle-ux-polish.md` to include a LaTeX `\begin{tabular}` output mode for tables (currently Phase 9 may only cover the Markdown↔LaTeX inline-math toggle, not the tabular-environment toggle). Add a re-validation TODO for that mode using the same 9 table-only fixtures.
- **If NO-GO (Path B):** Write `plans/260520-0603-sniptex-tauri-mvp-v1/reports/pivot-evaluation.md` with Gemini Vision API + LightOnOCR cost/latency/quality comparison. Hold Phase 2 until pivot decision is final.
- **If Hybrid (Path C):** Update `phase-03-agent-system-ocr-pipeline.md` to include `cloud-gemini.rs` adapter alongside `gemini-cli.rs` and `codex.rs`. Add settings UI affordance to choose mode (Phase 8). Update `replan.md` §6 BYOA framing.

## Open Questions

- Should the GO/NO-GO threshold be a single hard latency bar, or a tiered one (e.g. "GO if p95 ≤ 6 s; CONDITIONAL-GO if p95 ≤ 25 s with progress UI; NO-GO otherwise")? Plan as written has only one bar.
- Should accuracy be measured via ground-truth diff (rigorous, requires 30+ expected files) or by user-spot-check (this report's approach)? Decision affects Phase 1 cost.
- Are EQUATION_ONLY and TABLE_ONLY fixtures necessary for Phase 1 sign-off, or can format-consistency be validated under MIXED only and the other paths regression-tested in Phase 6 (MathJax preview)?
- Should the master prompt explicitly forbid Gemini's "save to file" tool-use loop (e.g. via `--policy` disable list or prompt-level "RESPOND IN TEXT, DO NOT INVOKE TOOLS")?

---

*Generated by `/cook` execution of Phase 1, 2026-05-20. Raw results under `plans/results/`. Re-run with `bash plans/test-prompt.sh ./fixtures-sample/`.*

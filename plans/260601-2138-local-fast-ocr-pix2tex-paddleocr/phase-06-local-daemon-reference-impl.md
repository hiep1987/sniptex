---
phase: 6
title: "Local Daemon Reference Implementation"
status: pending
priority: P1
effort: 1d
dependencies: [1]
---

# Phase 6: Local Daemon Reference Implementation

## Context Links

- `plans/260601-2138-local-fast-ocr-pix2tex-paddleocr/phase-01-local-ocr-service-contract.md` — contract this daemon must satisfy
- `plans/260601-2138-local-fast-ocr-pix2tex-paddleocr/phase-02-local-pix2tex-agent.md` — consumer of `/ocr/pix2tex`
- `plans/260601-2138-local-fast-ocr-pix2tex-paddleocr/phase-03-local-paddleocr-agent.md` — consumer of `/ocr/paddleocr`
- `plans/260601-2138-local-fast-ocr-pix2tex-paddleocr/phase-04-auto-local-fast-router.md` — consumer of `/classify`

## Overview

Ship a reference daemon under `scripts/local-ocr-server/` that implements the Phase 1 HTTP contract. Without this phase the sniptex adapter code has nothing to talk to, and "Phase 5: install scripts" would be installing an undefined server. This phase makes the daemon a real, testable artefact owned by sniptex.

The daemon is out-of-tree from the Tauri app build — it lives in `scripts/local-ocr-server/`, has its own `requirements.txt`, and is not packaged with sniptex releases. Users opt in by running the install scripts (Phase 5).

## Key Insights

- **FastAPI + uvicorn** is the right size for this — single-file server, async-friendly, automatic OpenAPI schema. No need for Flask/Django.
- **Classifier is a heuristic, not a model.** A small Python function that looks at aspect ratio, OCR text-density (cheap PaddleOCR fast-pass on a downscaled image), and pixel statistics covers the `equation | text | mixed | table | unknown` taxonomy well enough for routing. Avoiding a CLIP/CNN classifier saves ~400 MB install weight and dodges another dependency.
- **Model loading happens at daemon startup**, not per-request. pix2tex and PaddleOCR are both lazy-init in their constructors; the server pre-warms them on startup so the first request isn't a 10s cold-start.
- **No GPU required.** Both models run on CPU at acceptable speed for snip-sized images. CUDA is opt-in via env var so power users on NVIDIA hardware can flip it on, but the default path stays CPU.
- **Stateless.** Every request is independent — no session, no auth, no rate limit. Listening on `127.0.0.1` is the security boundary.
- **Per-model serialization.** pix2tex's underlying PyTorch model is not thread-safe; PaddleOCR has similar caveats. Wrap each model in an `asyncio.Lock`, so concurrent snips queue rather than race. This matches the existing sniptex pattern of `concurrency=1` for cloud agents — predictable behaviour, no surprise crashes.
- **No table support in v1.** Reflecting Phase 3's decision: PaddleOCR's bbox output is not assembled into Markdown tables here. Classifier returns `kind="table"` → router falls through to cloud-mistral.

## Requirements

Functional:
- Implement endpoints from Phase 1 service contract exactly:
  - `GET /health` → `{ ok, version, capabilities }`
  - `POST /classify` (multipart) → `{ kind, confidence }`
  - `POST /ocr/pix2tex` (multipart) → `200 { text, detected: "EQUATION_ONLY", confidence }`
  - `POST /ocr/paddleocr` (multipart):
    - `200 { text, detected: "MIXED", confidence }` for paragraph text
    - **`422 { error: "unsupported_table" }`** when the classifier (or a quick pre-pass) determines the image is table-shaped. v1 is paragraph-only here; never return `detected: "TABLE_ONLY"` from this endpoint.
- Pre-warm pix2tex and PaddleOCR models at startup; expose load progress via `/health.ok = false` until ready.
- Heuristic classifier in pure Python (no ML model), informed by:
  - aspect ratio (very wide → text, near-square → equation/table)
  - average colour density / background ratio (table → many vertical edges)
  - PaddleOCR fast-pass text count + average symbol-likeness (`\`, `frac`, `int` count → equation hint)
- Configurable via env vars:
  - `LOCAL_OCR_PORT` (default 8765)
  - `LOCAL_OCR_HOST` (default `127.0.0.1`, never bind `0.0.0.0` by default)
  - `LOCAL_OCR_DEVICE` (`cpu` | `cuda`, default `cpu`)
  - `LOCAL_OCR_DISABLE_PIX2TEX` / `LOCAL_OCR_DISABLE_PADDLE` for users who only want one capability

Non-functional:
- Cold start (load both models): target < 30 s on M1 CPU.
- Hot request: see Phase 2 / Phase 3 latency targets.
- Memory: stay under 2 GB resident with both models loaded.

## Architecture

```text
scripts/local-ocr-server/
├── README.md            # quick start, env vars, troubleshooting
├── requirements.txt     # fastapi, uvicorn, pix2tex, paddleocr, paddlepaddle, Pillow
├── pyproject.toml       # optional — for editable installs
├── server.py            # FastAPI app, endpoint handlers
├── classifier.py        # heuristic classify() function
├── pix2tex_wrapper.py   # thin adapter around pix2tex.cli.LatexOCR
├── paddle_wrapper.py    # thin adapter around paddleocr.PaddleOCR(lang="vi")
└── tests/
    ├── test_classifier.py   # heuristic decisions on fixture images
    ├── test_endpoints.py    # FastAPI TestClient, mocks the wrappers
    └── fixtures/            # 3-5 small reference images (committed; <100 KB)
```

Request flow:

```text
HTTP request → FastAPI handler
  → decode multipart image bytes → PIL.Image
  → dispatch to classifier.py OR wrapper module
  → wrap result in pydantic response model
  → return JSON
```

## Related Code Files

- Create: `scripts/local-ocr-server/server.py`
- Create: `scripts/local-ocr-server/classifier.py`
- Create: `scripts/local-ocr-server/pix2tex_wrapper.py`
- Create: `scripts/local-ocr-server/paddle_wrapper.py`
- Create: `scripts/local-ocr-server/requirements.txt`
- Create: `scripts/local-ocr-server/README.md`
- Create: `scripts/local-ocr-server/tests/test_classifier.py`
- Create: `scripts/local-ocr-server/tests/test_endpoints.py`
- Create: `scripts/local-ocr-server/tests/fixtures/*.png` (3-5 fixtures from MVP Phase 1 test set, ≤100 KB each)

## Implementation Steps

1. Scaffold `scripts/local-ocr-server/` with the file layout above.
2. Write `requirements.txt`:
   - `fastapi >= 0.115`
   - `uvicorn[standard] >= 0.30`
   - `pix2tex >= 0.1.4`
   - `paddleocr >= 2.8`
   - `paddlepaddle >= 2.6` (CPU build)
   - `Pillow >= 10`
3. Implement `pix2tex_wrapper.py` — singleton class that loads `LatexOCR()` once, exposes `async predict(image: PIL.Image) -> str` guarded by a module-level `asyncio.Lock` so concurrent requests serialize.
4. Implement `paddle_wrapper.py` — singleton class that loads `PaddleOCR(lang='vi', use_angle_cls=True)` once, exposes `async predict(image: PIL.Image) -> list[tuple[str, float]]` (text + per-line confidence) under its own `asyncio.Lock`.
5. Implement `classifier.py::classify(image: PIL.Image) -> tuple[str, float]` using heuristic features. Document each rule with a comment so future humans understand what the magic number means. Define module-level constants `EQUATION_THRESHOLD` and `TEXT_THRESHOLD` — these are the SAME values Phase 4's router reads (see Phase 4 for the rule table). Single source of truth.
6. Implement `server.py`:
   - FastAPI app with lifespan event to pre-warm both wrappers (set `app.state.ready = True` when both load).
   - 4 endpoints matching Phase 1 contract.
   - Pydantic models for response schemas.
   - `/health.ok` returns `app.state.ready`.
7. **Calibrate thresholds** against fixture sets (P1 of MVP plan: 10 EQUATION_ONLY + 9 TABLE_ONLY + 10 MIXED + a Vietnamese-paragraph set):
   - Sweep candidate thresholds (e.g. 0.55 → 0.85 in 0.05 steps for both `EQUATION_THRESHOLD` and `TEXT_THRESHOLD`).
   - Pick the pair minimizing mis-route rate (target < 5%).
   - Commit the chosen values as named constants in `classifier.py`, with a comment noting calibration date + dataset.
   - Write the chosen values + measured mis-route into `docs/local-fast-ocr.md` (Phase 5 inserts the table).
8. Add tests:
   - `test_classifier.py` — call `classify()` on 5 fixture images, assert expected kind.
   - `test_endpoints.py` — use FastAPI `TestClient`, mock the wrapper `predict()` methods, verify response shapes match Phase 1 contract.
   - `test_concurrency.py` — fire 3 concurrent `/ocr/pix2tex` requests via `httpx.AsyncClient`, assert they all succeed and the model didn't crash (basic lock validation).
9. Write `README.md` covering: install, run, env vars, troubleshooting "model load slow", how to disable one capability, **note that requests are serialized per model (max 1 concurrent)**.
10. Smoke run: `pip install -r requirements.txt && uvicorn server:app --port 8765`, then `curl http://127.0.0.1:8765/health` returns the expected schema.

## Todo List

- [ ] `scripts/local-ocr-server/` scaffolded.
- [ ] `requirements.txt` with pinned-major versions.
- [ ] pix2tex wrapper singleton with asyncio.Lock.
- [ ] PaddleOCR wrapper singleton with asyncio.Lock.
- [ ] Heuristic classifier with commented rules.
- [ ] Threshold calibration sweep run; constants committed in `classifier.py`.
- [ ] FastAPI server with 4 endpoints + lifespan pre-warm.
- [ ] Pydantic response models match Phase 1 contract byte-for-byte.
- [ ] Classifier test on 5 fixtures.
- [ ] Endpoint test via TestClient with mocked wrappers.
- [ ] Concurrency test (3 parallel requests succeed without crash).
- [ ] README install / run / env-vars / troubleshooting / concurrency-note.

## Success Criteria

- Daemon starts with `uvicorn server:app --port 8765` and responds to `GET /health` in <100 ms once warm.
- Hitting `/ocr/pix2tex` with a fixture equation image returns LaTeX in <2 s hot.
- Hitting `/ocr/paddleocr` with a Vietnamese-paragraph fixture preserves diacritics.
- Concurrent requests serialize cleanly — no NaN / model corruption from 3-parallel test.
- All Phase 6 tests pass; classifier hits ≥ 95 % accuracy (mis-route < 5 %) on the calibration fixture set after threshold tuning.
- Phase 2 + Phase 3 + Phase 4 adapters (sniptex side) can complete their tests against THIS daemon — no mock required.

## Risk Assessment

- Risk: `paddlepaddle` install fails on Apple Silicon. Mitigation: pin to a version known to ship M1 wheels; document fallback to `paddlepaddle==2.6.0` if 2.7+ regresses on macOS.
- Risk: pix2tex pulls torch ~600 MB. Mitigation: documented in install size table; users who only want text OCR can set `LOCAL_OCR_DISABLE_PIX2TEX=1`.
- Risk: classifier heuristic too crude → mis-routes. Mitigation: Phase 4 router only takes the local path if classifier confidence ≥ threshold; mis-classifications fall through to cloud.
- Risk: out-of-tree daemon code drifts from sniptex adapter expectations. Mitigation: Pydantic response models in `server.py` and Rust deserialise structs in `local_ocr_api.rs` are reviewed together in Phase 1.

## Security Considerations

- Default bind to `127.0.0.1` only — do NOT support `0.0.0.0` even via env var without an additional explicit `LOCAL_OCR_TRUST_LAN=1` flag (and even then, document that the existing cloud agents are the right tool for remote OCR, not this daemon).
- No authentication — relies on loopback isolation. Document this clearly in README.
- Image bytes are decoded with PIL; reject files > 10 MB to avoid memory DoS.

## Next Steps

- Phase 5 install scripts call into this daemon's `requirements.txt` / startup command.
- Optional vNext: build a sidecar binary (PyInstaller / nuitka) so the daemon can be Tauri-managed; out of scope here.

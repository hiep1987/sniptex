# Cloud Goclaw OCR Agent

**Ngày:** 2026-05-31
**Trạng thái:** Shipped (commit `c8ae659` Phase 4, `e962216` perf fix)
**Plan:** [`plans/260529-2320-goclaw-cloud-agent/`](../plans/260529-2320-goclaw-cloud-agent/plan.md)

---

## Là gì

`cloud-goclaw` là agent OCR thứ 5 trong SnipTeX. Khác với 4 agent còn lại (codex/gemini-cli là CLI local, cloud-gemini/cloud-mistral là API trực tiếp), `cloud-goclaw` **delegate OCR cho 1 Goclaw agent chạy server-side** trên VPS của user (`https://goclaw.tikz2svg.com`).

Goclaw đứng giữa làm proxy LLM:

```
SnipTeX (Rust) ──HTTPS upload──> Goclaw VPS ──gpt-5.4── ChatGPT Plus account
              └──WSS chat.send──> tex-ocr skill
                                  ↑
                          (Phase 1 skill teaches OCR rules)
```

## Tại sao có

| Lý do | Chi tiết |
|-------|---------|
| **Reuse ChatGPT Plus subscription** | Goclaw dùng `openai-codex-1` provider (OAuth ChatGPT) → user trả tiền 1 lần qua subscription, không bị tính API spend riêng |
| **LLM-swappable** | Đổi model trên Goclaw side (gpt-5.4 → claude → gemini) mà không động SnipTeX code |
| **Cùng infra với bot-tikz** | User đã vận hành Goclaw VPS cho `tikz-assistant`, thêm 1 agent là zero-cost ops |

## Setup (lần đầu)

### Bước 1: Tạo API key trên Goclaw

1. Đăng nhập `https://goclaw.tikz2svg.com` (admin)
2. Sidebar **System → API Keys** (route `/api-keys`)
3. **Create** key với:
   - Name: `sniptex-desktop` (gợi ý)
   - Scopes: `operator.read` + `operator.write`
   - Expires: 1 năm (hoặc theo policy của bạn)
4. **Copy ngay value `goclaw_xxx...` vào password manager** — Goclaw chỉ hiển thị 1 lần
   - ⚠️ **KHÔNG nhầm với ID** (`019e791b-...`) — ID không authenticate được, chỉ value `goclaw_xxx` mới valid

### Bước 2: Paste key vào SnipTeX

1. **Settings → Agents tab**
2. Tìm row **"Goclaw OCR Agent"** (badge `Cloud`)
3. Click **"Set API key"** → paste `goclaw_xxx...` → **Save key**
4. Row chuyển sang badge xanh **"Key set"**
5. (Optional) Drag agent lên đầu fallback chain nếu muốn dùng làm primary

## Setup (lần đầu cho VPS — chỉ admin Goclaw cần đọc)

Một lần duy nhất khi tích hợp Goclaw vào project mới. Nếu user đã có VPS Goclaw chạy `tikz-assistant`, các bước này có thể skip nếu agent `tex-ocr` đã được tạo sẵn.

1. **Deploy `tex-ocr` skill** vào volume `/var/lib/docker/volumes/goclaw_goclaw-data/_data/skills-store/tex-ocr/1/SKILL.md`. Source: [`bot-tex` repo](https://github.com/...) (separate from `bot-tikz`).
2. **Register skill** qua `POST /v1/skills/upload` (multipart zip, admin auth + `X-GoClaw-User-Id: system` header). Disk placement alone **không** đủ — Goclaw không scan startup.
3. **Tạo agent record** `tex-ocr` (display_name `SnipTeX OCR Agent`, provider `openai-codex-1`, model `gpt-5.4`, workspace `/app/workspace/tex-ocr`). Tools allow tối thiểu: `read_image, read_document, read_file, exec, message, use_skill`.
4. **Grant skill cho agent** qua `skill_agent_grants` (tex-ocr skill `visibility=internal` cần explicit grant).
5. **Disable built-in `pdf` skill cho master tenant** để gpt-5.4 không nhầm sang skill generic:
   ```sql
   INSERT INTO skill_tenant_configs (skill_id, tenant_id, enabled)
   SELECT id, '<master_tenant_uuid>', false FROM skills WHERE name='pdf';
   ```
6. **Config builtin tool `read_image`** → provider chain trỏ đúng `openai-codex-1` (default chain trỏ `openai-codex` stale, sẽ fail). Sửa qua admin UI: `/builtin-tools → read_image → Settings → MediaProviderChainForm`.
7. **Server → Tools → Also Allow**: thêm `exec` (mặc định Tool Profile có thể filter nó out, làm agent không rasterize được PDF).

Đầy đủ runbook + SQL trong [`plans/260529-2320-goclaw-cloud-agent/phase-01-goclaw-tex-ocr-skill.md`](../plans/260529-2320-goclaw-cloud-agent/phase-01-goclaw-tex-ocr-skill.md) → section "Execution Notes".

## Cách hoạt động (per OCR call)

Hai bước cho mỗi page/image:

1. **Upload** `POST https://goclaw.tikz2svg.com/api/v1/media/upload` (multipart, `Authorization: Bearer goclaw_xxx`) → returns `{ path: "/tmp/ws_upload_xxx.png" }`
2. **WS chat.send** `wss://goclaw.tikz2svg.com/ws`:
   ```json
   {"type":"req","id":"1","method":"connect","params":{"token":"goclaw_xxx","user_id":"sniptex-<uuid>"}}
   {"type":"req","id":"2","method":"chat.send","params":{
     "agentId":"tex-ocr",
     "sessionKey":"tex-ocr:<uuid>",
     "message":"",
     "media":[{"path":"<step1 path>","filename":"page-001.png"}]
   }}
   ```
   → Server `res` frame `{ok:true, payload:{content: "<latex/markdown>"}}`

Mỗi call mở WS mới, không pool. PDF flow: SnipTeX rasterize PDF → PNG/trang trên local → upload + chat từng PNG riêng (parallel với cap=2 cho cloud-goclaw).

**Implementation:** `src-tauri/src/agents/cloud_goclaw_api.rs` (~420 dòng). 20 unit tests covering parse + redaction + DispatchError mapping.

## Latency expectation

gpt-5.4 ≈ codex CLI latency. Trên Vietnamese math (~3000-3700 chars/page):

| Workload | Wall-clock (cap=2 parallel) |
|----------|---------------------------|
| Single image | 30-80s |
| 2-page PDF | ~65s |
| 5-page PDF (projection) | ~140s |

**Per-page budget: 120s** (`PDF_CLI_PAGE_TIMEOUT`). Page dày bất thường có thể hit timeout → dispatcher fallback chain tiếp tục với agent kế.

Nếu cần nhanh hơn, dùng `cloud-gemini` (~10s/2-page) — nhưng tốn Google AI Studio API quota.

## Key rotation

1. Tạo key mới trên Goclaw admin (Bước 1 Setup)
2. SnipTeX Settings → Goclaw → **Update API key** → paste new value
3. (Optional) Revoke key cũ: `POST /v1/api-keys/<old_id>/revoke` với gateway token

Không cần restart SnipTeX — key load fresh trên mỗi OCR call.

## Troubleshooting

### `api auth failed (HTTP 401)`

99% là paste nhầm **ID** thay vì **value**:
- Sai: `019e791b-6cf4-7611-a858-7dcc490b6462` (UUID format)
- Đúng: `goclaw_4c7540a5810249ce3f3ec9ba88b7fd98` (prefix `goclaw_`)

Fix: Settings → Update API key → paste đúng value.

### Test `chat.send` từ command line (debug)

```bash
# Verify key
curl -H "Authorization: Bearer goclaw_xxx..." \
  https://goclaw.tikz2svg.com/api/v1/skills | head -c 200

# Should return 200 + JSON skills list. 401 = key invalid.
```

### Agent đọc nhầm skill (output thiếu/sai format)

Triệu chứng: output có preamble `"Here is..."`, hoặc fence ` ```latex `, hoặc dùng tool `read_document` rồi fail.

Nguyên nhân: gpt-5.4 chọn nhầm built-in skill (`pdf`, `docx`, etc) thay vì `tex-ocr`.

Fix: Đảm bảo Bước 5 setup VPS đã chạy — disable built-in `pdf`/`docx`/etc cho tenant.

### Chậm bất thường (>120s/page)

- Check `read_image` builtin tool config trên Goclaw — provider chain phải trỏ `openai-codex-1` đang active
- Check ChatGPT Plus account còn quota
- Try `cloud-gemini` để rule out network issue

## Architecture references

| Component | File |
|-----------|------|
| Rust adapter | `src-tauri/src/agents/cloud_goclaw_api.rs` |
| Keychain accessor | `src-tauri/src/agents/keychain.rs` (`*_cloud_goclaw_api_key`) |
| Registry entry | `src-tauri/src/agents/registry.rs` (`CLOUD_GOCLAW_ID`) |
| Dispatcher arm | `src-tauri/src/ocr/dispatcher.rs::run_cloud_agent` |
| PDF concurrency cap | `src-tauri/src/commands.rs::pdf_page_concurrency` |
| Settings UI | `src/windows/settings/agents-tab.tsx` |
| Goclaw skill source | `bot-tex/skills-store/tex-ocr/1/SKILL.md` (separate repo) |

## What's out of scope (today)

- WS connection pooling — per-call lifecycle is intentional, matches reference `chatbot_goclaw.py`
- Streaming responses — `stream:true` not implemented; non-streaming OCR is fine
- Multi-Goclaw-instance support — 1 endpoint URL hardcoded for v1
- Per-tenant SnipTeX accounts — single user, single key

Re-open nếu cần.

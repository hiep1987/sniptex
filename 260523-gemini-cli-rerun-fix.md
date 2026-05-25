# Khắc phục: Rerun bằng Gemini CLI

**Ngày:** 2026-05-23  
**Trạng thái:** Đã fix nền tảng; đang harden theo plan `260524-1304-gemini-cli-clean-rerun-output-fix`

---

## Triệu chứng

- Click rerun → chọn `gemini-cli` → toast "Rerunning with gemini-cli…" hiện ra
- Frontend dispatch chain hoạt động đúng (console log xác nhận)
- Nhưng kết quả rerun sai hoặc thất bại

## Nguyên nhân gốc

**Gemini CLI giới hạn quyền đọc file trong workspace directory (lấy từ CWD của process).**

Luồng thực thi:

```
Tauri app (CWD = src-tauri/)
  → spawn gemini -p "...@\"/path/to/image.png\"" --approval-mode plan
  → Gemini CLI xác định workspace = src-tauri/
  → Image nằm ở ~/Library/Application Support/com.sniptex.app/images/
  → BỊ CHẶN: "Path not in workspace"
  → Gemini trả về message lỗi nhưng exit code = 0
  → Rust code coi như thành công, lấy message lỗi làm OCR output
```

Lỗi thứ hai: Gemini CLI rò rỉ thinking-mode tag (vd: `instruction`, `. . .94>thought`) vào dòng đầu stdout, gây nhiễu output.

## Cách khắc phục

### Fix 1: Stage ảnh vào workspace riêng cho Gemini CLI

**File:** `src-tauri/src/ocr/dispatcher.rs`

Thay vì đặt `current_dir($HOME)`, code hiện tạo workspace tạm:

```text
temp/sniptex/gemini-workspaces/{uuid}/input.png
```

Sau đó spawn Gemini với `current_dir(workspace)` và prompt dùng `@"input.png"`.

**Lý do:** `$HOME` giải quyết lỗi path nhưng mở rộng workspace quá lớn. Workspace riêng chỉ chứa ảnh OCR của lần gọi hiện tại, giảm rủi ro Gemini tool-loop đọc nhầm file khác.

### Fix 2: Dùng interactive-like text mode cho Gemini CLI

**File:** `src-tauri/src/agents/registry.rs`, `src-tauri/src/ocr/dispatcher.rs`

Gemini CLI được gọi với:

```text
gemini -p "Chuyển toàn bộ nội dung ảnh sang LaTeX. Chỉ xuất LaTeX, không giải thích @\"/absolute/path.png\"" \
  --skip-trust \
  --include-directories "/absolute/image/parent" \
  --output-format text \
  -e none
```

Live validation cho thấy `--output-format json`, pin model, `--approval-mode plan`, stage `input.png`, hoặc đặt `@path` ở dòng riêng đều có thể làm headless Gemini lệch khỏi hành vi TUI. Text mode với prompt một câu cùng dòng `@path` OCR đúng ảnh app-data `52f65375-3607-4fb0-a5be-fd25d0b3ddd3.png`.

Root cause cuối cùng không phải Gemini CLI nói chung, không phải `@file`, không phải agent path, và không phải model version. `MASTER_PROMPT` quá procedural cho `gemini -p` headless: classify category, format-by-category, examples, strict rules. Prompt đó đẩy model vào chế độ thực thi quy trình thay vì chỉ transcribe ảnh. Gemini CLI vì vậy dùng prompt riêng tối giản.

Lưu ý: Codex và Gemini Vision API vẫn dùng `MASTER_PROMPT`; Gemini CLI dùng prompt một câu. Hai contract này có thể tạo output structure khác nhau trên ảnh khó như bảng biến thiên, hình học, hệ trục, hoặc mixed text/math. Trước khi coi Gemini CLI ngang hàng, cần live test các fixture khó và consistency guard cross-agent.

### Fix 3: Reject unsafe Gemini output trước khi update history

**File:** `src-tauri/src/ocr/dispatcher.rs`, `src-tauri/src/ocr/consistency.rs`, `src/stores/history-store.ts`

Reject các trường hợp:

- response chứa tool/scaffold error rõ ràng
- response chứa marker lỗi CLI rõ ràng như `Error executing tool`, `Path not in workspace`, `Attempted path`
- response rỗng hoặc `[UNREADABLE]`
- rerun output có problem label khác row cũ, ví dụ `Câu 9` bị Gemini trả thành `Bài 5`
- rerun output có overlap quá thấp với text cũ, ví dụ Gemini trả đoạn đại số không liên quan cho ảnh Câu 9

`rerun_snip` vẫn strict: user chọn `gemini-cli` thì không fallback âm thầm sang Codex. Nếu Gemini fail, DB không update và History row cũ được giữ nguyên.

### Fix 3b: Keep Gemini CLI manual and guarded

**File:** `src-tauri/src/commands.rs`, `src/components/rerun-menu.tsx`, `src-tauri/src/agents/registry.rs`

Gemini CLI vẫn bị loại khỏi default fallback để app không tự rơi vào CLI khi Codex/Gemini API fail. History rerun vẫn có thể chọn Gemini CLI thủ công, nhưng kết quả phải qua consistency guard trước khi update DB.

Nếu Gemini CLI trả unrelated/stale content, rerun fail và row cũ giữ nguyên.

### Fix 4 (đã có từ trước): Tăng timeout

**File:** `src-tauri/src/ocr/dispatcher.rs`

```rust
const DISPATCH_TIMEOUT: Duration = Duration::from_secs(90);
```

**Lý do:** Gemini CLI p95 latency là 14–46s, timeout 30s gây fail thường xuyên với nội dung MIXED.

## Xác nhận

```
# Rust tests
cargo test --manifest-path src-tauri/Cargo.toml
pnpm exec tsc --noEmit
```

## File đã thay đổi

| File | Thay đổi |
|------|----------|
| `src-tauri/src/agents/registry.rs` | Gemini argv dùng JSON output và tắt extensions |
| `src-tauri/src/ocr/dispatcher.rs` | Parse JSON `.response`, reject unsafe output |
| `src-tauri/src/ocr/prompt.rs` | Prompt Gemini CLI tối giản, một câu |
| `src/components/rerun-menu.tsx` | Hiển thị Gemini CLI cho rerun thủ công |
| `src/stores/history-store.ts` | Rerun fail giữ row cũ và set error |
| `src-tauri/tests/rust/gemini_cli_output_test.rs` | Test JSON parsing và unsafe guards |

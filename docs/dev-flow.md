# Dev Flow — SnipTeX

Tài liệu này mô tả cách dev hằng ngày tương tác với CI/CD pipeline của
SnipTeX: khi nào CI chỉ verify, khi nào build installer, khi nào tag
nên là pre-release, và user vào landing thấy gì.

Đọc kèm `docs/releasing.md` (chi tiết kỹ thuật cho từng release).

---

## 4 phase chính

### 1. Hằng ngày — dev / fix / feature

```
git push origin main
        ↓
✅ .github/workflows/ci.yml chạy
   - pnpm tsc + vite build (JS)
   - cargo fmt + clippy + test (Rust matrix: ubuntu / macos-latest / windows-latest)
   - ❌ KHÔNG build DMG/MSI

✅ .github/workflows/deploy-pages.yml chạy NẾU có thay đổi:
   - site/**
   - .github/workflows/deploy-pages.yml
```

Mỗi commit lên `main` chỉ verify code không broken. **Không tốn time
build installer.** Cứ push thoải mái.

### 2. Khi muốn ra binary — tag `v*-dev`

Khi cần installer để test thật, share beta tester, hoặc bản thân muốn
dùng feature mới hằng ngày:

```bash
# Bump version trong 3 file (giữ đồng nhất)
# - package.json                         "version": "0.0.2"
# - src-tauri/tauri.conf.json            "version": "0.0.2"
# - src-tauri/Cargo.toml                 version = "0.0.2"

git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m "chore(release): bump to v0.0.2-dev"
git push origin main

# Tạo + push tag (workflow chỉ trigger trên tag bắt đầu bằng v)
git tag v0.0.2-dev
git push origin v0.0.2-dev
```

→ `.github/workflows/release.yml` trigger (~10-15 phút):

| Step | Output |
|------|--------|
| Build macos-latest (Apple Silicon) | `SnipTeX_0.0.2_aarch64.dmg`, `SnipTeX_aarch64.app.tar.gz`, `.sig` |
| Build windows-latest (x64) | `SnipTeX_0.0.2_x64_en-US.msi`, `SnipTeX_0.0.2_x64-setup.exe`, `.nsis.zip`, `.sig` |
| Checksums + latest.json | `checksums.txt`, `latest.json` (Tauri updater manifest) |
| Auto-create | **Draft release** "SnipTeX v0.0.2-dev" với tất cả artifacts attached |

### 3. Publish draft với pre-release flag

Vào `https://github.com/hiep1987/sniptex/releases`:

1. Thấy draft "SnipTeX v0.0.2-dev" → click **Edit**
2. (Optional) viết release notes
3. ✅ **Tick "Set as a pre-release"**
4. Click **Publish release**

Lý do tick pre-release:

- Auto-updater **bỏ qua pre-release mặc định** → user trên stable không
  bị auto-update lên dev build
- `/releases/latest` endpoint **không bao gồm pre-release** → các Cask
  formula / installer script trỏ tới `/releases/latest` không bị broken

### 4. User vào landing thấy gì

```
hiep1987.github.io/sniptex
        ↓
[ Download .dmg ] [ Download .msi ]
        ↓ (click)
/releases (browse-all page)
        ↓
SnipTeX v0.0.2-dev  [Pre-release badge]
        ↓ (click vào release)
Page release với DMG + MSI + checksums.txt attached
        ↓
User download asset phù hợp
```

Note: nút Download landing hiện tại trỏ `/releases` (browse-all) chứ
**không** `/releases/latest`. Lý do: trong giai đoạn dev (chỉ có
pre-release), `/latest` sẽ 404. Sau khi v0.1.0 stable ship, có thể flip
landing button về `/releases/latest/download/SnipTeX_*.dmg` để one-click.

---

## Launch milestone — v0.1.0

Khi cut v0.1.0 (sau Phase 14 demo video + Phase 15 marketing + Apple Dev
fund đủ):

```bash
# Cập nhật 3 file: version = "0.1.0"
git commit -am "chore(release): cut v0.1.0"
git push origin main

git tag v0.1.0
git push origin v0.1.0
```

CI build → draft release. Tại bước Publish:

- ❌ **KHÔNG tick pre-release** (đây là stable)
- ✅ Tick "Set as the latest release"
- Click **Publish release**

Hiệu quả:

| Where | Before | After v0.1.0 publish |
|-------|--------|----------------------|
| `/releases/latest` | 404 (chỉ pre-release tồn tại) | v0.1.0 |
| Auto-updater feed | Returns "no update" | Detects v0.1.0; user dev/0.0.x sẽ thấy update prompt |
| Landing nút Download | Vào `/releases` browse-all | Có thể flip về `/releases/latest/download/...` direct |
| Cask formula `Casks/sniptex.rb` | SHA256 từ Phase 11 local build (stale) | Cập nhật SHA256 mới từ CI build, submit homebrew-cask PR (Phase 15) |

Sau v0.1.0, mọi pre-release tag tiếp theo (v0.1.1-dev, v0.2.0-alpha, ...)
vẫn dùng quy trình Phase 2-3 ở trên.

---

## Tag naming convention

| Tag | Mục đích | Pre-release? | Auto-updater visible? |
|-----|----------|--------------|-----------------------|
| `v0.0.x-dev` | Dev iteration, chia sẻ cho beta tester / chính bản thân | ✅ | ❌ |
| `v0.0.x-alpha` | Public alpha, tester biết chấp nhận risk | ✅ | ❌ |
| `v0.0.x-beta` | Public beta, gần stable | ✅ | ❌ |
| `v0.1.0`, `v0.1.1`, `v1.0.0` | Stable release | ❌ | ✅ |
| `v0.1.0-rc1`, `v0.1.0-rc2` | Release candidate trước launch | ✅ | ❌ |

SemVer pre-release suffix (`-dev`, `-alpha`, ...) cũng được Tauri's
updater hiểu đúng — nó so version theo SemVer chuẩn.

---

## Anti-pattern

❌ **KHÔNG cut v0.1.0 chỉ vì muốn user tải được** trong khi sản phẩm
chưa stable. Một khi v0.1.0 đã shipped, mọi fix sau phải bump
v0.1.1, v0.1.2... đầy CHANGELOG. Dùng pre-release tag để giữ v0.1.0
cho launch milestone thật sự.

❌ **KHÔNG tick "Set as the latest release" với pre-release tag.** Sẽ
làm auto-updater push dev build cho user stable → user bị broken
features chưa test xong.

❌ **KHÔNG xóa tag đã publish.** Người đã download asset từ tag đó sẽ
mất reference. Nếu tag broken, cut tag mới (vd. v0.0.2-dev hỏng → cut
v0.0.3-dev), không retag.

---

## Tham chiếu

- `docs/releasing.md` — quy trình kỹ thuật cho từng release (bump
  version, signing key rotation, troubleshooting)
- `docs/install-guide.md` — install steps cho end users (Gatekeeper,
  SmartScreen, Smart App Control)
- `.github/workflows/ci.yml` — verify workflow (PR + push to main)
- `.github/workflows/release.yml` — tag-triggered release workflow
- `.github/workflows/deploy-pages.yml` — landing page deploy (site/**)
- `plans/260520-0603-sniptex-tauri-mvp-v1/` — implementation plan

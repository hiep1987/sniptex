#!/usr/bin/env bash
# SnipTeX — Prompt validation script
# Tests master OCR prompt against Gemini CLI and Codex on a set of images.
#
# Usage:
#   ./test-prompt.sh <path-to-images-dir>
#   ./test-prompt.sh ~/test-images/sgk
#
# Output:
#   results/<agent>/<image-name>.txt   — raw output from each agent
#   results/summary.csv                 — latency, length, detected type, status
#   results/comparison.md               — side-by-side review document
#
# Requirements (Mac):
#   - gemini CLI installed:  gemini --version
#   - codex CLI installed:   codex --version
#   - GNU coreutils:         brew install coreutils
#     (needed for gdate with nanosecond precision)

set -uo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Config
# ─────────────────────────────────────────────────────────────────────────────

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly RESULTS_DIR="${SCRIPT_DIR}/results"
readonly TIMEOUT_SEC=60
readonly SUPPORTED_EXTS=("png" "jpg" "jpeg" "webp")

# Colors (Mac-friendly)
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[0;33m'
readonly BLUE='\033[0;34m'
readonly BOLD='\033[1m'
readonly NC='\033[0m'

# ─────────────────────────────────────────────────────────────────────────────
# Master prompt (mirror of src-tauri/src/ocr/prompt.rs)
# ─────────────────────────────────────────────────────────────────────────────

readonly MASTER_PROMPT='You are an OCR engine. Convert the image to text following these rules.

DETECTION (internal, do not emit):
Silently classify the image into ONE category, then use that category ONLY to choose the output format below. Do NOT print the category name. Do NOT prefix or suffix your output with "EQUATION_ONLY", "TABLE_ONLY", or "MIXED".
- EQUATION_ONLY: image contains only one or more math expressions, no surrounding text
- TABLE_ONLY: image contains only a table, no surrounding text
- MIXED: any combination of text, equations, tables, lists

OUTPUT FORMAT BY CATEGORY:

If EQUATION_ONLY:
  Output ONLY raw LaTeX without $ delimiters.
  Multiple equations: separate with \\
  Example: \int_0^1 x^2 \, dx = \frac{1}{3}

If TABLE_ONLY:
  Decide between two sub-formats based on the visual structure:

  SIMPLE GRID (no merged cells, no header hierarchy, no row/column spans):
    Output GitHub Markdown table.
    Example: | a | b |
|---|---|
| 1 | 2 |
    Inside table cells: only wrap mathematical variables, fractions, equations, and symbolic expressions in $...$. Plain numeric intervals like [40; 45), plain integers, plain percentages (15%), and ordinary words MUST remain unwrapped.

  COMPLEX GRID (any merged cells — rowspan, colspan, multi-tier headers, cells that span vertically or horizontally):
    Output raw LaTeX tabular directly, NOT Markdown. GitHub Markdown cannot express merged cells; emitting a flattened MD grid would lose structural information.
    Use:
      - \\begin{tabular}{|c|c|...|} ... \\end{tabular} with a column count matching the bottom-most (most-divided) header row.
      - \\multirow{N}{*}{content} for cells that span N rows vertically.
      - \\multicolumn{N}{|c|}{content} for cells that span N columns horizontally.
      - \\cline{a-b} after a row when only columns a through b have a horizontal rule (used under a multicolumn header).
      - \\hline between full-width row separators.
      - Do NOT wrap cell contents in $...$ unless the cell genuinely contains math. Plain header text, plain integers, plain labels stay unwrapped.
    Example (header "Group" spans 2 rows, header "Counts" spans 2 columns over "Type I" and "Type II"):
\\begin{tabular}{|c|c|c|}
\\hline
\\multirow{2}{*}{Group} & \\multicolumn{2}{|c|}{Counts} \\\\
\\cline{2-3} & Type I & Type II \\\\
\\hline
A & 1 & 2 \\\\
\\hline
\\end{tabular}

If MIXED:
  Output Markdown.
  Inline math: $...$
  Display math: $$...$$
  Tables: GitHub Markdown
  Code: fenced ```lang blocks
  Preserve original structure (headings, lists, paragraphs)

STRICT RULES:
- Preserve Vietnamese diacritics exactly (ă â ê ô ơ ư đ và dấu thanh)
- Do NOT translate. Keep source language.
- Do NOT add explanations, preambles ("Here is...", "Sure!")
- Do NOT wrap output in ```markdown or ```latex fences
- Do NOT add sign-offs ("Let me know if...")
- If unreadable, output exactly: [UNREADABLE]
- Math symbols: use standard LaTeX (\alpha not α, \int not ∫)
- Preserve fractions as \frac{}{}, exponents as ^{}, subscripts as _{}

Begin output now:'

# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

log_info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()      { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error()   { echo -e "${RED}[ERR]${NC}   $*" >&2; }
log_header()  { echo -e "\n${BOLD}━━━ $* ━━━${NC}\n"; }

# millisecond timestamp (Mac requires gdate from coreutils for ms precision)
now_ms() {
    if command -v gdate &>/dev/null; then
        gdate +%s%3N
    else
        # Fallback: second precision only
        echo "$(date +%s)000"
    fi
}

check_deps() {
    local missing=()
    
    if ! command -v gemini &>/dev/null; then
        missing+=("gemini")
    fi
    if ! command -v codex &>/dev/null; then
        missing+=("codex")
    fi
    if ! command -v gdate &>/dev/null; then
        log_warn "gdate not found. Install for millisecond timing: brew install coreutils"
    fi
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing CLI tools: ${missing[*]}"
        log_error "Install Gemini:  npm install -g @google/gemini-cli"
        log_error "Install Codex:   npm install -g @openai/codex"
        exit 1
    fi
    
    log_ok "Gemini version: $(gemini --version 2>&1 | head -1)"
    log_ok "Codex version:  $(codex --version 2>&1 | head -1)"
}

# Detect output type using same heuristic as Rust smart_format
detect_type() {
    local content="$1"
    local trimmed
    trimmed="$(echo "$content" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
    
    # Heuristic 1: Markdown table
    if echo "$trimmed" | grep -qE '\|[-]+\|'; then
        local non_table_lines
        non_table_lines=$(echo "$trimmed" | grep -v '|' | grep -v '^$' | wc -l | tr -d ' ')
        if [[ "$non_table_lines" == "0" ]]; then
            echo "TABLE_ONLY"
            return
        fi
        echo "MIXED"
        return
    fi
    
    # Heuristic 2: $ delimiter or markdown structure
    if echo "$trimmed" | grep -qE '\$|^# |^- ' || \
       echo "$trimmed" | grep -qE $'\n\n'; then
        echo "MIXED"
        return
    fi
    
    # Heuristic 3: LaTeX commands without much natural text
    local has_latex=0
    if echo "$trimmed" | grep -qE '\\frac|\\int|\\sum|\^|_'; then
        has_latex=1
    fi
    
    local natural_words
    natural_words=$(echo "$trimmed" | tr -cs 'a-zA-Z' '\n' | awk 'length > 2' | grep -v '\\' | wc -l | tr -d ' ')
    
    if [[ "$has_latex" == "1" ]] && [[ "$natural_words" -lt 3 ]]; then
        echo "EQUATION_ONLY"
        return
    fi
    
    echo "MIXED"
}

# Sanitize for CSV (escape commas, newlines, quotes)
csv_escape() {
    local s="$1"
    s="${s//\"/\"\"}"
    echo "\"$s\""
}

# Truncate string for preview in summary
preview() {
    local s="$1"
    local max="${2:-80}"
    s="${s//$'\n'/ }"
    if [[ ${#s} -gt $max ]]; then
        echo "${s:0:$max}..."
    else
        echo "$s"
    fi
}

# ─────────────────────────────────────────────────────────────────────────────
# Agent runners
# ─────────────────────────────────────────────────────────────────────────────

run_gemini() {
    local image_path="$1"
    local output_file="$2"
    
    # Gemini CLI: -p with @ syntax for image
    # Add --yolo to skip confirmations, -m flash for speed/cost
    local start end elapsed
    start=$(now_ms)
    
    # Quote the @ path so spaces and embedded '@' (e.g. '@2x' in CleanShot names) parse correctly.
    if timeout "$TIMEOUT_SEC" gemini -p "${MASTER_PROMPT}
@\"${image_path}\"" --yolo > "$output_file" 2>&1; then
        end=$(now_ms)
        elapsed=$((end - start))
        echo "$elapsed"
        return 0
    else
        end=$(now_ms)
        elapsed=$((end - start))
        echo "$elapsed"
        return 1
    fi
}

run_codex() {
    local image_path="$1"
    local output_file="$2"

    # Codex CLI: exec mode with image input.
    # --skip-git-repo-check: required when CWD is not a git repo
    # --output-last-message: write only the final assistant message to a file (no session header/footer)
    local last_msg_file="${output_file%.raw.txt}.last.txt"
    local start end elapsed
    start=$(now_ms)

    if timeout "$TIMEOUT_SEC" codex exec \
        --skip-git-repo-check \
        --image "$image_path" \
        --output-last-message "$last_msg_file" \
        -- "${MASTER_PROMPT}" > "$output_file" 2>&1; then
        end=$(now_ms)
        elapsed=$((end - start))
        # Replace verbose stdout with the clean last-message content for downstream post_process.
        if [[ -s "$last_msg_file" ]]; then
            cp "$last_msg_file" "$output_file"
        fi
        echo "$elapsed"
        return 0
    else
        end=$(now_ms)
        elapsed=$((end - start))
        echo "$elapsed"
        return 1
    fi
}

# Strip common preambles (lightweight version of Rust post-process)
post_process() {
    local content="$1"

    # Remove Gemini CLI startup banners that appear on every -p invocation.
    content=$(echo "$content" | sed -E '
        /^YOLO mode is enabled\./d
        /^Ripgrep is not available\./d
    ')

    # Remove common preambles in first line
    content=$(echo "$content" | sed -E '1{
        /^(Here'\''s|Here is|Sure!|Sure,|Of course|Certainly|Below is|The image shows|Đây là|Sau đây là|Dưới đây)/d
    }')
    
    # Remove markdown/latex code fences
    content=$(echo "$content" | sed -E '
        /^```(markdown|latex|md|tex)[[:space:]]*$/d
        /^```[[:space:]]*$/d
    ')
    
    # Trim leading/trailing blank lines (Mac: tac unavailable, use tail -r)
    local rev_cmd
    if command -v tac &>/dev/null; then
        rev_cmd="tac"
    else
        rev_cmd="tail -r"
    fi
    content=$(echo "$content" | awk 'NF {p=1} p' | eval "$rev_cmd" | awk 'NF {p=1} p' | eval "$rev_cmd")
    
    echo "$content"
}

# ─────────────────────────────────────────────────────────────────────────────
# Main test loop
# ─────────────────────────────────────────────────────────────────────────────

run_tests() {
    local images_dir="$1"
    
    # Find images
    local images=()
    while IFS= read -r -d '' f; do
        images+=("$f")
    done < <(find "$images_dir" -type f \( \
        -iname "*.png" -o -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.webp" \
    \) -print0 | sort -z)
    
    local total=${#images[@]}
    if [[ $total -eq 0 ]]; then
        log_error "No images found in: $images_dir"
        exit 1
    fi
    
    log_info "Found $total images"
    
    # Setup output dirs
    mkdir -p "${RESULTS_DIR}/gemini" "${RESULTS_DIR}/codex" "${RESULTS_DIR}/raw"
    
    # CSV header
    local summary_csv="${RESULTS_DIR}/summary.csv"
    echo "image,agent,status,latency_ms,output_chars,detected_type,preview" > "$summary_csv"
    
    # Stats
    local gem_ok=0 gem_fail=0 gem_total_ms=0
    local cdx_ok=0 cdx_fail=0 cdx_total_ms=0
    
    local idx=0
    for img in "${images[@]}"; do
        idx=$((idx + 1))
        local base
        base=$(basename "$img")
        local stem="${base%.*}"
        
        echo -e "\n${BOLD}[$idx/$total]${NC} $base"
        
        # ─── Gemini ───
        local gem_out="${RESULTS_DIR}/gemini/${stem}.txt"
        local gem_raw="${RESULTS_DIR}/raw/${stem}.gemini.raw.txt"
        
        log_info "Running Gemini..."
        local gem_ms
        if gem_ms=$(run_gemini "$img" "$gem_raw"); then
            local cleaned
            cleaned=$(post_process "$(cat "$gem_raw")")
            echo "$cleaned" > "$gem_out"
            
            local chars=${#cleaned}
            local dtype
            dtype=$(detect_type "$cleaned")
            
            log_ok "Gemini: ${gem_ms}ms, ${chars} chars, ${dtype}"
            echo "$(csv_escape "$base"),gemini,ok,${gem_ms},${chars},${dtype},$(csv_escape "$(preview "$cleaned")")" >> "$summary_csv"
            
            gem_ok=$((gem_ok + 1))
            gem_total_ms=$((gem_total_ms + gem_ms))
        else
            log_error "Gemini failed (${gem_ms}ms)"
            echo "$(csv_escape "$base"),gemini,fail,${gem_ms},0,N/A,$(csv_escape "$(preview "$(cat "$gem_raw")")")" >> "$summary_csv"
            gem_fail=$((gem_fail + 1))
        fi
        
        # ─── Codex ───
        local cdx_out="${RESULTS_DIR}/codex/${stem}.txt"
        local cdx_raw="${RESULTS_DIR}/raw/${stem}.codex.raw.txt"
        
        log_info "Running Codex..."
        local cdx_ms
        if cdx_ms=$(run_codex "$img" "$cdx_raw"); then
            local cleaned
            cleaned=$(post_process "$(cat "$cdx_raw")")
            echo "$cleaned" > "$cdx_out"
            
            local chars=${#cleaned}
            local dtype
            dtype=$(detect_type "$cleaned")
            
            log_ok "Codex: ${cdx_ms}ms, ${chars} chars, ${dtype}"
            echo "$(csv_escape "$base"),codex,ok,${cdx_ms},${chars},${dtype},$(csv_escape "$(preview "$cleaned")")" >> "$summary_csv"
            
            cdx_ok=$((cdx_ok + 1))
            cdx_total_ms=$((cdx_total_ms + cdx_ms))
        else
            log_error "Codex failed (${cdx_ms}ms)"
            echo "$(csv_escape "$base"),codex,fail,${cdx_ms},0,N/A,$(csv_escape "$(preview "$(cat "$cdx_raw")")")" >> "$summary_csv"
            cdx_fail=$((cdx_fail + 1))
        fi
    done
    
    # ─── Generate comparison.md ───
    local comp_md="${RESULTS_DIR}/comparison.md"
    {
        echo "# SnipTeX Prompt Test — Side-by-side Comparison"
        echo
        echo "Generated: $(date '+%Y-%m-%d %H:%M:%S')"
        echo "Images dir: \`$images_dir\`"
        echo "Total images: $total"
        echo
        echo "## Summary"
        echo
        echo "| Agent | OK | Fail | Avg latency | Success rate |"
        echo "|---|---|---|---|---|"
        local gem_avg=0 cdx_avg=0
        if [[ $gem_ok -gt 0 ]]; then gem_avg=$((gem_total_ms / gem_ok)); fi
        if [[ $cdx_ok -gt 0 ]]; then cdx_avg=$((cdx_total_ms / cdx_ok)); fi
        local gem_rate=$((gem_ok * 100 / total))
        local cdx_rate=$((cdx_ok * 100 / total))
        echo "| Gemini | $gem_ok | $gem_fail | ${gem_avg}ms | ${gem_rate}% |"
        echo "| Codex  | $cdx_ok | $cdx_fail | ${cdx_avg}ms | ${cdx_rate}% |"
        echo
        echo "## Side-by-side outputs"
        echo
        for img in "${images[@]}"; do
            local base
            base=$(basename "$img")
            local stem="${base%.*}"
            local gem_file="${RESULTS_DIR}/gemini/${stem}.txt"
            local cdx_file="${RESULTS_DIR}/codex/${stem}.txt"
            
            echo "### $base"
            echo
            echo "![${base}](${img})"
            echo
            echo "**Gemini:**"
            echo
            echo '```'
            [[ -f "$gem_file" ]] && cat "$gem_file" || echo "[FAILED]"
            echo '```'
            echo
            echo "**Codex:**"
            echo
            echo '```'
            [[ -f "$cdx_file" ]] && cat "$cdx_file" || echo "[FAILED]"
            echo '```'
            echo
            echo "---"
            echo
        done
    } > "$comp_md"
    
    # ─── Final report ───
    log_header "Test Complete"
    echo "Total images:    $total"
    echo
    echo -e "${BOLD}Gemini:${NC}"
    echo "  Success: $gem_ok / $total ($(( gem_ok * 100 / total ))%)"
    if [[ $gem_ok -gt 0 ]]; then
        echo "  Avg latency: $((gem_total_ms / gem_ok))ms"
    fi
    echo
    echo -e "${BOLD}Codex:${NC}"
    echo "  Success: $cdx_ok / $total ($(( cdx_ok * 100 / total ))%)"
    if [[ $cdx_ok -gt 0 ]]; then
        echo "  Avg latency: $((cdx_total_ms / cdx_ok))ms"
    fi
    echo
    echo "Outputs:"
    echo "  ${RESULTS_DIR}/gemini/*.txt"
    echo "  ${RESULTS_DIR}/codex/*.txt"
    echo "  ${RESULTS_DIR}/summary.csv"
    echo "  ${RESULTS_DIR}/comparison.md  ← open this to review side-by-side"
    echo
    log_info "Tip: open the comparison report in your browser:"
    echo "  open ${comp_md}"
}

# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

main() {
    if [[ $# -lt 1 ]]; then
        echo "Usage: $0 <path-to-images-dir>"
        echo ""
        echo "Example:"
        echo "  $0 ~/test-images/sgk"
        echo ""
        echo "Supports: ${SUPPORTED_EXTS[*]}"
        exit 1
    fi
    
    local images_dir="$1"
    if [[ ! -d "$images_dir" ]]; then
        log_error "Directory not found: $images_dir"
        exit 1
    fi
    
    log_header "SnipTeX Prompt Test"
    log_info "Images dir: $images_dir"
    log_info "Results dir: $RESULTS_DIR"
    log_info "Timeout: ${TIMEOUT_SEC}s per call"
    
    check_deps
    run_tests "$images_dir"
}

main "$@"

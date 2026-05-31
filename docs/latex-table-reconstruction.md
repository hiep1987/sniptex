# LaTeX Table Reconstruction

This document tracks table shapes that SnipTeX already reconstructs when `Copy as TeX` receives flattened Markdown from OCR agents.

## Pipeline

1. OCR agent returns text.
2. `post_process` preserves raw complex LaTeX tables when they already contain `\multirow`, `\multicolumn`, or `\cline`.
3. `markdown_tables_to_latex_tabular` converts Markdown tables to LaTeX.
4. `tabular_complex_grid` runs first for known flattened complex-grid shapes.
5. Simple tables fall back to generic Markdown table conversion.

Source files:

- `src-tauri/src/ocr/postprocess.rs`
- `src-tauri/src/ocr/tabular.rs`
- `src-tauri/src/ocr/tabular_complex_grid.rs`
- `src-tauri/tests/rust/ocr_tabular_test.rs`

## Supported Groups

### 1. Simple Grid

**Shape:** no merged cells, no hierarchical header, no row/column spans.

**Typical OCR Markdown:**

```md
| a | b |
|---|---|
| 1 | 2 |
```

**TeX output:**

```tex
\begin{tabular}{|l|l|}
\hline
a & b \\ \hline
1 & 2 \\ \hline
\end{tabular}
```

**Tests:** inline unit tests in `src-tauri/src/ocr/tabular.rs`.

### 2. Two-Level Column Header

**Shape:** first header row has leading row-span labels and one parent header spanning multiple trailing columns.

**Known fixture:** Vietnamese machine groups table: `Nhóm`, `Số máy mỗi nhóm`, `Loại I`, `Loại II`.

**Typical flattened OCR Markdown:**

```md
| Nhóm | Số máy mỗi nhóm | Số máy trong từng nhóm để sản xuất một đơn vị sản phẩm | |
|---|---|---|---|
| | | Loại I | Loại II |
| A | 10 | 2 | 2 |
| B | 4 | 0 | 2 |
| C | 12 | 2 | 4 |
```

**TeX output shape:**

```tex
\begin{tabular}{|c|c|c|c|}
\hline
\multirow{2}{*}{Nhóm} & \multirow{2}{*}{\begin{tabular}{c}Số máy mỗi\\nhóm\end{tabular}} & \multicolumn{2}{c|}{...} \\
\cline{3-4}
 &  & Loại I & Loại II \\
\hline
...
\end{tabular}
```

**Handled variants:**

- Missing trailing header cell from `cloud-gemini`.
- Center/right alignment separators from `cloud-goclaw`.
- Bold subheaders from `gemini-cli`.
- Math-wrapped row labels such as `$A$` are preserved; plain `A` labels are wrapped.

**Tests:** `reconstructs_cloud_mistral_flattened_complex_grid`, `reconstructs_cloud_gemini_missing_trailing_header_cell`, `reconstructs_cloud_goclaw_flattened_complex_grid`, `reconstructs_gemini_cli_bold_subheaders`.

### 3. Title Row Spanning All Columns

**Shape:** first OCR row contains a title in the first cell and empty cells for the remaining columns.

**Known fixture:** `Country List`.

**Typical flattened OCR Markdown:**

```md
| Country List |  |  |
|---|---|---|
| Country Name or Area Name | ISO ALPHA 2 Code | ISO ALPHA 3 |
| Afghanistan | AF | AFG |
```

**TeX output shape:**

```tex
\begin{tabular}{|l|c|c|}
\hline
\multicolumn{3}{|c|}{Country List} \\
\hline
Country Name or Area Name & ISO ALPHA 2 Code & ISO ALPHA 3 \\
\hline
...
\hline
\end{tabular}
```

**Tests:** `reconstructs_title_row_spanning_all_columns`.

### 4. Raw Complex LaTeX From Agent

**Shape:** agent already outputs a complex `tabular` with merge commands.

**Detection:** raw LaTeX table body contains at least one of:

- `\multirow`
- `\multicolumn`
- `\cline`

**Behavior:** preserve the raw table instead of flattening to Markdown.

**Tests:** `preserves_tabular_with_multirow_and_multicolumn`, `preserves_nested_tabular_inside_multirow_header`.

## Adding New Groups

1. Capture real OCR Markdown output from the agent, especially `cloud-mistral`.
2. Add one failing test to `src-tauri/tests/rust/ocr_tabular_test.rs`.
3. Implement a narrow detector in `tabular_complex_grid.rs`.
4. Return `None` when the pattern is uncertain so simple-table conversion remains unchanged.
5. Run:

```bash
cargo test --test ocr_tabular
cargo test
```

## Guardrails

- Do not make the complex-grid detector guess arbitrary merged cells.
- Prefer fixture-driven heuristics over broad string rewrites.
- Preserve raw LaTeX complex tables when agents already emit them.
- Keep conversion deterministic and offline; no API calls in `Copy as TeX`.
- Do not change the `cloud-mistral` OCR API path to chat completion for this feature.

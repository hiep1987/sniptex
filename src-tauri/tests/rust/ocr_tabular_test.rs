use sniptex_lib::ocr::markdown_tables_to_latex_tabular;

const LONG_HEADER: &str = "Số máy trong từng nhóm để sản xuất một đơn vị sản phẩm";
const LONG_HEADER_TEX: &str =
    "\\begin{tabular}{c}Số máy trong từng nhóm\\\\để sản xuất một đơn vị sản phẩm\\end{tabular}";

#[test]
fn reconstructs_cloud_mistral_flattened_complex_grid() {
    let md = format!(
        "|  Nhóm | Số máy mỗi nhóm | {LONG_HEADER}  |   |\n\
         | --- | --- | --- | --- |\n\
         |   |   |  Loại I | Loại II  |\n\
         |  A | 10 | 2 | 2  |\n\
         |  B | 4 | 0 | 2  |\n\
         |  C | 12 | 2 | 4  |\n"
    );
    let tex = markdown_tables_to_latex_tabular(&md);

    assert!(tex.contains("\\multirow{2}{*}{Nhóm}"), "got: {tex}");
    assert!(
        tex.contains("\\multirow{2}{*}{\\begin{tabular}{c}Số máy mỗi\\\\nhóm\\end{tabular}}"),
        "got: {tex}"
    );
    assert!(
        tex.contains(&format!("\\multicolumn{{2}}{{c|}}{{{LONG_HEADER_TEX}}}")),
        "got: {tex}"
    );
    assert!(!tex.contains("\\multicolumn{2}{|c|}"), "got: {tex}");
    assert!(tex.contains("\\cline{3-4}"), "got: {tex}");
    assert!(tex.contains("$A$ & 10 & 2 & 2"), "got: {tex}");
}

#[test]
fn reconstructs_cloud_gemini_missing_trailing_header_cell() {
    let md = format!(
        "| Nhóm | Số máy mỗi nhóm | {LONG_HEADER} |\n\
         |---|---|---|\n\
         |  |  | Loại I | Loại II |\n\
         | A | 10 | 2 | 2 |\n\
         | B | 4 | 0 | 2 |\n\
         | C | 12 | 2 | 4 |\n"
    );
    let tex = markdown_tables_to_latex_tabular(&md);

    assert!(tex.starts_with("\\begin{tabular}{|c|c|c|c|}"), "got: {tex}");
    assert!(tex.contains("\\multirow{2}{*}{Nhóm}"), "got: {tex}");
    assert!(tex.contains("\\multicolumn{2}{c|}"), "got: {tex}");
    assert!(tex.contains(" &  & Loại I & Loại II"), "got: {tex}");
    assert!(tex.contains("$C$ & 12 & 2 & 4"), "got: {tex}");
}

#[test]
fn reconstructs_cloud_goclaw_flattened_complex_grid() {
    let md = format!(
        "| Nhóm | Số máy mỗi nhóm | {LONG_HEADER} |  |\n\
         |---|---:|---:|---:|\n\
         |  |  | Loại I | Loại II |\n\
         | A | 10 | 2 | 2 |\n\
         | B | 4 | 0 | 2 |\n\
         | C | 12 | 2 | 4 |\n"
    );
    let tex = markdown_tables_to_latex_tabular(&md);

    assert!(tex.contains("\\multirow{2}{*}{Nhóm}"), "got: {tex}");
    assert!(tex.contains("\\cline{3-4}"), "got: {tex}");
    assert!(tex.contains("$B$ & 4 & 0 & 2"), "got: {tex}");
}

#[test]
fn reconstructs_gemini_cli_bold_subheaders() {
    let md = format!(
        "| Nhóm | Số máy mỗi nhóm | {LONG_HEADER} | |\n\
         | :---: | :---: | :---: | :---: |\n\
         | | | **Loại I** | **Loại II** |\n\
         | $A$ | 10 | 2 | 2 |\n\
         | $B$ | 4 | 0 | 2 |\n\
         | $C$ | 12 | 2 | 4 |\n"
    );
    let tex = markdown_tables_to_latex_tabular(&md);

    assert!(tex.contains(" &  & Loại I & Loại II"), "got: {tex}");
    assert!(!tex.contains("**Loại"), "got: {tex}");
    assert!(tex.contains("$A$ & 10 & 2 & 2"), "got: {tex}");
}

#[test]
fn reconstructs_title_row_spanning_all_columns() {
    let md = concat!(
        "| Country List |  |  |\n",
        "|---|---|---|\n",
        "| Country Name or Area Name | ISO ALPHA 2 Code | ISO ALPHA 3 |\n",
        "| Afghanistan | AF | AFG |\n",
        "| Aland Islands | AX | ALA |\n",
        "| Albania | AL | ALB |\n",
        "| Algeria | DZ | DZA |\n",
        "| American Samoa | AS | ASM |\n",
        "| Andorra | AD | AND |\n",
        "| Angola | AO | AGO |\n",
    );
    let tex = markdown_tables_to_latex_tabular(md);

    assert!(tex.starts_with("\\begin{tabular}{|l|c|c|}"), "got: {tex}");
    assert!(
        tex.contains("\\multicolumn{3}{|c|}{Country List}"),
        "got: {tex}"
    );
    assert!(!tex.contains("Country List &  &"), "got: {tex}");
    assert!(
        tex.contains("Country Name or Area Name & ISO ALPHA 2 Code & ISO ALPHA 3"),
        "got: {tex}"
    );
    assert!(tex.contains("Angola & AO & AGO"), "got: {tex}");
}

#[test]
fn converts_round3_table_only_codex_fixtures_to_tabular() {
    let fixtures = [
        (
            "15.01.25",
            include_str!(
                "../../../plans/results-round3-table-only/codex/CleanShot 2026-05-20 at 15.01.25@2x.txt"
            ),
        ),
        (
            "05.32.35",
            include_str!(
                "../../../plans/results-round3-table-only/codex/CleanShot 2026-05-21 at 05.32.35@2x.txt"
            ),
        ),
        (
            "05.32.54",
            include_str!(
                "../../../plans/results-round3-table-only/codex/CleanShot 2026-05-21 at 05.32.54@2x.txt"
            ),
        ),
        (
            "05.33.01",
            include_str!(
                "../../../plans/results-round3-table-only/codex/CleanShot 2026-05-21 at 05.33.01@2x.txt"
            ),
        ),
        (
            "05.33.07",
            include_str!(
                "../../../plans/results-round3-table-only/codex/CleanShot 2026-05-21 at 05.33.07@2x.txt"
            ),
        ),
        (
            "05.36.32",
            include_str!(
                "../../../plans/results-round3-table-only/codex/CleanShot 2026-05-21 at 05.36.32@2x.txt"
            ),
        ),
        (
            "05.37.34",
            include_str!(
                "../../../plans/results-round3-table-only/codex/CleanShot 2026-05-21 at 05.37.34@2x.txt"
            ),
        ),
        (
            "05.37.53",
            include_str!(
                "../../../plans/results-round3-table-only/codex/CleanShot 2026-05-21 at 05.37.53@2x.txt"
            ),
        ),
        (
            "05.38.29",
            include_str!(
                "../../../plans/results-round3-table-only/codex/CleanShot 2026-05-21 at 05.38.29@2x.txt"
            ),
        ),
    ];

    for (name, md) in fixtures {
        let tex = markdown_tables_to_latex_tabular(md);
        assert!(
            tex.contains("\\begin{tabular}") && tex.contains("\\end{tabular}"),
            "{name} did not produce a tabular environment: {tex}"
        );
        assert!(tex.contains("\\hline"), "{name} missing hline: {tex}");
        assert!(
            !tex.contains("|---"),
            "{name} still has Markdown separator: {tex}"
        );
        assert_ne!(tex.trim(), md.trim(), "{name} was not converted");
    }
}

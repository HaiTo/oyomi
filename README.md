# oyomi

[![CI](https://github.com/HaiTo/oyomi/actions/workflows/ci.yml/badge.svg)](https://github.com/HaiTo/oyomi/actions/workflows/ci.yml)

Render Office documents (xlsx / docx / pptx) as deterministic text, so that `git diff` shows what actually changed.

The name is *oyomi*, written お読み: Japanese for a reading of something, where the leading *o* doubles as Office.

日本語版は [README.ja.md](README.ja.md) にあります。

## The problem

git treats Office files as opaque blobs and reports `Binary files ... differ`. If your specifications live in Excel or Word, nobody can review a change in a merge request.

The usual workarounds stop halfway.

`soffice --headless --convert-to csv` converts **only the first sheet**. On a three-sheet workbook whose content sits in the third sheet, the CSV came out as 4 lines and the 620-cell sheet was gone. It also takes about 1.5 seconds per file once warm, and over 10 seconds on the first run.

openpyxl reads every sheet, but it is slower and leaves the layout to you.

Neither of them asks the question that matters for review: does the output diff well?

## Install

```console
$ cargo install --path .
```

## Usage

```console
$ oyomi catalog.xlsx
Cover!A1	Document
Cover!F1	Job Catalog
Items!B62	JOB900
...

$ oyomi --list catalog.xlsx
Cover	4x6	8 cells
History	3x5	9 cells
Items	124x5	620 cells

$ oyomi --sheet Items catalog.xlsx
```

| Option | Behaviour |
|---|---|
| (none) | One line per cell, keyed by address: `Sheet!B62<TAB>value`. Use it to locate something |
| `--row` | One line per row, with no position in the key. Use it for diffs |
| `--list` | Sheet names, dimensions and non-empty cell counts only |
| `--sheet NAME` | That sheet only |

docx renders one line per paragraph and per table cell, pptx one line per paragraph with the slide number.

## As a git textconv driver

```console
$ cat .gitattributes
*.xlsx diff=oyomi
*.xlsm diff=oyomi
*.docx diff=oyomi
*.pptx diff=oyomi

$ git config diff.oyomi.textconv "oyomi --row"
$ git config diff.oyomi.cachetextconv true
```

`git diff --stat` still reports byte counts, because git does not run textconv for stat output.

## Why the key carries no position

This is most of what the tool is.

The first version emitted `Sheet!B62<TAB>value` and nothing else. Single-cell edits read well. But **inserting a row shifts every row number below it**, so adding three rows to a 620-cell sheet produced a 314-line diff.

Dropping the row number from the key brings the same change down to 6 lines.

```diff
+History	A=2	C=1.1	E=added three jobs
+Items	A=group-09	B=JOB900	C=batch	D=new job 900	E=yes
+Items	A=group-09	B=JOB901	C=batch	D=new job 901	E=yes
+Items	A=group-09	B=JOB902	C=batch	D=new job 902	E=yes
```

docx behaves the same way. Keyed by paragraph ordinal, inserting three paragraphs into a 60-paragraph document gives a 93-line diff; without the ordinal it is 7 lines, and the version bump and the new section are readable as themselves.

A key derived from position changes for everything after a single insertion, which stops diff from pairing lines up. So the key has to come from the content, or not exist.

## Measurements

Generated fixtures, not real documents: a three-sheet workbook of 637 non-empty cells, and a 63-paragraph docx.

| | oyomi | alternative |
|---|---|---|
| One workbook, 3 sheets, 637 cells | 0.006s | soffice 1.54s warm, and two sheets missing |
| 20 workbooks | 0.107s | openpyxl 0.366s |
| Three rows inserted, diff size | 6 lines (`--row`) | 314 lines (cell keys) |
| Three paragraphs inserted, diff size | 7 lines (`--row`) | 93 lines (paragraph ordinals) |

## Tests

```console
$ cargo test
$ cargo llvm-cov --summary-only
```

Every fixture is generated in `tests/common/mod.rs` — the xlsx, docx and pptx
packages, a workbook whose second sheet is declared but absent, a part that is
not valid UTF-8, and a hand-built zip whose entry no reader will open. The
repository therefore holds no binary files of its own.

Line coverage is 99.6%. The one uncovered branch is the defensive path for a zip
central directory that disagrees with its own entries.

## Not handled yet

Documents made only of images come out empty. A line such as `<media: N files>` is needed, otherwise replacing a screenshot produces an empty diff.

Formulas are invisible. calamine returns cached results, so rewriting a formula that evaluates to the same value shows nothing.

Text inside xlsx shapes and cell comments is not collected.

For docx tracked changes, `w:del` is wrapped in `<del>`, but `w:ins` passes through unmarked.

Speaker notes are matched to slides by the number in the part name rather than through the package relationships, so a deck whose notes are numbered independently of its slides pairs them wrongly.

## License

MIT

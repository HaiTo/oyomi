mod common;

use common::*;
use oyomi::{render, Options};

fn out(path: &str, opts: &Options) -> String {
    let mut buf = Vec::new();
    render(path, opts, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

fn cells() -> Options {
    Options::default()
}

fn rows() -> Options {
    Options {
        row: true,
        ..Options::default()
    }
}

#[test]
fn cell_keys_carry_sheet_and_address() {
    let sheet = row(
        1,
        &[
            cell_str("A1", "Group"),
            cell_str("B1", "Id"),
            cell_num("C1", "42"),
        ],
    ) + &row(2, &[cell_str("A2", "group-01"), cell_bool("C2", true)]);
    let f = xlsx("addr.xlsx", &[("Items", sheet)]);

    assert_eq!(
        out(&f, &cells()),
        "Items!A1\tGroup\n\
         Items!B1\tId\n\
         Items!C1\t42\n\
         Items!A2\tgroup-01\n\
         Items!C2\ttrue\n"
    );
}

#[test]
fn addresses_follow_the_used_range_offset() {
    // Data starting away from A1 must still report its real address.
    let sheet = row(3, &[cell_str("C3", "left"), cell_str("AB3", "far right")]);
    let f = xlsx("offset.xlsx", &[("S", sheet)]);

    assert_eq!(out(&f, &cells()), "S!C3\tleft\nS!AB3\tfar right\n");
}

#[test]
fn row_keys_drop_the_row_number() {
    let sheet = row(1, &[cell_str("A1", "Id"), cell_str("C1", "Kind")]);
    let f = xlsx("rowkeys.xlsx", &[("Items", sheet)]);

    assert_eq!(out(&f, &rows()), "Items\tA=Id\tC=Kind\n");
}

#[test]
fn empty_rows_produce_no_line() {
    let sheet = row(1, &[cell_str("A1", "x")]) + &row(2, &[]) + &row(3, &[cell_str("A3", "y")]);
    let f = xlsx("gaps.xlsx", &[("S", sheet)]);

    assert_eq!(out(&f, &rows()), "S\tA=x\nS\tA=y\n");
    assert_eq!(out(&f, &cells()), "S!A1\tx\nS!A3\ty\n");
}

#[test]
fn tabs_and_newlines_stay_on_one_line() {
    // XML normalises CR to LF before we see it, so the carriage return of the
    // fixture arrives as a newline. `esc` is checked directly in units.rs.
    let sheet = row(1, &[cell_str("A1", "a\nb\tc\\d\re")]);
    let f = xlsx("escape.xlsx", &[("S", sheet)]);

    assert_eq!(out(&f, &cells()), "S!A1\ta\\nb\\tc\\\\d\\ne\n");
}

#[test]
fn dates_render_as_iso_like_timestamps() {
    let sheet = row(1, &[cell_date("A1", "44562")]);
    let f = xlsx("date.xlsx", &[("S", sheet)]);

    assert_eq!(out(&f, &cells()), "S!A1\t2022-01-01 00:00:00\n");
}

#[test]
fn cell_errors_are_visible() {
    let sheet = row(1, &[cell_err("A1", "#DIV/0!")]);
    let f = xlsx("err.xlsx", &[("S", sheet)]);

    assert_eq!(out(&f, &cells()), "S!A1\t#ERR(Div0)\n");
}

#[test]
fn list_reports_dimensions_and_counts() {
    let a = row(1, &[cell_str("A1", "x"), cell_str("B1", "y")]);
    let b = row(1, &[cell_str("A1", "z")]);
    let f = xlsx("list.xlsx", &[("First", a), ("Second", b)]);

    let opts = Options {
        list: true,
        ..Options::default()
    };
    assert_eq!(
        out(&f, &opts),
        "First\t1x2\t2 cells\nSecond\t1x1\t1 cells\n"
    );
}

#[test]
fn sheet_filter_selects_one_sheet() {
    let a = row(1, &[cell_str("A1", "first")]);
    let b = row(1, &[cell_str("A1", "second")]);
    let f = xlsx("filter.xlsx", &[("First", a), ("Second", b)]);

    let opts = Options {
        sheet: Some("Second".into()),
        ..Options::default()
    };
    assert_eq!(out(&f, &opts), "Second!A1\tsecond\n");

    let missing = Options {
        sheet: Some("Nope".into()),
        ..Options::default()
    };
    assert_eq!(out(&f, &missing), "");
}

#[test]
fn an_unreadable_sheet_does_not_hide_the_others() {
    let a = row(1, &[cell_str("A1", "kept")]);
    let f = xlsx_with(
        "broken.xlsx",
        &[("Good", a), ("Broken", String::new())],
        true,
    );

    let s = out(&f, &cells());
    assert!(s.contains("Good!A1\tkept\n"), "{s}");
    assert!(s.contains("!! sheet Broken:"), "{s}");
}

#[test]
fn inserting_rows_leaves_the_row_keyed_output_a_superset() {
    let before = row(1, &[cell_str("A1", "h")])
        + &row(2, &[cell_str("A2", "one")])
        + &row(3, &[cell_str("A3", "two")]);
    let after = row(1, &[cell_str("A1", "h")])
        + &row(2, &[cell_str("A2", "one")])
        + &row(3, &[cell_str("A3", "inserted")])
        + &row(4, &[cell_str("A4", "two")]);
    let v1 = xlsx("v1.xlsx", &[("S", before)]);
    let v2 = xlsx("v2.xlsx", &[("S", after)]);

    // With content keys an insertion only adds lines: the old rendering is a
    // subsequence of the new one, which is exactly what lets diff stay small.
    assert!(is_subsequence(&out(&v1, &rows()), &out(&v2, &rows())));
    // With address keys it is not, because every row below the insertion is
    // renumbered.
    assert!(!is_subsequence(&out(&v1, &cells()), &out(&v2, &cells())));
}

#[test]
fn the_extension_chooses_the_renderer() {
    let d = docx("dispatch.docx", &[para("word")]);
    assert_eq!(out(&d, &rows()), "p\t\tword\n");

    let p = pptx(
        "dispatch.pptx",
        &[("ppt/slides/slide1.xml", slide_xml(&["slide"]))],
    );
    assert_eq!(out(&p, &rows()), "slide001.body\tslide\n");

    // Template extensions take the same path as their document counterparts.
    let dt = docx("dispatch.dotx", &[para("template")]);
    assert_eq!(out(&dt, &rows()), "p\t\ttemplate\n");
    let pt = pptx(
        "dispatch.potx",
        &[("ppt/slides/slide1.xml", slide_xml(&["template"]))],
    );
    assert_eq!(out(&pt, &rows()), "slide001.body\ttemplate\n");
}

#[test]
fn unsupported_and_missing_files_report_why() {
    assert_eq!(
        render("x.txt", &cells(), &mut Vec::new()).unwrap_err(),
        "unsupported extension: txt"
    );
    assert!(render("x", &cells(), &mut Vec::new())
        .unwrap_err()
        .contains("unsupported extension"));
    assert!(render("no-such-file.xlsx", &cells(), &mut Vec::new()).is_err());
}

#[test]
fn a_writer_that_refuses_is_reported_not_ignored() {
    let f = xlsx("writefail.xlsx", &[("S", row(1, &[cell_str("A1", "x")]))]);
    assert!(render(&f, &cells(), &mut FailWriter).is_err());
    assert!(render(&f, &rows(), &mut FailWriter).is_err());
    let list = Options {
        list: true,
        ..Options::default()
    };
    assert!(render(&f, &list, &mut FailWriter).is_err());

    // The line that reports an unreadable sheet goes through the same writer.
    let broken = xlsx_with("writefail2.xlsx", &[("Broken", String::new())], true);
    assert!(render(&broken, &cells(), &mut FailWriter).is_err());
}

fn is_subsequence(a: &str, b: &str) -> bool {
    let mut it = b.lines();
    a.lines().all(|line| it.any(|c| c == line))
}

mod common;

use common::*;
use oyomi::{word, Keys};

fn out(path: &str, keys: Keys) -> String {
    let mut buf = Vec::new();
    word(path, keys, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

#[test]
fn paragraph_keys_number_the_paragraphs_and_keep_the_style() {
    let f = docx(
        "styles.docx",
        &[
            para_styled("Heading1", "Overview"),
            para("The system records the event."),
        ],
    );

    assert_eq!(
        out(&f, Keys::Address),
        "p0001\tHeading1\tOverview\n\
         p0002\t\tThe system records the event.\n"
    );
}

#[test]
fn table_cells_are_keyed_by_their_coordinates() {
    let f = docx(
        "table.docx",
        &[para("before"), table(&["left", "right"]), para("after")],
    );

    assert_eq!(
        out(&f, Keys::Address),
        "p0001\t\tbefore\n\
         t1.r001.c01\t\tleft\n\
         t1.r001.c02\t\tright\n\
         p0004\t\tafter\n"
    );
}

#[test]
fn content_keys_say_only_what_kind_of_line_it_is() {
    let f = docx(
        "flat.docx",
        &[para_styled("Heading1", "Overview"), table(&["cell"])],
    );

    assert_eq!(
        out(&f, Keys::Content),
        "p\tHeading1\tOverview\ntc\t\tcell\n"
    );
}

#[test]
fn deleted_text_is_kept_and_marked() {
    let f = docx("tracked.docx", &[para_deleted("removed sentence")]);

    assert_eq!(out(&f, Keys::Content), "p\t\t<del>removed sentence</del>\n");
}

#[test]
fn blank_paragraphs_produce_no_line() {
    let f = docx("blank.docx", &[para(""), para("   "), para("kept")]);

    assert_eq!(out(&f, Keys::Content), "p\t\tkept\n");
}

#[test]
fn inserting_paragraphs_leaves_the_content_keyed_output_a_superset() {
    let before = [para("one"), para("two"), para("three")];
    let after = [para("one"), para("inserted"), para("two"), para("three")];
    let v1 = docx("wv1.docx", &before);
    let v2 = docx("wv2.docx", &after);

    assert!(is_subsequence(
        &out(&v1, Keys::Content),
        &out(&v2, Keys::Content)
    ));
    assert!(!is_subsequence(
        &out(&v1, Keys::Address),
        &out(&v2, Keys::Address)
    ));
}

#[test]
fn a_package_without_a_document_part_reports_why() {
    let f = pack("empty.docx", &[("_rels/.rels", String::from("<x/>"))]);
    let e = word(&f, Keys::Address, &mut Vec::new()).unwrap_err();
    assert!(e.contains("word/document.xml"), "{e}");
}

#[test]
fn malformed_xml_reports_why() {
    let f = pack(
        "bad.docx",
        &[("word/document.xml", String::from("<w:body><w:p></w:body>"))],
    );
    assert!(word(&f, Keys::Address, &mut Vec::new()).is_err());
}

#[test]
fn a_file_that_is_not_a_zip_reports_why() {
    let f = write_tmp("notzip.docx", b"plain text, not a package");
    assert!(word(&f, Keys::Address, &mut Vec::new()).is_err());
}

#[test]
fn empty_elements_other_than_a_style_are_ignored() {
    let f = docx("empties.docx", &[para_with_empty_elements("line")]);
    assert_eq!(out(&f, Keys::Content), "p\tBody\tline\n");
}

#[test]
fn an_unknown_entity_reports_why() {
    let f = docx(
        "entity.docx",
        &[para_raw("<w:p><w:r><w:t>&nosuch;</w:t></w:r></w:p>")],
    );
    assert!(word(&f, Keys::Content, &mut Vec::new()).is_err());
}

#[test]
fn a_part_that_is_not_utf8_reports_why() {
    let f = pack_bytes(
        "notutf8.docx",
        &[("word/document.xml", vec![0xff, 0xfe, 0x00, 0x01])],
    );
    assert!(word(&f, Keys::Content, &mut Vec::new()).is_err());
}

#[test]
fn a_missing_file_reports_why() {
    assert!(word("no-such-file.docx", Keys::Content, &mut Vec::new()).is_err());
}

#[test]
fn a_writer_that_refuses_is_reported_not_ignored() {
    let f = docx("writefail.docx", &[para("line")]);
    assert!(word(&f, Keys::Content, &mut FailWriter).is_err());
}

fn is_subsequence(a: &str, b: &str) -> bool {
    let mut it = b.lines();
    a.lines().all(|line| it.any(|c| c == line))
}

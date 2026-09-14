mod common;

use common::*;
use oyomi::{media, render, Keys, Options};

/// The fingerprints asserted below are plain CRC32 of these bytes, the same
/// value `zlib.crc32` produces, taken from the zip central directory.
fn png(marker: u8) -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    v.extend_from_slice(&[marker; 24]);
    v
}

fn out(path: &str, keys: Keys) -> String {
    let mut buf = Vec::new();
    media(path, keys, false, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

#[test]
fn pictures_are_listed_with_a_fingerprint() {
    let f = docx_with_parts(
        "pics.docx",
        &[para("text")],
        &[
            ("word/media/image1.png", png(1)),
            ("word/media/image2.jpeg", png(2)),
        ],
    );

    assert_eq!(
        out(&f, Keys::Address),
        "media\tword/media/image1.png\t32\tcrc32:988e77e0\n\
         media\tword/media/image2.jpeg\t32\tcrc32:58ac27b4\n"
    );
    // Content keys keep the kind and drop the numbering, and order by the
    // fingerprint so that the list does not depend on part names either.
    assert_eq!(
        out(&f, Keys::Content),
        "media\t.jpeg\t32\tcrc32:58ac27b4\n\
         media\t.png\t32\tcrc32:988e77e0\n"
    );
}

#[test]
fn a_document_made_only_of_pictures_is_no_longer_empty() {
    let f = xlsx_extra(
        "shots.xlsx",
        &[("S", String::new())],
        false,
        &[("xl/media/image1.png", png(7))],
    );

    let mut buf = Vec::new();
    render(&f, &Options::default(), &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert_eq!(s, "media\txl/media/image1.png\t32\tcrc32:c3bbd109\n");
}

#[test]
fn replacing_a_picture_changes_its_line() {
    let before = docx_with_parts("swap1.docx", &[], &[("word/media/image1.png", png(1))]);
    let after = docx_with_parts("swap2.docx", &[], &[("word/media/image1.png", png(9))]);

    assert_ne!(out(&before, Keys::Content), out(&after, Keys::Content));
    assert_ne!(out(&before, Keys::Address), out(&after, Keys::Address));
}

#[test]
fn renumbering_a_picture_changes_nothing_under_content_keys() {
    // Saving a document can renumber its parts without any picture changing.
    let before = docx_with_parts("num1.docx", &[], &[("word/media/image1.png", png(3))]);
    let after = docx_with_parts("num2.docx", &[], &[("word/media/image7.png", png(3))]);

    assert_eq!(out(&before, Keys::Content), out(&after, Keys::Content));
    assert_ne!(out(&before, Keys::Address), out(&after, Keys::Address));
}

#[test]
fn embedded_objects_count_too() {
    let f = docx_with_parts(
        "ole.docx",
        &[],
        &[("word/embeddings/oleObject1.bin", png(4))],
    );
    assert!(out(&f, Keys::Address).contains("word/embeddings/oleObject1.bin"));
}

#[test]
fn a_part_without_an_extension_is_still_named() {
    let f = docx_with_parts("noext.docx", &[], &[("word/media/image1", png(5))]);
    assert!(out(&f, Keys::Content).starts_with("media\t(no extension)\t"));
}

#[test]
fn list_mode_gives_a_count_and_a_size() {
    let f = xlsx_extra(
        "listed.xlsx",
        &[("S", row(1, &[cell_str("A1", "x")]))],
        false,
        &[
            ("xl/media/image1.png", png(1)),
            ("xl/media/image2.png", png(2)),
        ],
    );
    let opts = Options {
        list: true,
        ..Options::default()
    };
    let mut buf = Vec::new();
    render(&f, &opts, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.ends_with("media\t2 files\t64 bytes\n"), "{s}");
}

#[test]
fn asking_for_one_sheet_leaves_the_package_out() {
    let f = xlsx_extra(
        "one.xlsx",
        &[("S", row(1, &[cell_str("A1", "x")]))],
        false,
        &[("xl/media/image1.png", png(1))],
    );
    let opts = Options {
        sheet: Some("S".into()),
        ..Options::default()
    };
    let mut buf = Vec::new();
    render(&f, &opts, &mut buf).unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "S!A1\tx\n");
}

#[test]
fn a_document_without_pictures_gains_no_line() {
    let f = docx("nopics.docx", &[para("text")]);
    assert_eq!(out(&f, Keys::Address), "");

    let mut buf = Vec::new();
    media(&f, Keys::Address, true, &mut buf).unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "");
}

#[test]
fn something_that_is_not_a_package_is_passed_over() {
    // The older binary spreadsheet formats reach this function too.
    let f = write_tmp("plain.xls", b"not a package");
    assert_eq!(out(&f, Keys::Address), "");
    assert_eq!(out("no-such-file.xlsx", Keys::Address), "");
}

#[test]
fn a_writer_that_refuses_is_reported_not_ignored() {
    let f = docx_with_parts("wf.docx", &[], &[("word/media/image1.png", png(1))]);
    assert!(media(&f, Keys::Address, false, &mut FailWriter).is_err());
    assert!(media(&f, Keys::Address, true, &mut FailWriter).is_err());
}

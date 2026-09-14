mod common;

use common::*;
use oyomi::{slides, Keys};

fn out(path: &str, keys: Keys) -> String {
    let mut buf = Vec::new();
    slides(path, keys, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

#[test]
fn slides_are_ordered_by_number_not_by_name() {
    // Written to the package in the wrong order on purpose: as text, "slide10"
    // sorts before "slide2".
    let f = pptx(
        "order.pptx",
        &[
            ("ppt/slides/slide10.xml", slide_xml(&["tenth"])),
            ("ppt/slides/slide2.xml", slide_xml(&["second"])),
            ("ppt/slides/slide1.xml", slide_xml(&["first"])),
        ],
    );

    assert_eq!(
        out(&f, Keys::Content),
        "slide001.body\tfirst\n\
         slide002.body\tsecond\n\
         slide010.body\ttenth\n"
    );
}

#[test]
fn paragraph_ordinals_appear_only_with_address_keys() {
    let f = pptx(
        "keys.pptx",
        &[("ppt/slides/slide1.xml", slide_xml(&["title", "body text"]))],
    );

    assert_eq!(
        out(&f, Keys::Address),
        "slide001.body.p001\ttitle\n\
         slide001.body.p002\tbody text\n"
    );
    assert_eq!(
        out(&f, Keys::Content),
        "slide001.body\ttitle\n\
         slide001.body\tbody text\n"
    );
}

#[test]
fn notes_are_labelled_and_follow_their_slide() {
    let f = pptx(
        "notes.pptx",
        &[
            ("ppt/notesSlides/notesSlide1.xml", slide_xml(&["say this"])),
            ("ppt/slides/slide1.xml", slide_xml(&["on screen"])),
        ],
    );

    assert_eq!(
        out(&f, Keys::Content),
        "slide001.body\ton screen\n\
         slide001.note\tsay this\n"
    );
}

#[test]
fn blank_paragraphs_and_unrelated_parts_are_skipped() {
    let f = pptx(
        "mixed.pptx",
        &[
            ("ppt/slideLayouts/slideLayout1.xml", slide_xml(&["layout"])),
            ("docProps/app.xml", String::from("<x/>")),
            ("ppt/slides/slide1.xml", slide_xml(&["", "  ", "kept"])),
        ],
    );

    assert_eq!(out(&f, Keys::Content), "slide001.body\tkept\n");
}

#[test]
fn a_package_with_no_slides_renders_nothing() {
    let f = pptx("bare.pptx", &[]);
    assert_eq!(out(&f, Keys::Content), "");
}

#[test]
fn a_part_named_without_digits_falls_back_to_zero() {
    let f = pptx(
        "nodigits.pptx",
        &[("ppt/slides/slide.xml", slide_xml(&["unnumbered"]))],
    );
    assert_eq!(out(&f, Keys::Content), "slide000.body\tunnumbered\n");
}

#[test]
fn malformed_slide_xml_reports_why() {
    let f = pptx(
        "badslide.pptx",
        &[("ppt/slides/slide1.xml", String::from("<a:p></p:sld>"))],
    );
    assert!(slides(&f, Keys::Content, &mut Vec::new()).is_err());
}

#[test]
fn a_missing_or_unreadable_file_reports_why() {
    assert!(slides("no-such-file.pptx", Keys::Content, &mut Vec::new()).is_err());
    let f = write_tmp("notzip.pptx", b"not a package");
    assert!(slides(&f, Keys::Content, &mut Vec::new()).is_err());
}

#[test]
fn an_unknown_entity_reports_why() {
    let f = pptx(
        "entity.pptx",
        &[(
            "ppt/slides/slide1.xml",
            String::from("<p:sld><a:p><a:t>&nosuch;</a:t></a:p></p:sld>"),
        )],
    );
    assert!(slides(&f, Keys::Content, &mut Vec::new()).is_err());
}

#[test]
fn a_writer_that_refuses_is_reported_not_ignored() {
    let f = pptx(
        "writefail.pptx",
        &[("ppt/slides/slide1.xml", slide_xml(&["line"]))],
    );
    assert!(slides(&f, Keys::Content, &mut FailWriter).is_err());
}

#[test]
fn a_slide_part_that_is_not_utf8_reports_why() {
    let f = pack_bytes(
        "notutf8.pptx",
        &[("ppt/slides/slide1.xml", vec![0xff, 0xfe, 0x00, 0x01])],
    );
    assert!(slides(&f, Keys::Content, &mut Vec::new()).is_err());
}

#[test]
fn an_entry_the_reader_cannot_open_reports_why() {
    let f = zip_with_unreadable_entry("aes.pptx", "ppt/slides/slide1.xml");
    let e = slides(&f, Keys::Content, &mut Vec::new()).unwrap_err();
    assert!(!e.is_empty(), "{e}");
}

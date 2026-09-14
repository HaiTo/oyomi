//! Fixture builders. Every test document is generated here, so the repository
//! carries no binary blobs and every expectation is readable next to the test.
//!
//! Each test binary pulls in the whole module and uses a subset of it.
#![allow(dead_code)]

use std::io::Write;
use std::path::PathBuf;
use zip::write::SimpleFileOptions;

/// Pack named parts into a zip container and write it under `target/`.
pub fn pack(name: &str, parts: &[(&str, String)]) -> String {
    let owned: Vec<(&str, Vec<u8>)> = parts
        .iter()
        .map(|(p, b)| (*p, b.as_bytes().to_vec()))
        .collect();
    pack_bytes(name, &owned)
}

/// Same, for parts that must not be valid UTF-8.
pub fn pack_bytes(name: &str, parts: &[(&str, Vec<u8>)]) -> String {
    let mut buf = Vec::new();
    {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default();
        for (part, body) in parts {
            z.start_file(*part, opts).unwrap();
            z.write_all(body).unwrap();
        }
        z.finish().unwrap();
    }
    write_tmp(name, &buf)
}

pub fn write_tmp(name: &str, bytes: &[u8]) -> String {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("target/test-fixtures");
    std::fs::create_dir_all(&dir).unwrap();
    dir.push(name);
    std::fs::write(&dir, bytes).unwrap();
    dir.to_string_lossy().into_owned()
}

pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ---------- xlsx ----------

pub fn cell_str(addr: &str, v: &str) -> String {
    format!(
        r#"<c r="{addr}" t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#,
        xml_escape(v)
    )
}

pub fn cell_num(addr: &str, v: &str) -> String {
    format!(r#"<c r="{addr}"><v>{v}</v></c>"#)
}

/// `s="1"` points at the date format declared in [`styles`].
pub fn cell_date(addr: &str, serial: &str) -> String {
    format!(r#"<c r="{addr}" s="1"><v>{serial}</v></c>"#)
}

pub fn cell_bool(addr: &str, v: bool) -> String {
    format!(r#"<c r="{addr}" t="b"><v>{}</v></c>"#, u8::from(v))
}

pub fn cell_err(addr: &str, v: &str) -> String {
    format!(r#"<c r="{addr}" t="e"><v>{}</v></c>"#, xml_escape(v))
}

pub fn row(n: u32, cells: &[String]) -> String {
    format!(r#"<row r="{n}">{}</row>"#, cells.concat())
}

fn sheet_xml(rows: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{rows}</sheetData></worksheet>"#
    )
}

fn styles() -> String {
    // cellXfs 0 is general, cellXfs 1 uses built-in date format 14 so that
    // calamine reports the cell as a date rather than a number.
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="1"><font/></fonts><fills count="1"><fill/></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0" xfId="0"/><xf numFmtId="14" xfId="0" applyNumberFormat="1"/></cellXfs></styleSheet>"#
        .to_string()
}

/// Build an xlsx from `(sheet name, rows xml)` pairs. When `drop_last_part` is
/// set the final sheet is declared in the workbook but its part is left out,
/// which is how a damaged workbook reaches the renderer.
pub fn xlsx_with(name: &str, sheets: &[(&str, String)], drop_last_part: bool) -> String {
    let mut types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#,
    );
    let mut wb_sheets = String::new();
    let mut wb_rels = String::from(
        r#"<Relationship Id="rIdS" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#,
    );
    let mut parts: Vec<(String, String)> = Vec::new();

    for (i, (sheet_name, rows)) in sheets.iter().enumerate() {
        let n = i + 1;
        let last = i + 1 == sheets.len();
        wb_sheets.push_str(&format!(
            r#"<sheet name="{}" sheetId="{n}" r:id="rId{n}"/>"#,
            xml_escape(sheet_name)
        ));
        wb_rels.push_str(&format!(
            r#"<Relationship Id="rId{n}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{n}.xml"/>"#
        ));
        if last && drop_last_part {
            continue;
        }
        types.push_str(&format!(
            r#"<Override PartName="/xl/worksheets/sheet{n}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#
        ));
        parts.push((format!("xl/worksheets/sheet{n}.xml"), sheet_xml(rows)));
    }
    types.push_str("</Types>");

    let workbook = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>{wb_sheets}</sheets></workbook>"#
    );
    let root_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdWB" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.to_string();
    let wb_rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{wb_rels}</Relationships>"#
    );

    let mut all: Vec<(&str, String)> = vec![
        ("[Content_Types].xml", types),
        ("_rels/.rels", root_rels),
        ("xl/workbook.xml", workbook),
        ("xl/_rels/workbook.xml.rels", wb_rels),
        ("xl/styles.xml", styles()),
    ];
    for (p, b) in &parts {
        all.push((p.as_str(), b.clone()));
    }
    pack(name, &all)
}

pub fn xlsx(name: &str, sheets: &[(&str, String)]) -> String {
    xlsx_with(name, sheets, false)
}

// ---------- docx ----------

pub fn para(text: &str) -> String {
    format!(
        r#"<w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
        xml_escape(text)
    )
}

pub fn para_styled(style: &str, text: &str) -> String {
    format!(
        r#"<w:p><w:pPr><w:pStyle w:val="{style}"/></w:pPr><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
        xml_escape(text)
    )
}

pub fn para_deleted(text: &str) -> String {
    format!(
        r#"<w:p><w:del><w:r><w:delText xml:space="preserve">{}</w:delText></w:r></w:del></w:p>"#,
        xml_escape(text)
    )
}

/// A one-row table; each entry becomes one cell holding one paragraph.
pub fn table(cells: &[&str]) -> String {
    let tcs: String = cells
        .iter()
        .map(|c| format!("<w:tc>{}</w:tc>", para(c)))
        .collect();
    format!("<w:tbl><w:tr>{tcs}</w:tr></w:tbl>")
}

/// A paragraph carrying empty elements that are not `w:pStyle`, and a style
/// element whose first attribute is not `w:val`, so that both "not this one"
/// branches are taken.
pub fn para_with_empty_elements(text: &str) -> String {
    format!(
        r#"<w:p><w:pPr><w:pStyle w:themeShade="1" w:val="Body"/><w:spacing w:after="0"/></w:pPr><w:r><w:br/><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
        xml_escape(text)
    )
}

/// Raw body XML, for fixtures that have to be invalid on purpose.
pub fn para_raw(xml: &str) -> String {
    xml.to_string()
}

/// A writer that refuses every write, to exercise the error paths that a real
/// file or pipe reaches only when the disk fills or the reader hangs up.
pub struct FailWriter;

impl Write for FailWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "no room",
        ))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "no room",
        ))
    }
}

pub fn docx(name: &str, body: &[String]) -> String {
    let doc = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{}</w:body></w:document>"#,
        body.concat()
    );
    pack(
        name,
        &[
            ("[Content_Types].xml", content_types_docx()),
            ("_rels/.rels", rels_to("word/document.xml")),
            ("word/document.xml", doc),
        ],
    )
}

fn content_types_docx() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#.to_string()
}

fn rels_to(target: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="{target}"/></Relationships>"#
    )
}

// ---------- pptx ----------

pub fn slide_xml(paragraphs: &[&str]) -> String {
    let ps: String = paragraphs
        .iter()
        .map(|t| format!(r#"<a:p><a:r><a:t>{}</a:t></a:r></a:p>"#, xml_escape(t)))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree>{ps}</p:spTree></p:cSld></p:sld>"#
    )
}

/// `parts` are `(zip entry name, slide xml)`, written in the given order so a
/// test can put slide10 before slide2 on purpose.
pub fn pptx(name: &str, parts: &[(&str, String)]) -> String {
    let mut all: Vec<(&str, String)> = vec![
        ("[Content_Types].xml", content_types_docx()),
        ("_rels/.rels", rels_to("ppt/presentation.xml")),
    ];
    for (p, b) in parts {
        all.push((p, b.clone()));
    }
    pack(name, &all)
}

/// A zip holding one entry that the reader refuses to open at all: the
/// encrypted flag is set and the compression method is the AES one. Built by
/// hand because no writer here produces such an archive.
pub fn zip_with_unreadable_entry(file_name: &str, entry: &str) -> String {
    fn le16(v: &mut Vec<u8>, n: u16) {
        v.extend_from_slice(&n.to_le_bytes());
    }
    fn le32(v: &mut Vec<u8>, n: u32) {
        v.extend_from_slice(&n.to_le_bytes());
    }
    const AES: u16 = 99;
    const ENCRYPTED: u16 = 1;
    let name = entry.as_bytes();
    let mut z = Vec::new();

    le32(&mut z, 0x0403_4b50); // local file header
    le16(&mut z, 20); // version needed
    le16(&mut z, ENCRYPTED);
    le16(&mut z, AES);
    le16(&mut z, 0); // time
    le16(&mut z, 0); // date
    le32(&mut z, 0); // crc32
    le32(&mut z, 0); // compressed size
    le32(&mut z, 0); // uncompressed size
    le16(&mut z, name.len() as u16);
    le16(&mut z, 0); // extra length
    z.extend_from_slice(name);

    let cd_offset = z.len() as u32;
    le32(&mut z, 0x0201_4b50); // central directory header
    le16(&mut z, 20); // version made by
    le16(&mut z, 20); // version needed
    le16(&mut z, ENCRYPTED);
    le16(&mut z, AES);
    le16(&mut z, 0); // time
    le16(&mut z, 0); // date
    le32(&mut z, 0); // crc32
    le32(&mut z, 0); // compressed size
    le32(&mut z, 0); // uncompressed size
    le16(&mut z, name.len() as u16);
    le16(&mut z, 0); // extra length
    le16(&mut z, 0); // comment length
    le16(&mut z, 0); // disk number
    le16(&mut z, 0); // internal attributes
    le32(&mut z, 0); // external attributes
    le32(&mut z, 0); // offset of local header
    z.extend_from_slice(name);
    let cd_size = z.len() as u32 - cd_offset;

    le32(&mut z, 0x0605_4b50); // end of central directory
    le16(&mut z, 0); // this disk
    le16(&mut z, 0); // disk with central directory
    le16(&mut z, 1); // entries on this disk
    le16(&mut z, 1); // entries total
    le32(&mut z, cd_size);
    le32(&mut z, cd_offset);
    le16(&mut z, 0); // comment length

    write_tmp(file_name, &z)
}

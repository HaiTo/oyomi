//! oyomi — render OOXML documents as deterministic text.
//!
//! The only goal is to give every line a key that survives editing, so that
//! `git diff` can pair lines up. Faithful reproduction of layout is not a goal.

use std::io::{Read, Write};

// Re-exported so that callers can build and inspect cell values without
// depending on calamine directly.
pub use calamine::{CellErrorType, Data, ExcelDateTime, ExcelDateTimeType};

pub const USAGE: &str = "usage: oyomi [--sheet NAME] [--list] [--row] <file.xlsx|.docx|.pptx>";

/// What a line is keyed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keys {
    /// Position: cell address, paragraph ordinal, table coordinates.
    Address,
    /// Content only. A row inserted above no longer rewrites every key below it.
    Content,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub sheet: Option<String>,
    pub list: bool,
    pub row: bool,
}

impl Options {
    pub fn keys(&self) -> Keys {
        if self.row {
            Keys::Content
        } else {
            Keys::Address
        }
    }
}

/// Parse argv (without the program name). `Err` carries the message to print.
pub fn parse_args(args: &[String]) -> Result<(String, Options), String> {
    let mut path: Option<String> = None;
    let mut opts = Options::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--sheet" => {
                i += 1;
                match args.get(i) {
                    Some(v) => opts.sheet = Some(v.clone()),
                    None => return Err("--sheet needs a sheet name".into()),
                }
            }
            "--list" => opts.list = true,
            "--row" => opts.row = true,
            s if s.starts_with("--") => return Err(format!("unknown option: {s}")),
            s => path = Some(s.to_string()),
        }
        i += 1;
    }
    match path {
        Some(p) => Ok((p, opts)),
        None => Err(USAGE.to_string()),
    }
}

/// Run one invocation and return the process exit code.
pub fn run(args: &[String], out: &mut impl Write, err: &mut impl Write) -> i32 {
    let (path, opts) = match parse_args(args) {
        Ok(v) => v,
        Err(msg) => {
            let _ = writeln!(err, "{msg}");
            return 2;
        }
    };
    if let Err(e) = render(&path, &opts, out) {
        // A textconv driver that exits non-zero makes git drop the diff entirely,
        // so a damaged file still leaves through the front door.
        let _ = writeln!(out, "!! oyomi: {e}");
    }
    let _ = out.flush();
    0
}

/// Dispatch on the file extension.
pub fn render(path: &str, opts: &Options, out: &mut impl Write) -> Result<(), String> {
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "xlsx" | "xlsm" | "xltx" | "xltm" | "xls" | "xlsb" | "ods" => spreadsheet(path, opts, out),
        "docx" | "dotx" => word(path, opts.keys(), out),
        "pptx" | "potx" => slides(path, opts.keys(), out),
        other => Err(format!("unsupported extension: {other}")),
    }
}

// ---------- spreadsheet ----------

pub fn spreadsheet(path: &str, opts: &Options, out: &mut impl Write) -> Result<(), String> {
    use calamine::{Data, Reader};
    let mut wb = calamine::open_workbook_auto(path).map_err(|e| e.to_string())?;
    let names: Vec<String> = wb.sheet_names().to_vec();
    for name in names {
        if let Some(only) = &opts.sheet {
            if only != &name {
                continue;
            }
        }
        let range = match wb.worksheet_range(&name) {
            Ok(r) => r,
            Err(e) => {
                // One unreadable sheet must not cost the reader the other sheets.
                writeln!(out, "!! sheet {name}: {e}").map_err(|e| e.to_string())?;
                continue;
            }
        };
        let (r0, c0) = range.start().unwrap_or((0, 0));
        if opts.list {
            let n = range
                .cells()
                .filter(|(_, _, v)| !matches!(v, Data::Empty))
                .count();
            let (rh, rw) = (range.height(), range.width());
            writeln!(out, "{name}\t{rh}x{rw}\t{n} cells").map_err(|e| e.to_string())?;
            continue;
        }
        for (ri, row) in range.rows().enumerate() {
            if opts.keys() == Keys::Content {
                let cells: Vec<String> = row
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| !matches!(c, Data::Empty))
                    .map(|(ci, c)| format!("{}={}", col_letters(c0 as usize + ci), esc(&value(c))))
                    .collect();
                if !cells.is_empty() {
                    writeln!(out, "{name}\t{}", cells.join("\t")).map_err(|e| e.to_string())?;
                }
                continue;
            }
            for (ci, cell) in row.iter().enumerate() {
                if matches!(cell, Data::Empty) {
                    continue;
                }
                let addr = format!("{}{}", col_letters(c0 as usize + ci), r0 as usize + ri + 1);
                writeln!(out, "{name}!{addr}\t{}", esc(&value(cell))).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

/// Render one cell. Every branch has to be stable across runs and platforms.
pub fn value(cell: &calamine::Data) -> String {
    use calamine::Data::*;
    use calamine::DataType;
    match cell {
        Int(v) => v.to_string(),
        Float(v) => v.to_string(),
        String(v) => v.clone(),
        Bool(v) => v.to_string(),
        DateTime(_) => cell
            .as_datetime()
            .map(|d| d.to_string())
            .unwrap_or_else(|| "<datetime>".into()),
        DateTimeIso(v) | DurationIso(v) => v.clone(),
        Error(e) => format!("#ERR({e:?})"),
        Empty => std::string::String::new(),
    }
}

/// 0 -> A, 25 -> Z, 26 -> AA. Spreadsheet columns are bijective base 26.
pub fn col_letters(mut n: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

// ---------- docx / pptx ----------

fn open_part(path: &str, name: &str) -> Result<String, String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    // Name the part: "!! oyomi: specified file not found in archive" on its own
    // says nothing about which part of the package was missing.
    let mut e = z.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    let mut s = String::new();
    e.read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(s)
}

pub fn word(path: &str, keys: Keys, out: &mut impl Write) -> Result<(), String> {
    use quick_xml::events::Event;
    let xml = open_part(path, "word/document.xml")?;
    let mut r = quick_xml::Reader::from_str(&xml);
    let mut buf = Vec::new();
    let mut text = String::new();
    let mut style = String::new();
    let mut para = 0usize;
    let (mut tbl, mut row, mut col) = (0usize, 0usize, 0usize);
    let mut in_del = false;
    loop {
        match r.read_event_into(&mut buf) {
            Err(e) => return Err(e.to_string()),
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"w:tbl" => {
                    tbl += 1;
                    row = 0;
                }
                b"w:tr" => {
                    row += 1;
                    col = 0;
                }
                b"w:tc" => col += 1,
                b"w:del" => in_del = true,
                b"w:p" => {
                    para += 1;
                    text.clear();
                    style.clear();
                }
                _ => {}
            },
            Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:pStyle" {
                    for a in e.attributes().flatten() {
                        if a.key.as_ref() == b"w:val" {
                            style = String::from_utf8_lossy(&a.value).into_owned();
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let t = e.unescape().map_err(|e| e.to_string())?;
                if in_del {
                    // Deleted text is kept, marked, so a revision reads as a revision.
                    text.push_str("<del>");
                    text.push_str(&t);
                    text.push_str("</del>");
                } else {
                    text.push_str(&t);
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"w:del" => in_del = false,
                b"w:tbl" => row = 0,
                b"w:p" => {
                    if !text.trim().is_empty() {
                        let key = match (keys, tbl > 0 && row > 0) {
                            (Keys::Content, true) => "tc".to_string(),
                            (Keys::Content, false) => "p".to_string(),
                            (Keys::Address, true) => format!("t{tbl}.r{row:03}.c{col:02}"),
                            (Keys::Address, false) => format!("p{para:04}"),
                        };
                        writeln!(out, "{key}\t{style}\t{}", esc(text.trim()))
                            .map_err(|e| e.to_string())?;
                    }
                    text.clear();
                }
                _ => {}
            },
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

pub fn slides(path: &str, keys: Keys, out: &mut impl Write) -> Result<(), String> {
    use quick_xml::events::Event;
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let mut parts: Vec<(u32, bool, String)> = Vec::new();
    for i in 0..z.len() {
        // Defensive: the index came from the archive itself, so this only fails
        // on a central directory that disagrees with its own entries.
        let mut e = z.by_index(i).map_err(|e| e.to_string())?;
        let name = e.name().to_string();
        let note = name.starts_with("ppt/notesSlides/notesSlide");
        let slide = name.starts_with("ppt/slides/slide");
        if !((slide || note) && name.ends_with(".xml")) {
            continue;
        }
        let mut xml = String::new();
        e.read_to_string(&mut xml)
            .map_err(|err| format!("{name}: {err}"))?;
        parts.push((trailing_number(&name), note, xml));
    }
    // Zip entry order is whatever the writer chose, and "slide10" sorts before
    // "slide2" as text, so order by the number.
    parts.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    for (n, note, xml) in parts {
        let mut r = quick_xml::Reader::from_str(&xml);
        let mut buf = Vec::new();
        let mut text = String::new();
        let mut p = 0usize;
        loop {
            match r.read_event_into(&mut buf) {
                Err(e) => return Err(e.to_string()),
                Ok(Event::Eof) => break,
                Ok(Event::Start(e)) if e.name().as_ref() == b"a:p" => {
                    p += 1;
                    text.clear();
                }
                Ok(Event::Text(e)) => text.push_str(&e.unescape().map_err(|e| e.to_string())?),
                Ok(Event::End(e)) if e.name().as_ref() == b"a:p" => {
                    if !text.trim().is_empty() {
                        let kind = if note { "note" } else { "body" };
                        let key = match keys {
                            Keys::Content => format!("slide{n:03}.{kind}"),
                            Keys::Address => format!("slide{n:03}.{kind}.p{p:03}"),
                        };
                        writeln!(out, "{key}\t{}", esc(text.trim())).map_err(|e| e.to_string())?;
                    }
                    text.clear();
                }
                _ => {}
            }
            buf.clear();
        }
    }
    Ok(())
}

/// The digits at the end of a part name: "ppt/slides/slide12.xml" -> 12.
fn trailing_number(name: &str) -> u32 {
    name.trim_end_matches(".xml")
        .rsplit(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|d| d.parse().ok())
        .unwrap_or(0)
}

/// The least escaping that keeps one cell on one line. Line-wise readability
/// of the diff outranks round-tripping the original bytes.
pub fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\r', "")
        .replace('\n', "\\n")
}

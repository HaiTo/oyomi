use oyomi::{
    col_letters, esc, parse_args, value, CellErrorType, Data, ExcelDateTime, ExcelDateTimeType,
    Keys, Options, USAGE,
};

#[test]
fn columns_are_bijective_base_26() {
    for (n, want) in [
        (0, "A"),
        (25, "Z"),
        (26, "AA"),
        (27, "AB"),
        (51, "AZ"),
        (52, "BA"),
        (701, "ZZ"),
        (702, "AAA"),
        (16383, "XFD"), // the last column Excel offers
    ] {
        assert_eq!(col_letters(n), want, "column {n}");
    }
}

#[test]
fn escaping_keeps_a_value_on_one_line() {
    assert_eq!(esc("plain"), "plain");
    assert_eq!(esc("a\tb"), "a\\tb");
    assert_eq!(esc("a\nb"), "a\\nb");
    assert_eq!(esc("a\r\nb"), "a\\nb");
    assert_eq!(esc("a\rb"), "ab");
    // The backslash goes first, so an escape introduced here is not re-escaped.
    assert_eq!(esc("a\\nb"), "a\\\\nb");
    assert_eq!(esc(""), "");
}

#[test]
fn every_cell_kind_renders() {
    assert_eq!(value(&Data::Int(-7)), "-7");
    assert_eq!(value(&Data::Float(1.5)), "1.5");
    assert_eq!(value(&Data::Float(3.0)), "3");
    assert_eq!(value(&Data::String("text".into())), "text");
    assert_eq!(value(&Data::Bool(true)), "true");
    assert_eq!(value(&Data::Bool(false)), "false");
    assert_eq!(
        value(&Data::DateTimeIso("2026-01-02T03:04:05".into())),
        "2026-01-02T03:04:05"
    );
    assert_eq!(value(&Data::DurationIso("PT1H".into())), "PT1H");
    assert_eq!(value(&Data::Error(CellErrorType::Div0)), "#ERR(Div0)");
    assert_eq!(value(&Data::Error(CellErrorType::Ref)), "#ERR(Ref)");
    assert_eq!(value(&Data::Empty), "");
}

#[test]
fn serial_dates_and_durations_render() {
    let d = Data::DateTime(ExcelDateTime::new(
        44562.5,
        ExcelDateTimeType::DateTime,
        false,
    ));
    assert_eq!(value(&d), "2022-01-01 12:00:00");

    let delta = Data::DateTime(ExcelDateTime::new(1.5, ExcelDateTimeType::TimeDelta, false));
    assert_eq!(value(&delta), "1900-01-01 12:00:00");

    // A serial too large for a calendar date leaves the marker rather than
    // dropping the cell, so the line still shows that something is there.
    let far = Data::DateTime(ExcelDateTime::new(1e12, ExcelDateTimeType::DateTime, false));
    assert_eq!(value(&far), "<datetime>");
}

#[test]
fn options_choose_the_key_scheme() {
    assert_eq!(Options::default().keys(), Keys::Address);
    assert_eq!(
        Options {
            row: true,
            ..Options::default()
        }
        .keys(),
        Keys::Content
    );
}

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn arguments_parse_in_any_order() {
    let (path, opts) = parse_args(&args(&["--row", "a.xlsx", "--sheet", "S", "--list"])).unwrap();
    assert_eq!(path, "a.xlsx");
    assert_eq!(opts.sheet.as_deref(), Some("S"));
    assert!(opts.list);
    assert_eq!(opts.keys(), Keys::Content);

    let (path, opts) = parse_args(&args(&["b.docx"])).unwrap();
    assert_eq!(path, "b.docx");
    assert!(opts.sheet.is_none());
    assert!(!opts.list);
    assert_eq!(opts.keys(), Keys::Address);

    // The last path wins rather than becoming a silent second input.
    let (path, _) = parse_args(&args(&["first.xlsx", "second.xlsx"])).unwrap();
    assert_eq!(path, "second.xlsx");
}

#[test]
fn bad_arguments_say_what_is_wrong() {
    assert_eq!(parse_args(&args(&[])).unwrap_err(), USAGE);
    assert_eq!(
        parse_args(&args(&["--nope", "a.xlsx"])).unwrap_err(),
        "unknown option: --nope"
    );
    assert_eq!(
        parse_args(&args(&["a.xlsx", "--sheet"])).unwrap_err(),
        "--sheet needs a sheet name"
    );
}

#[test]
fn keys_are_comparable_and_printable() {
    assert_ne!(Keys::Address, Keys::Content);
    assert_eq!(format!("{:?}", Keys::Content), "Content");
    assert_eq!(
        format!("{:?}", Options::default().clone().keys()),
        "Address"
    );
}

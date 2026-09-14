mod common;

use common::*;

fn run(args: &[&str]) -> (i32, String, String) {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = oyomi::run(&argv, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn a_readable_file_is_rendered_and_exits_zero() {
    let f = xlsx("cli.xlsx", &[("S", row(1, &[cell_str("A1", "value")]))]);
    let (code, out, err) = run(&["--row", &f]);

    assert_eq!(code, 0);
    assert_eq!(out, "S\tA=value\n");
    assert_eq!(err, "");
}

#[test]
fn a_damaged_file_still_exits_zero() {
    // git drops the whole diff if a textconv driver fails, so the error has to
    // travel as content on stdout instead of as an exit code.
    let f = write_tmp("damaged.xlsx", b"not a package at all");
    let (code, out, err) = run(&[&f]);

    assert_eq!(code, 0);
    assert!(out.starts_with("!! oyomi: "), "{out}");
    assert_eq!(err, "");
}

#[test]
fn an_unsupported_extension_still_exits_zero() {
    let (code, out, _) = run(&["notes.txt"]);
    assert_eq!(code, 0);
    assert_eq!(out, "!! oyomi: unsupported extension: txt\n");
}

#[test]
fn bad_usage_exits_two_and_writes_to_stderr() {
    let (code, out, err) = run(&[]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, format!("{}\n", oyomi::USAGE));

    let (code, _, err) = run(&["--nope"]);
    assert_eq!(code, 2);
    assert_eq!(err, "unknown option: --nope\n");
}

#[test]
fn the_installed_binary_behaves_the_same() {
    let f = xlsx("bin.xlsx", &[("S", row(1, &[cell_str("A1", "value")]))]);
    let exe = env!("CARGO_BIN_EXE_oyomi");

    let ok = std::process::Command::new(exe)
        .args(["--row", &f])
        .output()
        .unwrap();
    assert!(ok.status.success());
    assert_eq!(String::from_utf8_lossy(&ok.stdout), "S\tA=value\n");

    let bad = std::process::Command::new(exe)
        .arg("--nope")
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&bad.stderr),
        "unknown option: --nope\n"
    );
}

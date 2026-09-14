use std::io::{BufWriter, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut err = std::io::stderr();
    let code = oyomi::run(&args, &mut out, &mut err);
    let _ = out.flush();
    std::process::exit(code);
}

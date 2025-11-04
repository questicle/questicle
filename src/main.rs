use questicle::{Host, Interpreter, Parser};
use std::{fs, path::PathBuf};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut repl = false;
    let mut file: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-r" | "--repl" => repl = true,
            path => {
                file = Some(PathBuf::from(path));
            }
        }
    }

    let host = Host::default();
    let mut interp = Interpreter::with_host(host);

    if let Some(ref path) = file {
        let src = fs::read_to_string(&path).expect("failed to read file");
        run_source(&src, &mut interp);
    }

    if repl || file.is_none() {
        run_repl(&mut interp);
    }
}

fn run_source(src: &str, interp: &mut Interpreter) {
    match Parser::new(src).parse_program() {
        Ok(program) => {
            if let Err(e) = interp.eval(program) {
                eprintln!("Runtime error: {}", e);
                std::process::exit(70);
            }
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(65);
        }
    }
}

fn run_repl(interp: &mut Interpreter) {
    use rustyline::{error::ReadlineError, DefaultEditor};

    let mut rl = DefaultEditor::new().expect("failed to init REPL");
    println!("Questicle REPL. Ctrl-D to exit.");
    loop {
        match rl.readline(">> ") {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                rl.add_history_entry(line.as_str()).ok();
                match Parser::new(&line).parse_program() {
                    Ok(program) => match interp.eval(program) {
                        Ok(val) => {
                            if let Some(v) = val {
                                println!("{v}");
                            }
                        }
                        Err(e) => eprintln!("Runtime error: {e}"),
                    },
                    Err(e) => eprintln!("Parse error: {e}"),
                }
            }
            Err(ReadlineError::Interrupted) => { /* Ctrl-C: new line */ }
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("REPL error: {e}");
                break;
            }
        }
    }
}

fn print_help() {
    println!("Questicle - game scripting language\n");
    println!("Usage: qk [options] [file.qk]\n");
    println!("Options:\n  -r, --repl   Start an interactive REPL\n  -h, --help   Show this help");
}

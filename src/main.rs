// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Questicle
use questicle::{Host, Interpreter, Parser};
use std::io::{self, Read};
use std::{fs, path::PathBuf};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut repl = false;
    let mut file: Option<PathBuf> = None;
    // fmt options
    let mut fmt_mode = false;
    let mut fmt_check = false;
    let mut fmt_write = true;
    let mut fmt_stdin = false;
    let mut fmt_paths: Vec<PathBuf> = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-r" | "--repl" => repl = true,
            "fmt" => {
                fmt_mode = true;
            }
            "--check" => {
                fmt_check = true;
                fmt_write = false;
            }
            "--write" => {
                fmt_write = true;
            }
            "--stdin" => {
                fmt_stdin = true;
            }
            path => {
                if fmt_mode {
                    fmt_paths.push(PathBuf::from(path));
                } else {
                    file = Some(PathBuf::from(path));
                }
            }
        }
    }

    if fmt_mode {
        let code = run_fmt(fmt_stdin, &fmt_paths, fmt_check, fmt_write).unwrap_or(2);
        std::process::exit(code);
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
    println!("Options:\n  -r, --repl   Start an interactive REPL\n  -h, --help   Show this help\n\nSubcommands:\n  fmt [--check|--write] [--stdin] [paths...]  Format files");
}

fn run_fmt(stdin_mode: bool, paths: &[PathBuf], check: bool, write: bool) -> io::Result<i32> {
    use std::fs;
    use walkdir::WalkDir;
    // Helper to process one file content
    fn process(path: Option<&PathBuf>, content: &str, check: bool, write: bool) -> io::Result<i32> {
        let formatted = questicle::formatter::format_source(content);
        if formatted != content {
            if check {
                if let Some(p) = path {
                    eprintln!("Reformat needed: {}", p.display());
                }
                return Ok(1);
            }
            if write {
                if let Some(p) = path {
                    fs::write(p, formatted)?;
                } else {
                    print!("{}", formatted);
                }
            } else {
                print!("{}", formatted);
            }
        } else {
            if !write {
                print!("{}", formatted);
            }
        }
        Ok(0)
    }

    if stdin_mode {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        return process(None, &buf, check, write);
    }

    let mut code = 0;
    let mut targets: Vec<PathBuf> = Vec::new();
    if paths.is_empty() {
        // default: **/*.qk under cwd
        for entry in WalkDir::new(".").into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file()
                && entry.path().extension().map(|e| e == "qk").unwrap_or(false)
            {
                targets.push(entry.path().to_path_buf());
            }
        }
    } else {
        for p in paths {
            let md = std::fs::metadata(p);
            if let Ok(md) = md {
                if md.is_dir() {
                    for entry in WalkDir::new(p).into_iter().filter_map(Result::ok) {
                        if entry.file_type().is_file()
                            && entry.path().extension().map(|e| e == "qk").unwrap_or(false)
                        {
                            targets.push(entry.path().to_path_buf());
                        }
                    }
                } else if md.is_file() {
                    targets.push(p.clone());
                }
            }
        }
    }

    for t in targets {
        let content = std::fs::read_to_string(&t)?;
        let rc = process(Some(&t), &content, check, write)?;
        if rc == 1 {
            code = 1;
        }
    }
    Ok(code)
}

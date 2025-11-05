use questicle::{Host, Interpreter, Parser};
use std::fs;
use std::path::Path;

fn run_file(path: &Path) -> Result<(), String> {
    let src = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let program = Parser::new(&src)
        .parse_program()
        .map_err(|e| e.to_string())?;
    let host = Host::default();
    let mut interp = Interpreter::with_host(host);
    interp.eval(program).map(|_| ()).map_err(|e| e.to_string())
}

#[test]
fn run_all_examples() {
    let dir = Path::new("examples");
    for entry in fs::read_dir(dir).expect("examples dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().map(|e| e == "qk").unwrap_or(false) {
            run_file(&path).unwrap_or_else(|e| panic!("{} -> {}", path.display(), e));
        }
    }
}

use questicle::{Host, Interpreter, Parser};

#[test]
fn hello_runs() {
    let src = r#"
        print("hi");
        let x: number = 1 + 2 * 3;
        x;
    "#;
    let program = Parser::new(src).parse_program().expect("parse");
    let mut interp = Interpreter::with_host(Host::default());
    let v = interp.eval(program).expect("run");
    assert!(v.is_some());
}

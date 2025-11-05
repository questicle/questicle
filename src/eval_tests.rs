#[cfg(test)]
mod tests {
    use crate::{Host, Interpreter, Parser};

    fn run(src: &str) -> Option<crate::value::Value> {
        let p = Parser::new(src).parse_program().expect("parse");
        let mut i = Interpreter::with_host(Host::default());
        i.eval(p).expect("run")
    }

    #[test]
    fn implicit_return_last_expression() {
        let v = run("{ 1 + 2; 3 } ");
        assert!(format!("{}", v.unwrap()).contains("3"));
    }

    #[test]
    fn break_and_continue() {
        let src = r#"
        let i = 0;
        while (true) { i = i + 1; break; }
        i;
        "#;
        let v = run(src).unwrap();
        assert!(format!("{}", v).contains("1"));
    }

    #[test]
    fn closure_captures() {
        let src = r#"
        let x = 41;
        let f = fn() { { x + 1 } };
        f();
        "#;
        let v = run(src).unwrap();
        assert!(format!("{}", v).contains("42"));
    }
}

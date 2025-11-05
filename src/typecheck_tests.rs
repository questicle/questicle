#[cfg(test)]
mod tests {
    use crate::{parser::Parser, typecheck};

    fn tc(src: &str) -> typecheck::TypeCheckResult {
        let p = Parser::new(src).parse_program().expect("parse");
        typecheck::check_program(&p)
    }

    #[test]
    fn string_concat_and_any() {
        let r = tc("let a = \"x\" + 1;");
        assert!(r.errors.is_empty());
    }

    #[test]
    fn number_ops_enforced() {
        let r = tc("let a = \"x\" - 1;");
        assert!(!r.errors.is_empty());
        assert!(r.errors[0].message.contains("Number operands required"));
    }

    #[test]
    fn comparisons_and_logical_any_ok() {
        let r = tc("let b = 1 < \"x\"; let c = true && 1;");
        assert!(r.errors.is_empty());
    }

    #[test]
    fn record_inference_and_field() {
        let r = tc("let o = { a: 1, b: \"s\" }; let y = o.b;");
        assert!(r.errors.is_empty());
    }

    #[test]
    fn return_type_mismatch_hint() {
        let r = tc("fn f(): number { return \"x\"; }");
        assert!(!r.errors.is_empty());
        assert!(r.errors[0]
            .hint
            .as_ref()
            .map(|h| h.contains("return type"))
            .unwrap_or(false));
    }
}

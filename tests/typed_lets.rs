use questicle::Parser;

#[test]
fn let_requires_type_annotation() {
    let src = "let x = 1;"; // missing : type
    let err = Parser::new(src).parse_program().unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("expected : type annotation"));
}

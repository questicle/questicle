use std::fs;

use questicle::formatter::format_source;

#[test]
fn examples_are_idempotent() {
    // Pick a representative set of example files that reflect preferred style
    let paths = [
        "examples/basics.qk",
        "examples/typed.qk",
        "examples/inventory.qk",
        "examples/functions_and_closures.qk",
    ];
    for p in paths {
        let src = fs::read_to_string(p).expect(p);
        let out = format_source(&src);
        assert_eq!(out, src, "{} should be already formatted", p);
        assert_eq!(out, format_source(&out), "{} formatting is idempotent", p);
    }
}

#[test]
fn formats_bad_spacing_and_commas() {
    // bad input intentionally crammed and oddly spaced
    let src = r#"let a:number=1+2*3;let o={x:1,y:2};fn f(x:number,y:number){return{x:x,y:y}}"#;
    let out = format_source(src);
    // Basic expectations: spacing around operators and after commas; braces/newlines preserved
    assert!(out.contains("let a: number = 1 + 2 * 3;"), "out=\n{}", out);
    assert!(out.contains("let o = { x: 1, y: 2 };"), "out=\n{}", out);
    assert!(out.contains("fn f(x: number, y: number)"), "out=\n{}", out);
    assert!(out.contains("return { x: x, y: y };"), "out=\n{}", out);
}

#[test]
fn preserves_comments_and_collapses_blanks() {
    let src = r#"// head


let x:number=1; /* mid */ let y:number=2;


// tail
"#;
    let out = format_source(src);
    // comments present
    assert!(out.contains("// head"));
    assert!(out.contains("// tail"));
    // no excessive blank lines (more than 2)
    assert!(!out.contains("\n\n\n\n"));
    // spacing corrected around declarations and inline block comment kept
    assert!(
        out.contains("let x: number = 1; /* mid */ let y: number = 2;"),
        "{}",
        out
    );
}

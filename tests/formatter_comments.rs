use questicle::formatter::format_source;

#[test]
fn preserves_eol_and_block_comments() {
    let src = r#"let a=1; // eol
/* block */ let b = 2; /* mid */
let c = /* inside */ 3;
"#;
    let out = format_source(src);
    assert!(out.contains("// eol"));
    assert!(out.contains("/* block */"));
    assert!(out.contains("/* mid */"));
    assert!(out.contains("/* inside */"));
    assert_eq!(out, format_source(&out));
}

#[test]
fn collapses_blank_lines_but_not_comments() {
    let src = "// a\n\n\n\n// b\n\nlet x=1;\n\n\n";
    let out = format_source(src);
    // Max 2 blank lines in a row
    assert!(!out.contains("\n\n\n\n"));
}

#[test]
fn objects_and_commas_spacing() {
    let src = r#"let o={a:1,b:2,c:3};
let p = { a: 1, /*k*/ b: 2, c :3};
"#;
    let out = format_source(src);
    // Our formatter expands to multiline object when combined with block comments
    assert!(
        out.contains("let p = {\n  a: 1, /*k*/ b: 2, c: 3\n};"),
        "formatted: {}",
        out
    );
}

#[test]
fn function_and_blocks() {
    let src = r#"fn f(a,b){return a+b;} let z=3;"#;
    let out = format_source(src);
    // Our token-based formatter preserves 'fn f(a, b)' without inserting type annotations
    assert!(out.contains("fn f(a, b)"));
    assert!(out.contains("return a + b;"));
}

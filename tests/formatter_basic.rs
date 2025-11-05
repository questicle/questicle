use questicle::formatter::format_source;

#[test]
fn formats_and_preserves_comments() {
    let src = r#"// Header comment
let x=1; // inline

/* block
multi */
fn start(q){ { id:q.id, state:"started" } }
"#;
    let out = format_source(src);
    // idempotence
    assert_eq!(out, format_source(&out));
    // Must contain comments intact
    assert!(out.contains("// Header comment"));
    assert!(out.contains("// inline"));
    assert!(out.contains("/* block"));
    assert!(out.contains("multi */"));
    // Spacing around operators and colons
    assert!(out.contains("let x = 1;"));
    assert!(out.contains("id: q.id"));
}

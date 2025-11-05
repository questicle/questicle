#[cfg(test)]
mod tests {
    use crate::parser::{ParseError, Parser};

    #[test]
    fn parse_statements_and_blocks() {
        let src = r#"
        let x = 1;
        if (x) { x = x + 1; } else { x = x - 1; }
        while (x) { break; }
        for (i in [1,2]) { continue; }
        { let y = 2; }
        "#;
        let program = Parser::new(src).parse_program().expect("parse");
        assert!(program.statements.len() >= 4);
    }

    #[test]
    fn parse_map_vs_block_disambiguation() {
        let src = r#"
        let f = fn(q){ { id: q.id, state: "started" } };
        "#;
        let program = Parser::new(src).parse_program().expect("parse");
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn parse_error_reports_position() {
        let src = "let ="; // missing identifier
        let err = Parser::new(src).parse_program().unwrap_err();
        match err {
            ParseError::Expected { line, col, .. } | ParseError::Unexpected { line, col } => {
                assert!(line >= 1 && col >= 1)
            }
            _ => panic!("unexpected error kind"),
        }
    }
}

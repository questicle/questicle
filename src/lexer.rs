use crate::token::{Token, TokenKind};
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\r\f]+")]
enum LexToken {
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("!")]
    Bang,
    #[token("?")]
    Question,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Ne,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("->")]
    Arrow,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("=")]
    Assign,

    #[regex(r#"\"([^\\\n\"]|\\.)*\""#)]
    String,
    #[regex(r"[0-9]+(\.[0-9]+)?", priority = 2)]
    Number,
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", priority = 1)]
    Ident,
    #[regex(r"\n+")]
    Newline,
    #[regex(r"//[^\n]*")]
    LineComment,
    #[regex(r"/\*([^*]|\*+[^*/])*\*+/")]
    BlockComment,
}

pub struct Lexer<'a> {
    src: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src }
    }

    pub fn lex(&self) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut line = 1usize;
        let mut col = 1usize;
        let mut lex = LexToken::lexer(self.src);
        let mut last_end = 0usize;
        while let Some(tok) = lex.next() {
            let span = lex.span();
            // Update line/col by counting newlines in skipped ranges
            let skipped = &self.src[last_end..span.start];
            for ch in skipped.chars() {
                if ch == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
            }
            let t = match tok {
                Ok(LexToken::LParen) => TokenKind::LeftParen,
                Ok(LexToken::RParen) => TokenKind::RightParen,
                Ok(LexToken::LBrace) => TokenKind::LeftBrace,
                Ok(LexToken::RBrace) => TokenKind::RightBrace,
                Ok(LexToken::LBracket) => TokenKind::LeftBracket,
                Ok(LexToken::RBracket) => TokenKind::RightBracket,
                Ok(LexToken::Comma) => TokenKind::Comma,
                Ok(LexToken::Dot) => TokenKind::Dot,
                Ok(LexToken::Semicolon) => TokenKind::Semicolon,
                Ok(LexToken::Colon) => TokenKind::Colon,
                Ok(LexToken::Plus) => TokenKind::Plus,
                Ok(LexToken::Minus) => TokenKind::Minus,
                Ok(LexToken::Star) => TokenKind::Star,
                Ok(LexToken::Slash) => TokenKind::Slash,
                Ok(LexToken::Percent) => TokenKind::Percent,
                Ok(LexToken::Bang) => TokenKind::Bang,
                Ok(LexToken::Question) => TokenKind::Question,
                Ok(LexToken::Le) => TokenKind::LessEqual,
                Ok(LexToken::Ge) => TokenKind::GreaterEqual,
                Ok(LexToken::EqEq) => TokenKind::EqualEqual,
                Ok(LexToken::Ne) => TokenKind::BangEqual,
                Ok(LexToken::AndAnd) => TokenKind::AndAnd,
                Ok(LexToken::OrOr) => TokenKind::OrOr,
                Ok(LexToken::Arrow) => TokenKind::Arrow,
                Ok(LexToken::Lt) => TokenKind::Less,
                Ok(LexToken::Gt) => TokenKind::Greater,
                Ok(LexToken::Assign) => TokenKind::Assign,
                Ok(LexToken::String) => {
                    let s = &self.src[span.start..span.end];
                    let unquoted = &s[1..s.len() - 1];
                    let val = unescape(unquoted);
                    TokenKind::String(val)
                }
                Ok(LexToken::Number) => {
                    let s = &self.src[span.start..span.end];
                    TokenKind::Number(s.parse().unwrap())
                }
                Ok(LexToken::Ident) => {
                    let s = &self.src[span.start..span.end];
                    match s {
                        "let" => TokenKind::Let,
                        "fn" => TokenKind::Fn,
                        "if" => TokenKind::If,
                        "else" => TokenKind::Else,
                        "while" => TokenKind::While,
                        "for" => TokenKind::For,
                        "in" => TokenKind::In,
                        "return" => TokenKind::Return,
                        "true" => TokenKind::True,
                        "false" => TokenKind::False,
                        "null" => TokenKind::Null,
                        "break" => TokenKind::Break,
                        "continue" => TokenKind::Continue,
                        _ => TokenKind::Identifier(s.to_string()),
                    }
                }
                Ok(LexToken::Newline) => {
                    line += 1;
                    col = 1;
                    last_end = span.end;
                    continue;
                }
                Ok(LexToken::LineComment) => {
                    last_end = span.end;
                    continue;
                }
                Ok(LexToken::BlockComment) => {
                    let comm = &self.src[span.start..span.end];
                    for ch in comm.chars() {
                        if ch == '\n' {
                            line += 1;
                            col = 1;
                        } else {
                            col += 1;
                        }
                    }
                    last_end = span.end;
                    continue;
                }
                Err(_) => {
                    last_end = span.end;
                    continue;
                }
            };
            tokens.push(Token::new(t, line, col));
            last_end = span.end;
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind;

    #[test]
    fn lex_basic_and_skip_comments() {
        let src = "let x = 1 + 2; // comment\n/* block */ let y=3;";
        let toks = Lexer::new(src).lex();
        // We should not see comment tokens; check key sequence presence
        let kinds: Vec<_> = toks
            .iter()
            .map(|t| std::mem::discriminant(&t.kind))
            .collect();
        // Expect at least these tokens in order: Let, Identifier, Assign, Number, Plus, Number, Semicolon, Let, Identifier, Assign, Number, Semicolon
        let seq = vec![
            std::mem::discriminant(&TokenKind::Let),
            std::mem::discriminant(&TokenKind::Identifier(String::new())),
            std::mem::discriminant(&TokenKind::Assign),
            std::mem::discriminant(&TokenKind::Number(0.0)),
            std::mem::discriminant(&TokenKind::Plus),
            std::mem::discriminant(&TokenKind::Number(0.0)),
            std::mem::discriminant(&TokenKind::Semicolon),
            std::mem::discriminant(&TokenKind::Let),
            std::mem::discriminant(&TokenKind::Identifier(String::new())),
            std::mem::discriminant(&TokenKind::Assign),
            std::mem::discriminant(&TokenKind::Number(0.0)),
            std::mem::discriminant(&TokenKind::Semicolon),
        ];
        // Find subsequence
        let mut j = 0usize;
        for k in kinds {
            if k == seq[j] {
                j += 1;
                if j == seq.len() {
                    break;
                }
            }
        }
        assert_eq!(
            j,
            seq.len(),
            "lexer did not produce expected token sequence"
        );
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('r') => out.push('\r'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

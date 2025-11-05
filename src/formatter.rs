// SPDX-License-Identifier: MIT
// Comment-preserving formatter for Questicle

#[derive(Clone, Debug, PartialEq)]
enum TKind {
    // structural
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Semicolon,
    Colon,
    Op(String),
    Assign,
    // literals/idents
    Ident(String),
    Number(String),
    Str(String),
    Keyword(String),
    // comments and layout
    LineComment(String),  // includes leading // but not trailing newline
    BlockComment(String), // includes /* ... */ as-is
    Newline,
}

#[derive(Clone, Debug)]
struct Tok {
    kind: TKind,
}

#[derive(Clone, Debug)]
pub struct FormatterOptions {
    pub indent_size: usize,
    pub max_blank_lines: usize,
}

impl Default for FormatterOptions {
    fn default() -> Self {
        Self {
            indent_size: 2,
            max_blank_lines: 2,
        }
    }
}

pub fn format_source(src: &str) -> String {
    format_source_with_options(src, &FormatterOptions::default())
}

pub fn format_source_with_options(src: &str, opts: &FormatterOptions) -> String {
    let toks = tokenize(src);
    render(toks, opts)
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn tokenize(src: &str) -> Vec<Tok> {
    let mut v = Vec::new();
    let mut i = 0usize;
    let bytes = src.as_bytes();
    let len = bytes.len();
    let s = src;
    // No block comment state required; comments are emitted directly
    while i < len {
        let c = s[i..].chars().next().unwrap();
        // Newline
        if c == '\n' {
            v.push(Tok {
                kind: TKind::Newline,
            });
            i += 1;
            continue;
        }
        // Whitespace (spaces/tabs) -> skip, will be normalized
        if c == ' ' || c == '\t' || c == '\r' || c == '\u{000C}' {
            // form feed
            i += 1;
            continue;
        }
        // Comments
        if s[i..].starts_with("//") {
            let start = i;
            i += 2;
            while i < len {
                let ch = s[i..].chars().next().unwrap();
                if ch == '\n' {
                    break;
                }
                i += ch.len_utf8();
            }
            let text = &s[start..i];
            v.push(Tok {
                kind: TKind::LineComment(text.to_string()),
            });
            continue;
        }
        if s[i..].starts_with("/*") {
            let start = i;
            i += 2;
            while i + 1 < len {
                if s[i..].starts_with("*/") {
                    i += 2;
                    break;
                }
                let ch = s[i..].chars().next().unwrap();
                i += ch.len_utf8();
            }
            let text = &s[start..i.min(len)];
            v.push(Tok {
                kind: TKind::BlockComment(text.to_string()),
            });
            continue;
        }
        // Strings
        if c == '"' {
            let mut j = i + 1;
            let mut escaped = false;
            while j < len {
                let ch = s[j..].chars().next().unwrap();
                j += ch.len_utf8();
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }
                if ch == '"' {
                    break;
                }
            }
            let text = &s[i..j];
            v.push(Tok {
                kind: TKind::Str(text.to_string()),
            });
            i = j;
            continue;
        }
        // Two-char operators
        let two = if i + 1 < len { &s[i..i + 2] } else { "" };
        match two {
            "<=" | ">=" | "==" | "!=" | "&&" | "||" | "->" => {
                v.push(Tok {
                    kind: TKind::Op(two.to_string()),
                });
                i += 2;
                continue;
            }
            _ => {}
        }
        // Single char punctuation
        match c {
            '(' => {
                v.push(Tok {
                    kind: TKind::LParen,
                });
                i += 1;
                continue;
            }
            ')' => {
                v.push(Tok {
                    kind: TKind::RParen,
                });
                i += 1;
                continue;
            }
            '{' => {
                v.push(Tok {
                    kind: TKind::LBrace,
                });
                i += 1;
                continue;
            }
            '}' => {
                v.push(Tok {
                    kind: TKind::RBrace,
                });
                i += 1;
                continue;
            }
            '[' => {
                v.push(Tok {
                    kind: TKind::LBracket,
                });
                i += 1;
                continue;
            }
            ']' => {
                v.push(Tok {
                    kind: TKind::RBracket,
                });
                i += 1;
                continue;
            }
            ',' => {
                v.push(Tok { kind: TKind::Comma });
                i += 1;
                continue;
            }
            '.' => {
                v.push(Tok { kind: TKind::Dot });
                i += 1;
                continue;
            }
            ';' => {
                v.push(Tok {
                    kind: TKind::Semicolon,
                });
                i += 1;
                continue;
            }
            ':' => {
                v.push(Tok { kind: TKind::Colon });
                i += 1;
                continue;
            }
            '+' | '-' | '*' | '/' | '%' | '!' | '?' | '<' | '>' | '=' => {
                if c == '=' {
                    v.push(Tok {
                        kind: TKind::Assign,
                    });
                } else {
                    v.push(Tok {
                        kind: TKind::Op(c.to_string()),
                    });
                }
                i += 1;
                continue;
            }
            _ => {}
        }
        // Identifier or keyword
        if is_ident_start(c) {
            let mut j = i + c.len_utf8();
            while j < len {
                let ch = s[j..].chars().next().unwrap();
                if is_ident_continue(ch) {
                    j += ch.len_utf8();
                } else {
                    break;
                }
            }
            let ident = &s[i..j];
            let kind = match ident {
                "let" | "fn" | "if" | "else" | "while" | "for" | "in" | "return" | "true"
                | "false" | "null" | "break" | "continue" => TKind::Keyword(ident.to_string()),
                _ => TKind::Ident(ident.to_string()),
            };
            v.push(Tok { kind });
            i = j;
            continue;
        }
        // Number (simple)
        if c.is_ascii_digit() {
            let mut j = i + 1;
            while j < len {
                let ch = s[j..].chars().next().unwrap();
                if ch.is_ascii_digit() || ch == '.' {
                    j += ch.len_utf8();
                } else {
                    break;
                }
            }
            v.push(Tok {
                kind: TKind::Number(s[i..j].to_string()),
            });
            i = j;
            continue;
        }
        // Fallback: treat as identifier char
        v.push(Tok {
            kind: TKind::Ident(c.to_string()),
        });
        i += c.len_utf8();
    }
    v
}

fn render(toks: Vec<Tok>, opts: &FormatterOptions) -> String {
    let mut out = String::new();
    let mut indent: i32 = 0;
    let mut need_indent = true;
    let mut blanks = 0usize;
    let mut prev_was_token = false; // for spacing
    let mut prev_kind: Option<TKind> = None;

    // spacing rules handled inline

    let mut it = toks.into_iter().peekable();
    while let Some(tok) = it.next() {
        let kind_for_match = tok.kind.clone();
        let kind_for_prev = tok.kind.clone();
        match kind_for_match {
            TKind::Newline => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                need_indent = true;
                prev_was_token = false;
                blanks += 1;
            }
            TKind::LineComment(text) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                }
                if prev_was_token && !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push_str(&text);
                out.push('\n');
                need_indent = true;
                prev_was_token = false;
                blanks = 1; // comment ends a line
            }
            TKind::BlockComment(text) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                }
                let has_newline = text.contains('\n');
                // Preserve block as-is
                if has_newline {
                    let lines: Vec<&str> = text.split('\n').collect();
                    for (i, l) in lines.iter().enumerate() {
                        if i == 0 {
                            out.push_str(l);
                        } else {
                            out.push('\n');
                            write_indent(&mut out, indent, opts.indent_size);
                            out.push_str(l);
                        }
                    }
                } else {
                    out.push_str(&text);
                }
                // If the comment was multi-line, we likely ended with a newline already
                if has_newline {
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                    need_indent = true;
                    prev_was_token = false;
                    blanks = 1;
                } else {
                    // inline block comment stays inline; ensure spacing before next identifier-like token
                    need_indent = false;
                    prev_was_token = true;
                    blanks = 0;
                    if let Some(next) = it.peek() {
                        let needs_space = matches!(
                            next.kind,
                            TKind::Ident(_) | TKind::Number(_) | TKind::Str(_) | TKind::Keyword(_)
                        );
                        if needs_space && !out.ends_with(' ') && !out.ends_with('\n') {
                            out.push(' ');
                        }
                    }
                }
            }
            TKind::RBrace => {
                // Close block: dedent first
                if need_indent { /* ok */
                } else {
                    /* ensure newline before a closing brace unless already at line start */
                    out.push('\n');
                }
                indent = (indent - 1).max(0);
                write_indent(&mut out, indent, opts.indent_size);
                out.push('}');
                need_indent = false;
                prev_was_token = true;
                blanks = 0;
                // If next is semicolon, don't add newline now; else add newline
                match it.peek().map(|t| &t.kind) {
                    Some(TKind::Semicolon) => {}
                    _ => {
                        out.push('\n');
                        need_indent = true;
                        prev_was_token = false;
                        blanks = 1;
                    }
                }
            }
            TKind::LBrace => {
                // space before '{' if needed
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                }
                if !out.ends_with(' ')
                    && !out.ends_with('\n')
                    && !out.ends_with('{')
                    && !out.ends_with('(')
                {
                    out.push(' ');
                }
                out.push('{');
                out.push('\n');
                indent += 1;
                need_indent = true;
                prev_was_token = false;
                blanks = 1;
            }
            TKind::Semicolon => {
                out.push(';');
                out.push('\n');
                need_indent = true;
                prev_was_token = false;
                blanks = 1;
            }
            TKind::Comma => {
                out.push(',');
                out.push(' ');
                prev_was_token = true;
                blanks = 0;
                need_indent = false;
            }
            TKind::Colon => {
                out.push(':');
                out.push(' ');
                prev_was_token = true;
                blanks = 0;
                need_indent = false;
            }
            TKind::LParen => {
                // decide spacing before '('
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                }
                if let Some(TKind::Keyword(_)) = prev_kind {
                    out.push(' ');
                }
                out.push('(');
                prev_was_token = true;
                blanks = 0;
            }
            TKind::RParen => {
                out.push(')');
                prev_was_token = true;
                blanks = 0;
                need_indent = false;
            }
            TKind::LBracket => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                out.push('[');
                prev_was_token = true;
                blanks = 0;
            }
            TKind::RBracket => {
                out.push(']');
                prev_was_token = true;
                blanks = 0;
                need_indent = false;
            }
            TKind::Dot => {
                out.push('.');
                prev_was_token = true;
                blanks = 0;
                need_indent = false;
            }
            TKind::Assign | TKind::Op(_) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                if !out.ends_with(' ') && !out.ends_with('\n') {
                    out.push(' ');
                }
                match &tok.kind {
                    TKind::Assign => out.push('='),
                    TKind::Op(op) => out.push_str(op),
                    _ => {}
                }
                out.push(' ');
                prev_was_token = true;
                blanks = 0;
            }
            TKind::Ident(ref s) | TKind::Number(ref s) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                // Add space if previous token requires it
                if let Some(
                    TKind::Ident(_)
                    | TKind::Number(_)
                    | TKind::Str(_)
                    | TKind::RParen
                    | TKind::RBracket
                    | TKind::Keyword(_),
                ) = &prev_kind
                {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
                out.push_str(s);
                prev_was_token = true;
                blanks = 0;
            }
            TKind::Str(ref s) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                if let Some(
                    TKind::Ident(_)
                    | TKind::Number(_)
                    | TKind::Str(_)
                    | TKind::RParen
                    | TKind::RBracket,
                ) = &prev_kind
                {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
                out.push_str(s);
                prev_was_token = true;
                blanks = 0;
            }
            TKind::Keyword(ref k) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                if let Some(
                    TKind::Ident(_)
                    | TKind::Number(_)
                    | TKind::Str(_)
                    | TKind::RParen
                    | TKind::RBracket,
                ) = &prev_kind
                {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
                out.push_str(k);
                prev_was_token = true;
                blanks = 0;
            }
        }

        // collapse blank lines beyond max
        if need_indent && blanks > opts.max_blank_lines {
            // too many blanks
            // remove extra blank just added
            while blanks > opts.max_blank_lines && out.ends_with('\n') {
                out.pop();
                blanks -= 1;
            }
            out.push('\n');
            blanks = opts.max_blank_lines;
        }

        prev_kind = Some(match kind_for_prev {
            TKind::LParen => TKind::LParen,
            TKind::RParen => TKind::RParen,
            TKind::LBrace => TKind::LBrace,
            TKind::RBrace => TKind::RBrace,
            TKind::LBracket => TKind::LBracket,
            TKind::RBracket => TKind::RBracket,
            TKind::Comma => TKind::Comma,
            TKind::Dot => TKind::Dot,
            TKind::Semicolon => TKind::Semicolon,
            TKind::Colon => TKind::Colon,
            TKind::Assign => TKind::Assign,
            TKind::Op(op) => TKind::Op(op),
            TKind::Ident(s) => TKind::Ident(s),
            TKind::Number(s) => TKind::Number(s),
            TKind::Str(s) => TKind::Str(s),
            TKind::Keyword(k) => TKind::Keyword(k),
            TKind::LineComment(s) => TKind::LineComment(s),
            TKind::BlockComment(s) => TKind::BlockComment(s),
            TKind::Newline => TKind::Newline,
        });
    }

    // Trim trailing newlines to a single newline
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

fn write_indent(out: &mut String, indent: i32, indent_size: usize) {
    for _ in 0..(indent.max(0) as usize * indent_size) {
        out.push(' ');
    }
}

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
    while i < len {
        let c = s[i..].chars().next().unwrap();
        if c == '\n' {
            v.push(Tok {
                kind: TKind::Newline,
            });
            i += 1;
            continue;
        }
        if c == ' ' || c == '\t' || c == '\r' || c == '\u{000C}' {
            i += 1;
            continue;
        }
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
    // Track inline map/object braces vs statement blocks
    let mut brace_inline_stack: Vec<bool> = Vec::new();
    // Track bracket depth to tweak comma spacing in lists
    let mut bracket_depth: i32 = 0;
    // Track generic depth for list/map type arguments to avoid spaces around '<' and '>'
    let mut generic_depth: i32 = 0;
    // Track when we are inside a function signature so the following '{' is treated as a block
    let mut in_fn_signature: bool = false;

    // spacing rules handled inline

    let mut it = toks.into_iter().peekable();
    while let Some(tok) = it.next() {
        let kind_for_match = tok.kind.clone();
        let kind_for_prev = tok.kind.clone();
        match kind_for_match {
            TKind::Newline => {
                // Always emit a newline for explicit newline tokens; trim trailing spaces first
                while out.ends_with(' ') {
                    out.pop();
                }
                out.push('\n');
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
                // Do not inject a newline here; the following Newline token will handle it
                need_indent = false;
                prev_was_token = false;
                blanks = 0;
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
                    // Only insert a newline if one does not immediately follow in the token stream
                    let newline_follows =
                        matches!(it.peek().map(|t| &t.kind), Some(TKind::Newline));
                    if !newline_follows {
                        if !out.ends_with('\n') {
                            out.push('\n');
                        }
                        need_indent = true;
                        prev_was_token = false;
                        blanks = 1;
                    } else {
                        // Defer to the next Newline token; reset blank counter so it isn't collapsed
                        need_indent = true;
                        prev_was_token = false;
                        blanks = 0;
                    }
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
                // Close block or inline map/object
                let inline = brace_inline_stack.pop().unwrap_or(false);
                if inline {
                    if !out.ends_with(' ') && !out.ends_with('{') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                    out.push('}');
                    need_indent = false;
                    prev_was_token = true;
                    blanks = 0;
                } else {
                    // Close block: dedent first
                    if need_indent { /* ok */
                    } else {
                        // If the previous token was an inline object close, ensure a semicolon before we break the line
                        if matches!(prev_kind, Some(TKind::RBrace)) && !out.ends_with(';') {
                            out.push(';');
                        }
                        out.push('\n');
                    }
                    indent = (indent - 1).max(0);
                    write_indent(&mut out, indent, opts.indent_size);
                    out.push('}');
                    need_indent = false;
                    prev_was_token = true;
                    blanks = 0;
                    // If next is semicolon or a Newline token, don't add newline now; else add newline
                    match it.peek().map(|t| &t.kind) {
                        Some(TKind::Semicolon) | Some(TKind::Newline) => {}
                        _ => {
                            out.push('\n');
                            need_indent = true;
                            prev_was_token = false;
                            blanks = 1;
                        }
                    }
                }
            }
            TKind::LBrace => {
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
                let is_block_context = match &prev_kind {
                    Some(TKind::RParen) => true,
                    Some(TKind::Keyword(k)) => {
                        matches!(k.as_str(), "if" | "else" | "while" | "for" | "fn")
                    }
                    _ => false,
                } || in_fn_signature;
                if is_block_context {
                    out.push('{');
                    if !matches!(it.peek().map(|t| &t.kind), Some(TKind::Newline)) {
                        out.push('\n');
                    }
                    indent += 1;
                    need_indent = true;
                    prev_was_token = false;
                    blanks = 1;
                    brace_inline_stack.push(false);
                    // we've consumed the function signature
                    in_fn_signature = false;
                } else {
                    let mut look = it.clone();
                    let mut depth = 1i32;
                    let mut has_inline_block_comment_inside = false;
                    while let Some(n) = look.next() {
                        match n.kind {
                            TKind::LBrace => depth += 1,
                            TKind::RBrace => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            TKind::BlockComment(ref s) if !s.contains('\n') => {
                                has_inline_block_comment_inside = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                    // If immediate next token is a newline, treat as a multi-line object/map literal block
                    let newline_immediately_follows =
                        matches!(it.peek().map(|t| &t.kind), Some(TKind::Newline));
                    if has_inline_block_comment_inside || newline_immediately_follows {
                        out.push('{');
                        if !newline_immediately_follows {
                            out.push('\n');
                        }
                        indent += 1;
                        // If a Newline token follows immediately, let it insert the newline;
                        // otherwise we already inserted one above.
                        need_indent = !newline_immediately_follows;
                        prev_was_token = true;
                        // We'll let the upcoming Newline token (if any) manage blanks.
                        if !newline_immediately_follows {
                            blanks = 1;
                        }
                        brace_inline_stack.push(false);
                    } else {
                        out.push('{');
                        out.push(' ');
                        need_indent = false;
                        prev_was_token = true;
                        blanks = 0;
                        brace_inline_stack.push(true);
                    }
                }
            }
            TKind::Semicolon => {
                out.push(';');
                // If a single-line block or line comment follows, keep it inline instead of breaking line
                let inline_block_comment_follows = it
                    .peek()
                    .and_then(|t| match &t.kind {
                        TKind::BlockComment(s) => Some(!s.contains('\n')),
                        _ => None,
                    })
                    .unwrap_or(false);
                let inline_line_comment_follows =
                    matches!(it.peek().map(|t| &t.kind), Some(TKind::LineComment(_)));
                let else_follows = matches!(it.peek().map(|t| &t.kind), Some(TKind::Keyword(ref s)) if s == "else");
                let newline_follows = matches!(it.peek().map(|t| &t.kind), Some(TKind::Newline));
                if inline_block_comment_follows || inline_line_comment_follows || else_follows {
                    out.push(' ');
                    need_indent = false;
                    prev_was_token = true; // allow the comment arm to add itself inline
                    blanks = 0;
                } else if !newline_follows {
                    out.push('\n');
                    need_indent = true;
                    prev_was_token = false;
                    blanks = 1;
                }
            }
            TKind::Comma => {
                out.push(',');
                // Put a space after a comma except inside generic angle brackets
                if generic_depth <= 0 {
                    out.push(' ');
                }
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
                if let Some(TKind::Keyword(ref k)) = prev_kind {
                    if k == "if" || k == "while" || k == "for" {
                        out.push(' ');
                    }
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
                // ensure a space after 'in' before a list literal
                if let Some(TKind::Keyword(ref k)) = prev_kind {
                    if k == "in" && !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
                out.push('[');
                bracket_depth += 1;
                prev_was_token = true;
                blanks = 0;
            }
            TKind::RBracket => {
                out.push(']');
                bracket_depth = (bracket_depth - 1).max(0);
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
            TKind::Assign => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                if let Some(
                    TKind::Ident(_)
                    | TKind::Number(_)
                    | TKind::Str(_)
                    | TKind::RParen
                    | TKind::RBracket
                    | TKind::RBrace
                    | TKind::Op(_),
                ) = &prev_kind
                {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
                out.push('=');
                out.push(' ');
                prev_was_token = true;
                blanks = 0;
            }
            TKind::Op(op) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                let is_generic_open = op == "<"
                    && matches!(&prev_kind, Some(TKind::Ident(ref s)) if s == "list" || s == "map");
                let is_generic_close = op == ">" && generic_depth > 0;
                if !(is_generic_open || is_generic_close) {
                    if let Some(
                        TKind::Ident(_)
                        | TKind::Number(_)
                        | TKind::Str(_)
                        | TKind::RParen
                        | TKind::RBracket
                        | TKind::RBrace,
                    ) = &prev_kind
                    {
                        if !out.ends_with(' ') && !out.ends_with('\n') {
                            out.push(' ');
                        }
                    }
                }
                if is_generic_open {
                    generic_depth += 1;
                    out.push('<');
                } else if is_generic_close {
                    generic_depth -= 1;
                    out.push('>');
                } else {
                    out.push_str(&op);
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
                prev_was_token = true;
                blanks = 0;
            }
            TKind::Ident(s) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                if let Some(
                    TKind::Ident(_)
                    | TKind::Number(_)
                    | TKind::Str(_)
                    | TKind::RParen
                    | TKind::RBracket
                    | TKind::RBrace,
                ) = &prev_kind
                {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
                out.push_str(&s);
                prev_was_token = true;
                blanks = 0;
            }
            TKind::Number(s) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                if let Some(
                    TKind::Ident(_)
                    | TKind::Number(_)
                    | TKind::Str(_)
                    | TKind::RParen
                    | TKind::RBracket
                    | TKind::RBrace,
                ) = &prev_kind
                {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
                out.push_str(&s);
                prev_was_token = true;
                blanks = 0;
            }
            TKind::Str(s) => {
                if need_indent {
                    write_indent(&mut out, indent, opts.indent_size);
                    need_indent = false;
                }
                if let Some(
                    TKind::Ident(_)
                    | TKind::Number(_)
                    | TKind::Str(_)
                    | TKind::RParen
                    | TKind::RBracket
                    | TKind::RBrace,
                ) = &prev_kind
                {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
                out.push_str(&s);
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
                if k == "fn" {
                    in_fn_signature = true;
                }
                // Add a trailing space after keywords when followed by an identifier-like token or '{'.
                // Do NOT add here if next is '(', since LParen arm will add the space for constructs like `if (`.
                match it.peek().map(|t| &t.kind) {
                    Some(TKind::LParen) => { /* defer spacing to LParen */ }
                    Some(TKind::Ident(_))
                    | Some(TKind::Number(_))
                    | Some(TKind::Str(_))
                    | Some(TKind::Keyword(_))
                    | Some(TKind::LBrace) => {
                        if !out.ends_with(' ') && !out.ends_with('\n') {
                            out.push(' ');
                        }
                    }
                    _ => {}
                }
                prev_was_token = true;
                blanks = 0;
            }
        }

        // collapse blank lines beyond max
        if need_indent && blanks > opts.max_blank_lines {
            // too many blanks; trim down to the configured maximum
            while blanks > opts.max_blank_lines && out.ends_with('\n') {
                out.pop();
                blanks -= 1;
            }
            // Do not add another newline here; we already have at most max blank lines
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

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use questicle::{typecheck, Parser};

struct Backend {
    client: Client,
    docs: Arc<RwLock<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        let caps = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(
                    vec![".", ",", "(", " ", "\n"]
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                ),
                ..Default::default()
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            document_formatting_provider: Some(OneOf::Left(true)),
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".into(), ",".into()]),
                retrigger_characters: None,
                work_done_progress_options: Default::default(),
            }),
            definition_provider: Some(OneOf::Left(true)),
            ..Default::default()
        };
        Ok(InitializeResult {
            capabilities: caps,
            server_info: None,
        })
    }
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.docs.read().await;
        if let Some(text) = docs.get(&uri) {
            if let Some(line) = text.lines().nth(pos.line as usize) {
                // Prefer dotted identifiers when present (e.g., a.b.c)
                let dotted = dotted_at(line, pos.character as usize);
                let word = if dotted.contains('.') {
                    dotted
                } else {
                    word_at(line, pos.character as usize)
                };
                if !word.is_empty() {
                    if let Some((l, s, e)) = find_decl_of(text, &word) {
                        let loc = Location {
                            uri: uri.clone(),
                            range: Range::new(
                                Position::new(l as u32, s as u32),
                                Position::new(l as u32, e as u32),
                            ),
                        };
                        return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
                    }
                }
            }
        }
        Ok(None)
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Questicle LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.docs.write().await.insert(uri.clone(), text.clone());
        self.publish_diagnostics(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            self.docs.write().await.insert(uri.clone(), text.clone());
            self.publish_diagnostics(uri, text).await;
        }
    }

    async fn completion(
        &self,
        _params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        let mut items = Vec::new();
        for kw in [
            "let", "fn", "if", "else", "while", "for", "in", "return", "true", "false", "null",
        ] {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }
        for bi in [
            "print", "clock", "random", "len", "keys", "push", "pop", "on", "emit", "host",
        ] {
            items.push(CompletionItem {
                label: bi.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                ..Default::default()
            });
        }
        for ty_kw in ["number", "string", "bool", "null", "any", "list", "map"] {
            items.push(CompletionItem {
                label: ty_kw.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                ..Default::default()
            });
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        // Very simple hover: show token under cursor if it's a keyword/builtin
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.docs.read().await;
        if let Some(text) = docs.get(&uri) {
            // naive: split by lines and try to identify a word
            if let Some(line) = text.lines().nth(pos.line as usize) {
                let word = word_at(line, pos.character as usize);
                let doc = builtin_doc(&word).or_else(|| keyword_doc(&word));
                if let Some(d) = doc {
                    let contents = HoverContents::Scalar(MarkedString::String(d.to_string()));
                    return Ok(Some(Hover {
                        contents,
                        range: None,
                    }));
                }
                // Try type info
                if let Ok(program) = Parser::new(text).parse_program() {
                    let tc = questicle::typecheck::check_program(&program);
                    // Support dotted paths like slime.name or deeper a.b.c
                    if !word.is_empty() {
                        if let Some(hover_str) = resolve_hover_type(&tc.env, &word) {
                            let contents = HoverContents::Scalar(MarkedString::String(hover_str));
                            return Ok(Some(Hover {
                                contents,
                                range: None,
                            }));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let docs = self.docs.read().await;
        if let Some(text) = docs.get(&uri) {
            let mut symbols: Vec<SymbolInformation> = Vec::new();
            // Very rough: look for lines like "let name =" or "fn name("
            for (i, line) in text.lines().enumerate() {
                let trimmed = line.trim_start();
                if let Some(rest) = trimmed.strip_prefix("let ") {
                    if let Some((name, _)) = rest.split_once(' ') {
                        #[allow(deprecated)]
                        symbols.push(SymbolInformation {
                            name: name.to_string(),
                            kind: SymbolKind::VARIABLE,
                            location: Location {
                                uri: uri.clone(),
                                range: Range::new(
                                    Position::new(i as u32, 0),
                                    Position::new(i as u32, line.len() as u32),
                                ),
                            },
                            tags: None,
                            deprecated: None,
                            container_name: None,
                        });
                    }
                } else if let Some(rest) = trimmed.strip_prefix("fn ") {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        #[allow(deprecated)]
                        symbols.push(SymbolInformation {
                            name,
                            kind: SymbolKind::FUNCTION,
                            location: Location {
                                uri: uri.clone(),
                                range: Range::new(
                                    Position::new(i as u32, 0),
                                    Position::new(i as u32, line.len() as u32),
                                ),
                            },
                            tags: None,
                            deprecated: None,
                            container_name: None,
                        });
                    }
                }
            }
            return Ok(Some(DocumentSymbolResponse::Flat(symbols)));
        }
        Ok(None)
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let docs = self.docs.read().await;
        if let Some(text) = docs.get(&uri) {
            // Always use tolerant token-based formatter to preserve comments
            let pretty = questicle::formatter::format_source(text);
            let edit = TextEdit {
                range: Range::new(Position::new(0, 0), Position::new(u32::MAX, u32::MAX)),
                new_text: pretty,
            };
            return Ok(Some(vec![edit]));
        }
        Ok(Some(vec![]))
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.docs.read().await;
        if let Some(text) = docs.get(&uri) {
            // Extract the line up to cursor and find a function-like token and arg index by counting commas
            let lines: Vec<&str> = text.lines().collect();
            if (pos.line as usize) < lines.len() {
                let line_str = lines[pos.line as usize];
                let upto_len = std::cmp::min(pos.character as usize, line_str.len());
                let upto = &line_str[..upto_len];
                if let Some((fname, arg_index)) = extract_call_context(upto) {
                    if let Ok(program) = Parser::new(text).parse_program() {
                        let tc = questicle::typecheck::check_program(&program);
                        if let Some(questicle::typecheck::Type::Func(params, ret)) =
                            tc.env.vars.get(&fname)
                        {
                            let label = format!(
                                "{}({}) -> {}",
                                fname,
                                params
                                    .iter()
                                    .map(|p| p.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                ret
                            );
                            let parameters: Vec<ParameterInformation> = params
                                .iter()
                                .enumerate()
                                .map(|(i, p)| ParameterInformation {
                                    label: ParameterLabel::Simple(format!("arg{}: {}", i + 1, p)),
                                    documentation: None,
                                })
                                .collect();
                            let sig = SignatureInformation {
                                label,
                                documentation: None,
                                parameters: Some(parameters),
                                active_parameter: Some(arg_index as u32),
                            };
                            return Ok(Some(SignatureHelp {
                                signatures: vec![sig],
                                active_signature: Some(0),
                                active_parameter: Some(arg_index as u32),
                            }));
                        }
                    }
                }
            }
        }
        Ok(None)
    }
}

impl Backend {
    async fn publish_diagnostics(&self, uri: Url, text: String) {
        // Parse and publish errors
        match Parser::new(&text).parse_program() {
            Ok(program) => {
                // Run type checker
                let tc = typecheck::check_program(&program);
                let mut diags = Vec::new();
                for e in tc.errors {
                    // Heuristic: if we know the subject (var/function name), find its first occurrence
                    let range = if let Some(ref name) = e.subject {
                        if let Some((line, start, end)) = find_first_occurrence(&text, name) {
                            Range::new(
                                Position::new(line as u32, start as u32),
                                Position::new(line as u32, end as u32),
                            )
                        } else {
                            Range::new(Position::new(0, 0), Position::new(0, 1))
                        }
                    } else {
                        Range::new(Position::new(0, 0), Position::new(0, 1))
                    };
                    // Append location and hint if available
                    let line = range.start.line + 1; // 1-based for display
                    let col = range.start.character + 1;
                    let full_msg = if let Some(ref hint) = e.hint {
                        format!(
                            "{} (at line {}, col {})\nHint: {}",
                            e.message, line, col, hint
                        )
                    } else {
                        format!("{} (at line {}, col {})", e.message, line, col)
                    };
                    let d = Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: None,
                        code_description: None,
                        source: Some("questicle-typecheck".into()),
                        message: full_msg,
                        related_information: None,
                        tags: None,
                        data: None,
                    };
                    diags.push(d);
                }
                self.client.publish_diagnostics(uri, diags, None).await;
            }
            Err(e) => {
                // crude mapping: only has line/col in our error types in most cases
                let (line, col, msg) = match e {
                    questicle::parser::ParseError::Unexpected { line, col } => {
                        (line, col, "Unexpected token".to_string())
                    }
                    questicle::parser::ParseError::Expected {
                        expected,
                        line,
                        col,
                    } => (line, col, format!("Expected {expected}")),
                    questicle::parser::ParseError::Eof => (1, 1, "Unexpected end of input".into()),
                };
                let diag = Diagnostic {
                    range: Range::new(
                        Position::new(
                            (line.saturating_sub(1)) as u32,
                            (col.saturating_sub(1)) as u32,
                        ),
                        Position::new(
                            (line.saturating_sub(1)) as u32,
                            (col.saturating_sub(1) + 1) as u32,
                        ),
                    ),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("questicle".into()),
                    message: msg,
                    related_information: None,
                    tags: None,
                    data: None,
                };
                self.client.publish_diagnostics(uri, vec![diag], None).await;
            }
        }
    }
}

fn word_at(line: &str, col: usize) -> String {
    fn is_ident(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    let col = col.min(line.len());
    let mut start = col;
    let chars: Vec<char> = line.chars().collect();
    while start > 0 && is_ident(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_ident(chars[end]) {
        end += 1;
    }
    chars[start..end].iter().collect()
}

fn find_first_occurrence(text: &str, needle: &str) -> Option<(usize, usize, usize)> {
    for (i, line) in text.lines().enumerate() {
        if let Some(pos) = line.find(needle) {
            return Some((i, pos, pos + needle.len()));
        }
    }
    None
}

fn find_decl_of(text: &str, name: &str) -> Option<(usize, usize, usize)> {
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(after) = trimmed.strip_prefix("let ") {
            let ident: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if ident == name {
                let start = line.find(&ident)?;
                return Some((i, start, start + ident.len()));
            }
        } else if let Some(after) = trimmed.strip_prefix("fn ") {
            let ident: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if ident == name {
                let start = line.find(&ident)?;
                return Some((i, start, start + ident.len()));
            }
        }
    }
    None
}

fn builtin_doc(name: &str) -> Option<&'static str> {
    match name {
        "print" => Some("print(...): prints values to console"),
        "clock" => Some("clock(): seconds since epoch (float)"),
        "random" => Some("random(): 0.0 <= n < 1.0"),
        "len" => Some("len(x): length of string/list/map"),
        "keys" => Some("keys(map): list of string keys"),
        "push" => Some("push(list, value): returns new list with value appended"),
        "pop" => Some("pop(list): returns last element or null"),
        "on" => Some("on(name, fn): register event handler"),
        "emit" => Some("emit(name, data): emit event"),
        "host" => Some("host(op, payload): call host bridge"),
        _ => None,
    }
}
fn keyword_doc(name: &str) -> Option<&'static str> {
    match name {
        "let" => Some("Declare variable: let x = 1;"),
        "fn" => Some("Function declaration or literal"),
        "if" => Some("If statement: if (cond) { ... } else { ... }"),
        "while" => Some("While loop: while (cond) { ... }"),
        "for" => Some("For-in: for (i in list) { ... }"),
        "in" => Some("Used in for-in loops"),
        "return" => Some("Return from function"),
        "true" | "false" => Some("Boolean literal"),
        "null" => Some("Null literal"),
        _ => None,
    }
}

#[tokio::main]
async fn main() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend {
        client,
        docs: Arc::new(RwLock::new(HashMap::new())),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

// Very simple call context extractor: finds 'name(' and counts commas until current position.
fn extract_call_context(prefix: &str) -> Option<(String, usize)> {
    // Find last '(' to determine call start
    let bytes = prefix.as_bytes();
    let mut paren_idx: Option<usize> = None;
    for (i, &b) in bytes.iter().enumerate().rev() {
        if b == b'(' {
            paren_idx = Some(i);
            break;
        }
        if b == b')' {
            // We are inside a closed paren; bail
            return None;
        }
    }
    let start = paren_idx?;
    // Extract function name before '('
    let name_slice = &prefix[..start].trim_end();
    let fname: String = name_slice
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if fname.is_empty() {
        return None;
    }
    // Count commas after '('
    let mut depth = 0usize;
    let mut commas = 0usize;
    for &b in &bytes[start + 1..] {
        match b {
            b'(' => depth += 1,
            b')' => {
                if depth == 0 {
                    break;
                } else {
                    depth -= 1;
                }
            }
            b',' if depth == 0 => commas += 1,
            _ => {}
        }
    }
    Some((fname, commas))
}

// Extract a dotted identifier at the given column, e.g., slime.name or a.b.c
fn dotted_at(line: &str, col: usize) -> String {
    fn is_ident_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '.'
    }
    let col = col.min(line.len());
    let chars: Vec<char> = line.chars().collect();
    let mut start = col;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }
    chars[start..end].iter().collect()
}

// Resolve a hover type string from the type environment for a possibly dotted path
fn resolve_hover_type(env: &questicle::typecheck::TypeEnv, token: &str) -> Option<String> {
    use questicle::typecheck::Type;
    if token.is_empty() {
        return None;
    }
    // Split on '.'
    let parts: Vec<&str> = token.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let mut t = env.vars.get(parts[0]).cloned()?;
    // Walk subsequent fields if any
    for seg in &parts[1..] {
        match t {
            Type::Record(ref fields) => {
                t = fields.get(*seg).cloned().unwrap_or(Type::Any);
            }
            Type::Map(ref inner) => {
                // map values are homogeneous; field access returns inner type
                t = (**inner).clone();
            }
            _ => {
                // Not a record or map; cannot resolve further
                t = Type::Any;
            }
        }
    }
    Some(format!("{}: {}", token, t))
}

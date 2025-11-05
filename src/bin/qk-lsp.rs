use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use questicle::Parser;

struct Backend {
    client: Client,
    docs: Arc<RwLock<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
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
            ..Default::default()
        };
        Ok(InitializeResult {
            capabilities: caps,
            server_info: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Questicle LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
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

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
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
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
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
            }
        }
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
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
}

impl Backend {
    async fn publish_diagnostics(&self, uri: Url, text: String) {
        // Parse and publish errors
        match Parser::new(&text).parse_program() {
            Ok(_) => {
                self.client.publish_diagnostics(uri, vec![], None).await;
            }
            Err(e) => {
                // crude mapping: only has line/col in our error types in most cases
                let (line, col, msg) = match e {
                    questicle::parser::ParseError::Unexpected { line, col } => {
                        (line, col, format!("Unexpected token"))
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

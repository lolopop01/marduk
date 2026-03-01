//! LSP backend: document store, diagnostics, hover, and completion.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::analysis::{completion_context, find_enclosing_widget, word_at, Context};
use crate::knowledge::{prop_in_widget, widget_by_name, PropKind, WIDGETS};

// ── Backend ───────────────────────────────────────────────────────────────────

pub struct Backend {
    client: Client,
    docs: Arc<RwLock<HashMap<Url, String>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            docs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn update(&self, uri: Url, text: String) {
        let diagnostics = parse_diagnostics(&text);
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
        self.docs.write().await.insert(uri, text);
    }
}

// ── LanguageServer impl ───────────────────────────────────────────────────────

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        " ".to_string(),
                        ":".to_string(),
                        "\n".to_string(),
                    ]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "marduk-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "marduk-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    // ── Document lifecycle ────────────────────────────────────────────────────

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.update(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // We request FULL sync, so there's always exactly one change entry.
        if let Some(change) = params.content_changes.into_iter().last() {
            self.update(params.text_document.uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.docs.write().await.remove(&params.text_document.uri);
    }

    // ── Hover ─────────────────────────────────────────────────────────────────

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = &params.text_document_position_params.position;

        let docs = self.docs.read().await;
        let text = match docs.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        let word = match word_at(text, pos) {
            Some(w) => w,
            None => return Ok(None),
        };

        // Widget name hover
        if let Some(widget) = widget_by_name(word) {
            let md = format!("**{}**\n\n{}", widget.name, widget.doc);
            return Ok(Some(markdown_hover(md)));
        }

        // Property name hover — find the enclosing widget for context
        let before = text_before_pos(text, pos);
        if let Some(widget_name) = find_enclosing_widget(&before) {
            if let Some(prop) = prop_in_widget(&widget_name, word) {
                let kind_label = kind_doc(&prop.kind);
                let md = format!("**{}** · {}\n\n{}", prop.name, kind_label, prop.doc);
                return Ok(Some(markdown_hover(md)));
            }
        }

        Ok(None)
    }

    // ── Completion ────────────────────────────────────────────────────────────

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = &params.text_document_position.position;

        let docs = self.docs.read().await;
        let text = match docs.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        let items = match completion_context(text, pos) {
            Context::Widget => widget_items(),
            Context::Property { widget } => property_items(&widget),
            Context::Value { widget, prop } => value_items(&widget, &prop),
            Context::Unknown => vec![],
        };

        Ok(Some(CompletionResponse::Array(items)))
    }
}

// ── Diagnostics ───────────────────────────────────────────────────────────────

fn parse_diagnostics(text: &str) -> Vec<Diagnostic> {
    match marduk_mkml::parse_str(text) {
        Ok(_) => vec![],
        Err(e) => {
            // ParseError line/col are 1-based; LSP Position is 0-based.
            let line = e.line.saturating_sub(1) as u32;
            let col  = e.col.saturating_sub(1) as u32;
            vec![Diagnostic {
                range: Range {
                    start: Position::new(line, col),
                    end:   Position::new(line, col + 1),
                },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("marduk-lsp".to_string()),
                message: e.message.clone(),
                ..Default::default()
            }]
        }
    }
}

// ── Completion item builders ──────────────────────────────────────────────────

fn widget_items() -> Vec<CompletionItem> {
    WIDGETS
        .iter()
        .map(|w| {
            let detail = w.doc.lines().next().unwrap_or("").to_string();
            let mut item = CompletionItem::new_simple(w.name.to_string(), detail);
            item.kind = Some(CompletionItemKind::CLASS);
            item.insert_text = Some(format!("{} {{\n\t$0\n}}", w.name));
            item.insert_text_format = Some(InsertTextFormat::SNIPPET);
            item
        })
        .collect()
}

fn property_items(widget: &str) -> Vec<CompletionItem> {
    let info = match widget_by_name(widget) {
        Some(i) => i,
        None => return vec![],
    };
    info.props
        .iter()
        .map(|p| {
            let detail = p.doc.lines().next().unwrap_or("").to_string();
            let mut item = CompletionItem::new_simple(p.name.to_string(), detail);
            item.kind = Some(CompletionItemKind::PROPERTY);
            item.insert_text = Some(format!("{}: $0", p.name));
            item.insert_text_format = Some(InsertTextFormat::SNIPPET);
            item
        })
        .collect()
}

fn value_items(widget: &str, prop: &str) -> Vec<CompletionItem> {
    let prop_info = match prop_in_widget(widget, prop) {
        Some(p) => p,
        None => return vec![],
    };

    match &prop_info.kind {
        PropKind::Enum(variants) => variants
            .iter()
            .map(|v| {
                let mut item =
                    CompletionItem::new_simple(v.to_string(), String::new());
                item.kind = Some(CompletionItemKind::ENUM_MEMBER);
                item
            })
            .collect(),

        PropKind::Bool => vec![
            bool_item("0", "false"),
            bool_item("1", "true"),
        ],

        PropKind::Color => vec![{
            let mut item = CompletionItem::new_simple(
                "#rrggbbaa".to_string(),
                "Color literal (straight alpha, 8 hex digits)".to_string(),
            );
            item.kind = Some(CompletionItemKind::COLOR);
            item.insert_text = Some("#$0".to_string());
            item.insert_text_format = Some(InsertTextFormat::SNIPPET);
            item
        }],

        _ => vec![],
    }
}

fn bool_item(label: &str, detail: &str) -> CompletionItem {
    let mut item =
        CompletionItem::new_simple(label.to_string(), detail.to_string());
    item.kind = Some(CompletionItemKind::VALUE);
    item
}

// ── Misc helpers ──────────────────────────────────────────────────────────────

fn markdown_hover(md: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: md,
        }),
        range: None,
    }
}

fn kind_doc(kind: &PropKind) -> &'static str {
    match kind {
        PropKind::Number => "number",
        PropKind::Color => "color (`#rrggbbaa`)",
        PropKind::Bool => "bool (`0` or `1`)",
        PropKind::Enum(_) => "enum",
        PropKind::Event => "event name",
        PropKind::Font => "font name",
    }
}

fn text_before_pos(text: &str, pos: &Position) -> String {
    let line_idx = pos.line as usize;
    let col = pos.character as usize;
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i < line_idx {
            out.push_str(line);
            out.push('\n');
        } else if i == line_idx {
            out.push_str(&line[..col.min(line.len())]);
            break;
        } else {
            break;
        }
    }
    out
}

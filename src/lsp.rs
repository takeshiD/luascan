use crate::cli::LspOptions;
use crate::config::RuntimeVersion;
use crate::parser;
use anyhow::{Result, anyhow};
use jsonrpc::Result as LspResult;
use lsp_types::{
    Diagnostic, DidChangeTextDocumentParams, InitializeParams, InitializeResult, MessageType,
    Position, Range, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions,
};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tower_lsp::lsp_types::{DiagnosticSeverity, DidOpenTextDocumentParams, Url};
use tower_lsp::{Client, LanguageServer, LspService, Server, jsonrpc, lsp_types};
use tracing::{Level, event};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Backend {
    client: Client,
    root: Arc<RwLock<Option<PathBuf>>>,
}

impl Backend {
    fn new(client: Client, _: LspOptions) -> Self {
        Self {
            client,
            root: Arc::new(RwLock::new(None)),
        }
    }
    async fn check_syntax(&self, url: Url) {
        let path = url
            .to_file_path()
            .expect("failed to convert from url to path");
        if path.exists() {
            let mut file = File::open(path).expect("failed to open");
            let mut content = String::new();
            file.read_to_string(&mut content)
                .expect("failed to read content");
            let diagnotics: Vec<Diagnostic> =
                parser::parse(content.as_str(), crate::config::RuntimeVersion::Lua51)
                    .iter()
                    .map(|d| Diagnostic {
                        range: Range {
                            start: Position {
                                line: d.loc.line_start as u32,
                                character: d.loc.col_start as u32,
                            },
                            end: Position {
                                line: d.loc.line_end as u32,
                                character: d.loc.col_end as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: d.msg.clone(),
                        ..Diagnostic::default()
                    })
                    .collect();
            let log_msg = format!("chech syntax {:?} in {:?}", diagnotics, url);
            self.client
                .log_message(MessageType::INFO, log_msg.clone())
                .await;
            event!(Level::INFO, "{}", log_msg);
            self.client
                .publish_diagnostics(url.clone(), diagnotics, None)
                .await;
        }
    }
    async fn set_root(&self, path: PathBuf) -> Result<()> {
        if path.exists() {
            let root_ref = Arc::clone(&self.root);
            if let Ok(mut writer) = root_ref.write() {
                *writer = Some(path)
            }
            Ok(())
        } else {
            Err(anyhow!(
                "failed to set root path. {:?} is not existed.",
                path
            ))
        }
    }
    async fn get_root(&self) -> Option<PathBuf> {
        let root_ref = Arc::clone(&self.root);
        if let Ok(reader) = root_ref.read() {
            reader.clone()
        } else {
            None
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        let text_document_sync = TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::INCREMENTAL),
            will_save: Some(false),
            will_save_wait_until: Some(false),
            save: None,
        });
        let server_info = Some(ServerInfo {
            name: "luascan".to_string(),
            version: Some(VERSION.to_string()),
        });
        if let Some(url) = params.root_uri {
            let _ = self.set_root(PathBuf::from(url.to_string())).await;
        }
        Ok(InitializeResult {
            server_info,
            capabilities: ServerCapabilities {
                text_document_sync: Some(text_document_sync),
                ..ServerCapabilities::default()
            },
        })
    }
    async fn shutdown(&self) -> LspResult<()> {
        let log_msg = format!("shutdown in {:?}", self.get_root().await);
        self.client
            .log_message(MessageType::INFO, log_msg.clone())
            .await;
        event!(Level::INFO, "{}", log_msg);
        Ok(())
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let log_msg = format!("did open in {:?}", self.get_root().await);
        self.client
            .log_message(MessageType::INFO, log_msg.clone())
            .await;
        event!(Level::INFO, "{}", log_msg);
        if let Ok(path) = params.text_document.uri.to_file_path()
            && path.is_file()
            && params.text_document.language_id == "lua"
        {
            let uri = params.text_document.uri;
            self.check_syntax(uri).await;
        }
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let log_msg = format!("did change in {:?}", self.get_root().await);
        self.client
            .log_message(MessageType::INFO, log_msg.clone())
            .await;
        event!(Level::INFO, "{}", log_msg);
        if let Ok(path) = params.text_document.uri.to_file_path()
            && path.is_file()
        {
            let uri = params.text_document.uri;
            self.check_syntax(uri).await;
        }
    }
}

pub async fn run(options: LspOptions) -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let shared = Arc::new(options);

    let (service, socket) = LspService::new(move |client| {
        let options = shared.as_ref().clone();
        Backend::new(client, options)
    });

    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}

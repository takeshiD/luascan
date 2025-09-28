use crate::cli::LspOptions;
use anyhow::{Result, anyhow};
use jsonrpc::Result as LspResult;
use lsp_types::{
    InitializeParams, InitializeResult, MessageType, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
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

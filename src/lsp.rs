use crate::cli::LspOptions;
use crate::parser;
use anyhow::{Result, anyhow};
use clap::builder::Str;
use glob::glob;
use jsonrpc::Result as LspResult;
use lsp_types::{
    Diagnostic, InitializeParams, InitializeResult, InitializedParams, MessageType, OneOf,
    Position, Range, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tower_lsp::lsp_types::{
    DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, PositionEncodingKind, Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server, jsonrpc, lsp_types};
use tracing::{Level, event};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Backend {
    client: Client,
    root: Arc<RwLock<Option<PathBuf>>>,
    workspace: Arc<RwLock<HashMap<PathBuf, String>>>,
}

impl Backend {
    fn new(client: Client, _: LspOptions) -> Self {
        Self {
            client,
            root: Arc::new(RwLock::new(None)),
            workspace: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    async fn check_syntax(&self, uri: Url, content: String) {
        let start = Instant::now();
        let diagnotics: Vec<Diagnostic> =
            parser::parse(content.as_str(), crate::config::RuntimeVersion::Lua51)
                .iter()
                .map(|d| Diagnostic {
                    range: Range {
                        start: Position {
                            line: (d.loc.line_start as u32).saturating_sub(1),
                            character: (d.loc.col_start as u32).saturating_sub(1),
                        },
                        end: Position {
                            line: (d.loc.line_end as u32).saturating_sub(1),
                            character: (d.loc.col_end as u32).saturating_sub(1),
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: d.msg.clone(),
                    code: Some(lsp_types::NumberOrString::String(
                        "luascan code".to_string(),
                    )),
                    code_description: Some(lsp_types::CodeDescription {
                        href: Url::parse("http://example.com").expect("parse url failed"),
                    }),
                    source: Some("luascan source".to_string()),
                    ..Diagnostic::default()
                })
                .collect();
        let elapsed = start.elapsed();
        let log_msg = format!(
            "check syntax {:?} , elapsed {}.{:03}ms",
            diagnotics,
            elapsed.as_millis(),
            elapsed.as_millis()
        );
        self.client
            .log_message(MessageType::INFO, log_msg.clone())
            .await;
        event!(Level::INFO, "{}", log_msg);
        self.client
            .publish_diagnostics(uri.clone(), diagnotics.clone(), None)
            .await;
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
    async fn set_doc(&self, path: PathBuf, content: String) {
        let ws_ref = Arc::clone(&self.workspace);
        if let Ok(mut writer) = ws_ref.write() {
            writer.insert(path, content);
        }
    }
    async fn get_doc(&self, path: PathBuf) -> Option<String> {
        let ws_ref = Arc::clone(&self.workspace);
        if let Ok(reader) = ws_ref.read() {
            reader.get(&path).cloned()
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
            change: Some(TextDocumentSyncKind::FULL),
            will_save: Some(false),
            will_save_wait_until: Some(false),
            save: None,
        });
        let position_encoding = match params.capabilities.general {
            Some(general_cap) => match general_cap.position_encodings {
                Some(encodings) => {
                    if encodings.contains(&PositionEncodingKind::UTF8) {
                        Some(PositionEncodingKind::UTF8)
                    } else if encodings.contains(&PositionEncodingKind::UTF16) {
                        Some(PositionEncodingKind::UTF16)
                    } else if encodings.contains(&PositionEncodingKind::UTF32) {
                        Some(PositionEncodingKind::UTF32)
                    } else {
                        None
                    }
                }
                None => None,
            },
            None => None,
        };
        let server_info = Some(ServerInfo {
            name: "luascan".to_string(),
            version: Some(VERSION.to_string()),
        });
        if let Some(url) = params.root_uri.clone() {
            let mut path = PathBuf::from(url.path());
            let _ = self.set_root(path.clone()).await;
            // path.push("**/*.lua");
            // for entry in glob(path.to_str().expect("failed to convert from path to str"))
            //     .expect("failed to read path")
            // {
            //     match entry {
            //         Ok(p) => {
            //             event!(Level::INFO, "read {:?} in workspace", &p);
            //             let mut content = String::new();
            //             let mut file = File::open(&p).expect("failed to open file");
            //             file.read_to_string(&mut content)
            //                 .expect("failed to read file");
            //             self.set_doc(p.clone(), content.clone()).await;
            //             let uri = Url::from_file_path(
            //                 p.to_str().expect("failed to convert from path to str"),
            //             )
            //             .expect("failed to parse url");
            //             self.check_syntax(uri, content).await;
            //         }
            //         Err(e) => {
            //             event!(Level::INFO, "glob error {:?}", e);
            //         }
            //     }
            // }
        }
        Ok(InitializeResult {
            server_info,
            capabilities: ServerCapabilities {
                text_document_sync: Some(text_document_sync),
                position_encoding,
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let mut root_path = self.get_root().await.expect("failed to get root path");
        root_path.push("**/*.lua");
        for entry in glob(
            root_path
                .to_str()
                .expect("failed to convert from path to str"),
        )
        .expect("failed to read path")
        {
            match entry {
                Ok(p) => {
                    event!(Level::INFO, "read {:?} in workspace", &p);
                    let mut content = String::new();
                    let mut file = File::open(&p).expect("failed to open file");
                    file.read_to_string(&mut content)
                        .expect("failed to read file");
                    self.set_doc(p.clone(), content.clone()).await;
                    let uri = Url::from_file_path(
                        p.to_str().expect("failed to convert from path to str"),
                    )
                    .expect("failed to parse url");
                    self.check_syntax(uri, content).await;
                }
                Err(e) => {
                    event!(Level::INFO, "glob error {:?}", e);
                }
            }
        }
        let log_msg = format!("initialized in {:?}", self.get_root().await);
        self.client
            .log_message(MessageType::INFO, log_msg.clone())
            .await;
        event!(Level::INFO, "{}", log_msg);
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
            let content = params.text_document.text;
            self.set_doc(PathBuf::from(uri.path()), content.clone())
                .await;
            self.check_syntax(uri, content).await;
        }
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let log_msg = format!("did change in {:?}", self.get_root().await);
        self.client
            .log_message(MessageType::INFO, log_msg.clone())
            .await;
        event!(Level::INFO, "{}", log_msg);
        let uri = params.text_document.uri;
        let content = params.content_changes[0].text.clone();
        if let Ok(path) = uri.to_file_path()
            && path.is_file()
        {
            self.check_syntax(uri, content.clone()).await;
        }
    }
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let log_msg = format!("did save in {:?}", self.get_root().await);
        self.client
            .log_message(MessageType::INFO, log_msg.clone())
            .await;
        event!(Level::INFO, "{}", log_msg);
        let uri = params.text_document.uri;
        if let Ok(path) = uri.to_file_path()
            && path.is_file()
            && let Some(content) = params.text
        {
            self.check_syntax(uri, content.clone()).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, duplex};
    use tower_lsp::LspService;

    use crate::{
        cli::LspOptions,
        config::{Config, RuntimeConfig, WorkspaceConfig},
        lsp::Backend,
    };

    fn req(msg: &str) -> String {
        format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg)
    }
    fn create_lsp(options: LspOptions) -> (DuplexStream, DuplexStream) {
        let (service, socket) = LspService::new(|client| Backend::new(client, options));
        let (req_client, req_server) = duplex(64);
        let (resp_client, resp_server) = duplex(64);
        tokio::spawn(Server::new(req_server, resp_server, socket).serve(service));
        // req_client --> req_server ==LSP==> resp_server --> resp_client
        (req_client, resp_client)
    }

    #[tokio::test]
    async fn test_initialize_lsp() {
        let options = LspOptions {
            config: Config {
                runtime: RuntimeConfig::default(),
                workspace: WorkspaceConfig {
                    library: Vec::new(),
                },
            },
        };
        let (mut req_client, mut resp_client) = create_lsp(options);
        let init_request = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "capabilities": {
                    "textDocumentSync": 1
                }
            }
        }"#;
        req_client
            .write_all(req(init_request).as_bytes())
            .await
            .unwrap();
        let mut buf = vec![0; 1024];
        let _ = resp_client.read(&mut buf).await.unwrap();
        assert!(!buf.is_empty())
    }
}

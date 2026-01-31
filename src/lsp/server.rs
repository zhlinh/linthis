// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! LSP server implementation for linthis.
//!
//! This module provides the main language server that handles LSP protocol
//! messages and integrates with the linthis linting engine.

use std::path::PathBuf;
use std::sync::Arc;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use super::diagnostics::to_diagnostics;
use super::document::DocumentManager;
use crate::config::resolver::SharedConfigResolver;
use crate::{run, Language, RunMode, RunOptions};

/// LSP communication mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LspMode {
    /// Standard input/output (default, most compatible)
    #[default]
    Stdio,
    /// TCP socket communication
    Tcp,
}

impl std::str::FromStr for LspMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdio" => Ok(LspMode::Stdio),
            "tcp" => Ok(LspMode::Tcp),
            _ => Err(format!("Unknown LSP mode: {}. Use 'stdio' or 'tcp'.", s)),
        }
    }
}

/// The linthis language server.
pub struct LinthisLanguageServer {
    /// LSP client for sending notifications.
    client: Client,
    /// Document manager for tracking open files.
    documents: Arc<DocumentManager>,
    /// Config resolver for plugin configs (priority-based lookup)
    config_resolver: Option<SharedConfigResolver>,
}

impl LinthisLanguageServer {
    /// Create a new language server instance with a config resolver.
    pub fn with_config_resolver(client: Client, config_resolver: Option<SharedConfigResolver>) -> Self {
        Self {
            client,
            documents: Arc::new(DocumentManager::new()),
            config_resolver,
        }
    }

    /// Run linting on a document and publish diagnostics.
    async fn lint_document(&self, uri: Url) {
        // Get the document path
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                log::warn!("Cannot convert URI to file path: {}", uri);
                return;
            }
        };

        // Check if the file has a supported language
        if Language::from_path(&path).is_none() {
            // Not a supported file, clear any existing diagnostics
            self.client
                .publish_diagnostics(uri, vec![], None)
                .await;
            return;
        }

        // Run linting
        let diagnostics = self.run_lint(&path).await;

        // Publish diagnostics
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    /// Run linthis check on a file and return LSP diagnostics.
    async fn run_lint(&self, path: &PathBuf) -> Vec<Diagnostic> {
        let options = RunOptions {
            paths: vec![path.clone()],
            mode: RunMode::CheckOnly,
            quiet: true,
            no_cache: true, // Don't use cache for LSP (we want fresh results)
            config_resolver: self.config_resolver.clone(),
            ..Default::default()
        };

        match run(&options) {
            Ok(result) => {
                // Filter issues for this specific file
                let file_issues: Vec<_> = result
                    .issues
                    .iter()
                    .filter(|issue| {
                        // Normalize paths for comparison
                        let issue_path = issue.file_path.canonicalize().ok();
                        let target_path = path.canonicalize().ok();
                        match (issue_path, target_path) {
                            (Some(ip), Some(tp)) => ip == tp,
                            _ => issue.file_path == *path,
                        }
                    })
                    .collect();

                to_diagnostics(&file_issues.into_iter().cloned().collect::<Vec<_>>())
            }
            Err(e) => {
                log::error!("Lint error for {}: {}", path.display(), e);
                vec![]
            }
        }
    }

    /// Clear diagnostics for a document.
    async fn clear_diagnostics(&self, uri: Url) {
        self.client
            .publish_diagnostics(uri, vec![], None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LinthisLanguageServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // Sync open/close/save events
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                        ..Default::default()
                    },
                )),
                // TODO: Add formatting support in future
                // document_formatting_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "linthis".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        log::info!("linthis LSP server initialized");
        self.client
            .log_message(MessageType::INFO, "linthis LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        log::info!("linthis LSP server shutting down");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let content = params.text_document.text;
        let version = params.text_document.version;

        // Track the document
        self.documents.open(uri.clone(), content, version).await;

        // Lint on open
        self.lint_document(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;

        // Update document content (we get full content since we use FULL sync)
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.change(&uri, change.text, version).await;
        }

        // Note: We don't lint on change to avoid performance issues.
        // Linting happens on save instead.
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Lint on save
        self.lint_document(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        // Remove from tracking
        self.documents.close(&uri).await;

        // Clear diagnostics for closed file
        self.clear_diagnostics(uri).await;
    }
}

/// Run the LSP server.
///
/// # Arguments
/// * `mode` - Communication mode (stdio or tcp)
/// * `port` - TCP port (only used when mode is tcp)
pub async fn run_lsp_server(mode: LspMode, port: u16) -> anyhow::Result<()> {
    run_lsp_server_with_config(mode, port, None).await
}

/// Run the LSP server with an optional config resolver.
///
/// # Arguments
/// * `mode` - Communication mode (stdio or tcp)
/// * `port` - TCP port (only used when mode is tcp)
/// * `config_resolver` - Optional config resolver for plugin configs
pub async fn run_lsp_server_with_config(
    mode: LspMode,
    port: u16,
    config_resolver: Option<SharedConfigResolver>,
) -> anyhow::Result<()> {
    match mode {
        LspMode::Stdio => run_stdio_server(config_resolver).await,
        LspMode::Tcp => run_tcp_server(port, config_resolver).await,
    }
}

/// Run the LSP server over stdio.
async fn run_stdio_server(config_resolver: Option<SharedConfigResolver>) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(move |client| {
        LinthisLanguageServer::with_config_resolver(client, config_resolver.clone())
    });
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}

/// Run the LSP server over TCP.
async fn run_tcp_server(port: u16, config_resolver: Option<SharedConfigResolver>) -> anyhow::Result<()> {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    eprintln!("linthis LSP server listening on 127.0.0.1:{}", port);

    loop {
        let (stream, addr) = listener.accept().await?;
        eprintln!("Client connected from {}", addr);

        let (read, write) = tokio::io::split(stream);

        let resolver = config_resolver.clone();
        let (service, socket) = LspService::new(move |client| {
            LinthisLanguageServer::with_config_resolver(client, resolver.clone())
        });
        Server::new(read, write, socket).serve(service).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_mode_from_str() {
        assert_eq!("stdio".parse::<LspMode>().unwrap(), LspMode::Stdio);
        assert_eq!("tcp".parse::<LspMode>().unwrap(), LspMode::Tcp);
        assert_eq!("STDIO".parse::<LspMode>().unwrap(), LspMode::Stdio);
        assert_eq!("TCP".parse::<LspMode>().unwrap(), LspMode::Tcp);
        assert!("invalid".parse::<LspMode>().is_err());
    }

    #[test]
    fn test_lsp_mode_default() {
        assert_eq!(LspMode::default(), LspMode::Stdio);
    }
}

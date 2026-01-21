// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Document state management for the LSP server.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;

/// State of a single open document.
#[derive(Debug, Clone)]
pub struct DocumentState {
    /// The URI of the document.
    #[allow(dead_code)]
    pub uri: Url,
    /// The file path on disk.
    #[allow(dead_code)]
    pub path: PathBuf,
    /// The document version (incremented on each change).
    pub version: i32,
    /// The current content of the document.
    pub content: String,
}

impl DocumentState {
    /// Create a new document state.
    pub fn new(uri: Url, content: String, version: i32) -> Self {
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));

        Self {
            uri,
            path,
            version,
            content,
        }
    }
}

/// Manages the state of all open documents.
#[derive(Debug, Default)]
pub struct DocumentManager {
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
}

impl DocumentManager {
    /// Create a new document manager.
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Track a newly opened document.
    pub async fn open(&self, uri: Url, content: String, version: i32) {
        let state = DocumentState::new(uri.clone(), content, version);
        let mut docs = self.documents.write().await;
        docs.insert(uri, state);
    }

    /// Update the content of an open document.
    pub async fn change(&self, uri: &Url, content: String, version: i32) {
        let mut docs = self.documents.write().await;
        if let Some(state) = docs.get_mut(uri) {
            state.content = content;
            state.version = version;
        }
    }

    /// Remove a closed document.
    pub async fn close(&self, uri: &Url) {
        let mut docs = self.documents.write().await;
        docs.remove(uri);
    }

    /// Get the state of a document.
    #[allow(dead_code)]
    pub async fn get(&self, uri: &Url) -> Option<DocumentState> {
        let docs = self.documents.read().await;
        docs.get(uri).cloned()
    }

    /// Get the file path for a document URI.
    #[allow(dead_code)]
    pub async fn get_path(&self, uri: &Url) -> Option<PathBuf> {
        let docs = self.documents.read().await;
        docs.get(uri).map(|s| s.path.clone())
    }

    /// Get all open document URIs.
    #[allow(dead_code)]
    pub async fn all_uris(&self) -> Vec<Url> {
        let docs = self.documents.read().await;
        docs.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_lifecycle() {
        let manager = DocumentManager::new();
        let uri = Url::parse("file:///test/file.py").unwrap();

        // Open document
        manager
            .open(uri.clone(), "print('hello')".to_string(), 1)
            .await;

        // Check it exists
        let state = manager.get(&uri).await;
        assert!(state.is_some());
        let state = state.unwrap();
        assert_eq!(state.version, 1);
        assert_eq!(state.content, "print('hello')");

        // Update document
        manager
            .change(&uri, "print('world')".to_string(), 2)
            .await;

        let state = manager.get(&uri).await.unwrap();
        assert_eq!(state.version, 2);
        assert_eq!(state.content, "print('world')");

        // Close document
        manager.close(&uri).await;
        assert!(manager.get(&uri).await.is_none());
    }
}

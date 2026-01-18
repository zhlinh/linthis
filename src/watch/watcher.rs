// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! File system watcher using the notify crate.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crate::Language;

/// Kind of watch event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEventKind {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted
    Removed,
    /// File was renamed
    Renamed,
}

/// A watch event representing a file change
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// The path that changed
    pub path: PathBuf,
    /// Kind of change
    pub kind: WatchEventKind,
    /// Detected language (if any)
    pub language: Option<Language>,
}

impl WatchEvent {
    /// Create a new watch event
    pub fn new(path: PathBuf, kind: WatchEventKind) -> Self {
        let language = Language::from_path(&path);
        Self {
            path,
            kind,
            language,
        }
    }

    /// Check if this is a file we should lint
    pub fn is_lintable(&self) -> bool {
        self.language.is_some()
    }
}

/// Errors that can occur in file watching
#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("Failed to create watcher: {0}")]
    WatcherCreation(#[from] notify::Error),

    #[error("Failed to watch path {path}: {message}")]
    WatchPath { path: PathBuf, message: String },

    #[error("Watch channel closed")]
    ChannelClosed,
}

/// File system watcher
pub struct FileWatcher {
    /// The underlying notify watcher
    _watcher: RecommendedWatcher,
    /// Receiver for watch events
    rx: Receiver<WatchEvent>,
    /// Paths being watched
    watched_paths: Vec<PathBuf>,
}

impl FileWatcher {
    /// Create a new file watcher for the given paths
    pub fn new(paths: Vec<PathBuf>) -> Result<Self, WatchError> {
        let (tx, rx) = mpsc::channel();

        let event_tx = tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    Self::handle_event(event, &event_tx);
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(100)),
        )?;

        // Watch all paths
        let mut watched_paths = Vec::new();
        for path in &paths {
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            watcher
                .watch(&canonical, RecursiveMode::Recursive)
                .map_err(|e| WatchError::WatchPath {
                    path: canonical.clone(),
                    message: e.to_string(),
                })?;
            watched_paths.push(canonical);
        }

        Ok(Self {
            _watcher: watcher,
            rx,
            watched_paths,
        })
    }

    /// Handle a notify event and convert to WatchEvent
    fn handle_event(event: Event, tx: &Sender<WatchEvent>) {
        let kind = match event.kind {
            EventKind::Create(_) => Some(WatchEventKind::Created),
            EventKind::Modify(_) => Some(WatchEventKind::Modified),
            EventKind::Remove(_) => Some(WatchEventKind::Removed),
            EventKind::Any => None,
            EventKind::Access(_) => None,
            EventKind::Other => None,
        };

        if let Some(kind) = kind {
            for path in event.paths {
                // Skip directories
                if path.is_dir() {
                    continue;
                }

                // Skip hidden files and common non-source files
                if Self::should_skip_path(&path) {
                    continue;
                }

                let watch_event = WatchEvent::new(path, kind.clone());

                // Only send events for lintable files
                if watch_event.is_lintable() {
                    let _ = tx.send(watch_event);
                }
            }
        }
    }

    /// Check if a path should be skipped
    fn should_skip_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Skip hidden files and directories
        if let Some(name) = path.file_name() {
            if name.to_string_lossy().starts_with('.') {
                return true;
            }
        }

        // Skip common non-source paths
        let skip_patterns = [
            "/target/",
            "/node_modules/",
            "/.git/",
            "/build/",
            "/dist/",
            "/__pycache__/",
            "/.venv/",
            "/venv/",
            "/.idea/",
            "/.vscode/",
            "/vendor/",
            "/.gradle/",
            "/Pods/",
        ];

        for pattern in skip_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        // Skip lock files and generated files
        let skip_suffixes = [
            ".lock",
            ".log",
            ".min.js",
            ".min.css",
            ".generated.",
            ".g.dart",
            ".freezed.dart",
        ];

        for suffix in skip_suffixes {
            if path_str.ends_with(suffix) || path_str.contains(suffix) {
                return true;
            }
        }

        false
    }

    /// Get the next watch event (blocking)
    pub fn recv(&self) -> Result<WatchEvent, WatchError> {
        self.rx.recv().map_err(|_| WatchError::ChannelClosed)
    }

    /// Try to get the next watch event (non-blocking)
    pub fn try_recv(&self) -> Option<WatchEvent> {
        self.rx.try_recv().ok()
    }

    /// Get all pending events (non-blocking)
    pub fn drain_events(&self) -> Vec<WatchEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Get watched paths
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_watch_event_lintable() {
        let event = WatchEvent::new(PathBuf::from("test.rs"), WatchEventKind::Modified);
        assert!(event.is_lintable());
        assert_eq!(event.language, Some(Language::Rust));

        let event = WatchEvent::new(PathBuf::from("test.txt"), WatchEventKind::Modified);
        assert!(!event.is_lintable());
        assert_eq!(event.language, None);
    }

    #[test]
    fn test_should_skip_path() {
        assert!(FileWatcher::should_skip_path(Path::new(
            "/project/target/debug/main.rs"
        )));
        assert!(FileWatcher::should_skip_path(Path::new(
            "/project/.git/config"
        )));
        assert!(FileWatcher::should_skip_path(Path::new(
            "/project/node_modules/package/index.js"
        )));
        assert!(!FileWatcher::should_skip_path(Path::new(
            "/project/src/main.rs"
        )));
    }

    #[test]
    fn test_file_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(vec![temp_dir.path().to_path_buf()]);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_file_watcher_detects_changes() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(vec![temp_dir.path().to_path_buf()]).unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        // Canonicalize the test file path to handle symlinks (e.g., /var -> /private/var on macOS)
        let test_file_canonical = test_file.canonicalize().unwrap_or(test_file.clone());

        // Give the watcher time to detect the change
        std::thread::sleep(Duration::from_millis(200));

        // We should have received an event
        let events = watcher.drain_events();
        // Note: The number of events may vary depending on the platform
        // Some platforms may emit multiple events for a single file operation
        // Compare canonicalized paths to handle symlinks
        assert!(
            events.iter().any(|e| {
                let event_path = e.path.canonicalize().unwrap_or(e.path.clone());
                event_path == test_file_canonical
            }),
            "Expected to find event for {:?}, got {:?}",
            test_file_canonical,
            events
        );
    }
}

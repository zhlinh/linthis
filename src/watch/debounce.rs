// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Event debouncing to coalesce rapid file changes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::{WatchEvent, WatchEventKind};

/// Debouncer for coalescing rapid file change events.
///
/// When files are modified rapidly (e.g., during a save operation or
/// batch edit), we want to wait until the changes have settled before
/// triggering a lint run. This prevents unnecessary multiple runs.
#[derive(Debug)]
pub struct Debouncer {
    /// Debounce delay
    delay: Duration,
    /// Pending events (path -> (event, timestamp))
    pending: HashMap<PathBuf, (WatchEvent, Instant)>,
}

impl Debouncer {
    /// Create a new debouncer with the specified delay in milliseconds
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay: Duration::from_millis(delay_ms),
            pending: HashMap::new(),
        }
    }

    /// Add an event to the debouncer
    ///
    /// If an event for the same path already exists, it will be replaced
    /// and the timer will be reset.
    pub fn add_event(&mut self, event: WatchEvent) {
        // For remove events, don't debounce - process immediately
        if event.kind == WatchEventKind::Removed {
            self.pending.remove(&event.path);
            return;
        }

        // Update or insert the event with current timestamp
        self.pending
            .insert(event.path.clone(), (event, Instant::now()));
    }

    /// Get all events that have been debounced long enough
    ///
    /// Returns events whose timestamp is older than the debounce delay,
    /// removing them from the pending set.
    pub fn get_ready_events(&mut self) -> Vec<WatchEvent> {
        let now = Instant::now();
        let mut ready = Vec::new();

        // Collect paths that are ready
        let ready_paths: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, (_, timestamp))| now.duration_since(*timestamp) >= self.delay)
            .map(|(path, _)| path.clone())
            .collect();

        // Remove and collect ready events
        for path in ready_paths {
            if let Some((event, _)) = self.pending.remove(&path) {
                ready.push(event);
            }
        }

        ready
    }

    /// Get the number of pending events
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if there are any pending events
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Clear all pending events
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Get all pending paths (for display)
    pub fn pending_paths(&self) -> Vec<&PathBuf> {
        self.pending.keys().collect()
    }

    /// Get time until next event is ready (if any)
    pub fn time_until_ready(&self) -> Option<Duration> {
        let now = Instant::now();

        self.pending
            .values()
            .map(|(_, timestamp)| {
                let elapsed = now.duration_since(*timestamp);
                if elapsed >= self.delay {
                    Duration::ZERO
                } else {
                    self.delay - elapsed
                }
            })
            .min()
    }

    /// Force all pending events to be ready
    pub fn flush(&mut self) -> Vec<WatchEvent> {
        self.pending
            .drain()
            .map(|(_, (event, _))| event)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_debouncer_basic() {
        let mut debouncer = Debouncer::new(100);

        // Add an event
        let event = WatchEvent::new(PathBuf::from("test.rs"), WatchEventKind::Modified);
        debouncer.add_event(event);

        // Should not be ready immediately
        let ready = debouncer.get_ready_events();
        assert!(ready.is_empty());
        assert_eq!(debouncer.pending_count(), 1);

        // Wait for debounce delay
        thread::sleep(Duration::from_millis(150));

        // Now should be ready
        let ready = debouncer.get_ready_events();
        assert_eq!(ready.len(), 1);
        assert_eq!(debouncer.pending_count(), 0);
    }

    #[test]
    fn test_debouncer_coalesce() {
        let mut debouncer = Debouncer::new(100);

        // Add multiple events for same file
        for _ in 0..5 {
            let event = WatchEvent::new(PathBuf::from("test.rs"), WatchEventKind::Modified);
            debouncer.add_event(event);
            thread::sleep(Duration::from_millis(20));
        }

        // Should still only have one pending
        assert_eq!(debouncer.pending_count(), 1);

        // Wait for debounce
        thread::sleep(Duration::from_millis(150));

        // Should get only one event
        let ready = debouncer.get_ready_events();
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn test_debouncer_multiple_files() {
        let mut debouncer = Debouncer::new(50);

        // Add events for different files
        debouncer.add_event(WatchEvent::new(
            PathBuf::from("a.rs"),
            WatchEventKind::Modified,
        ));
        debouncer.add_event(WatchEvent::new(
            PathBuf::from("b.rs"),
            WatchEventKind::Modified,
        ));
        debouncer.add_event(WatchEvent::new(
            PathBuf::from("c.rs"),
            WatchEventKind::Modified,
        ));

        assert_eq!(debouncer.pending_count(), 3);

        // Wait and get all
        thread::sleep(Duration::from_millis(100));
        let ready = debouncer.get_ready_events();
        assert_eq!(ready.len(), 3);
    }

    #[test]
    fn test_debouncer_remove_not_debounced() {
        let mut debouncer = Debouncer::new(100);

        // Add a modify event
        debouncer.add_event(WatchEvent::new(
            PathBuf::from("test.rs"),
            WatchEventKind::Modified,
        ));

        // Add a remove event for same file
        debouncer.add_event(WatchEvent::new(
            PathBuf::from("test.rs"),
            WatchEventKind::Removed,
        ));

        // Remove should clear the pending event
        assert_eq!(debouncer.pending_count(), 0);
    }

    #[test]
    fn test_debouncer_flush() {
        let mut debouncer = Debouncer::new(1000);

        debouncer.add_event(WatchEvent::new(
            PathBuf::from("a.rs"),
            WatchEventKind::Modified,
        ));
        debouncer.add_event(WatchEvent::new(
            PathBuf::from("b.rs"),
            WatchEventKind::Modified,
        ));

        // Flush should return all without waiting
        let events = debouncer.flush();
        assert_eq!(events.len(), 2);
        assert_eq!(debouncer.pending_count(), 0);
    }

    #[test]
    fn test_time_until_ready() {
        let mut debouncer = Debouncer::new(100);

        // No pending events
        assert!(debouncer.time_until_ready().is_none());

        // Add event
        debouncer.add_event(WatchEvent::new(
            PathBuf::from("test.rs"),
            WatchEventKind::Modified,
        ));

        // Should have time remaining
        let time = debouncer.time_until_ready().unwrap();
        assert!(time > Duration::ZERO);
        assert!(time <= Duration::from_millis(100));

        // Wait for debounce
        thread::sleep(Duration::from_millis(150));

        // Should be zero or very small
        let time = debouncer.time_until_ready().unwrap();
        assert!(time <= Duration::from_millis(1));
    }
}

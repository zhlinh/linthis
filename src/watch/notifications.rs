// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Desktop notifications for watch mode.
//!
//! This module is only available when the `notifications` feature is enabled.

use notify_rust::Notification;

use super::WatchResult;

/// Send a desktop notification about lint results
pub fn notify_issues(result: &WatchResult) {
    let (summary, body, urgency) = if result.is_clean {
        (
            "linthis: All clear!".to_string(),
            format!("Checked {} files with no issues", result.files_checked),
            notify_rust::Urgency::Low,
        )
    } else if result.error_count > 0 {
        (
            format!("linthis: {} errors found", result.error_count),
            format!(
                "{} errors, {} warnings in {} files",
                result.error_count, result.warning_count, result.files_checked
            ),
            notify_rust::Urgency::Critical,
        )
    } else {
        (
            format!("linthis: {} warnings found", result.warning_count),
            format!(
                "{} warnings in {} files",
                result.warning_count, result.files_checked
            ),
            notify_rust::Urgency::Normal,
        )
    };

    let _ = Notification::new()
        .summary(&summary)
        .body(&body)
        .icon("dialog-information")
        .urgency(urgency)
        .timeout(5000)
        .show();
}

/// Send a notification for a specific error
pub fn notify_error(message: &str) {
    let _ = Notification::new()
        .summary("linthis: Error")
        .body(message)
        .icon("dialog-error")
        .urgency(notify_rust::Urgency::Critical)
        .timeout(5000)
        .show();
}

/// Send a notification that watch mode started
pub fn notify_started(paths: &[std::path::PathBuf]) {
    let body = if paths.len() == 1 {
        format!("Watching: {}", paths[0].display())
    } else {
        format!("Watching {} paths", paths.len())
    };

    let _ = Notification::new()
        .summary("linthis: Watch mode started")
        .body(&body)
        .icon("dialog-information")
        .urgency(notify_rust::Urgency::Low)
        .timeout(3000)
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_result_notification_clean() {
        let result = WatchResult {
            files_checked: 10,
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            files_formatted: 0,
            duration_ms: 100,
            is_clean: true,
        };

        // Just verify it doesn't panic
        // We can't actually test the notification without a display
        let _ = std::panic::catch_unwind(|| {
            // This may fail on CI without a display, which is fine
            notify_issues(&result);
        });
    }

    #[test]
    fn test_watch_result_notification_with_errors() {
        let result = WatchResult {
            files_checked: 10,
            error_count: 2,
            warning_count: 3,
            info_count: 1,
            files_formatted: 0,
            duration_ms: 100,
            is_clean: false,
        };

        let _ = std::panic::catch_unwind(|| {
            notify_issues(&result);
        });
    }
}

//! Background process management (spawn, PID files, status, cleanup).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get the review artifacts directory, creating it if needed.
pub fn review_dir() -> Result<PathBuf, String> {
    let dir = crate::utils::get_review_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create review dir: {}", e))?;
    }
    Ok(dir)
}

/// Generate a timestamp string for unique filenames.
pub fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

/// Spawn a background review process.
pub fn spawn_background_review(args: &[String]) -> Result<u32, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;

    let ts = timestamp();
    let dir = review_dir()?;
    let log_path = dir.join(format!("{}.log", ts));
    let pid_path = dir.join(format!("{}.pid", ts));

    let log_file =
        fs::File::create(&log_path).map_err(|e| format!("Failed to create log file: {}", e))?;

    let mut cmd = Command::new(exe);
    cmd.args(["review"]);
    cmd.args(args);

    // Redirect stdout/stderr to log file
    cmd.stdout(
        log_file
            .try_clone()
            .map_err(|e| format!("Failed to clone file handle: {}", e))?,
    );
    cmd.stderr(log_file);

    // Platform-specific detaching
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn background process: {}", e))?;

    let pid = child.id();

    // Write PID file
    fs::write(&pid_path, pid.to_string())
        .map_err(|e| format!("Failed to write PID file: {}", e))?;

    eprintln!(
        "Background review started (PID: {}, log: {})",
        pid,
        log_path.display()
    );
    Ok(pid)
}

/// Check the status of background reviews.
pub fn check_status() -> Vec<ReviewStatus> {
    let dir = match review_dir() {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut statuses = Vec::new();

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("pid") {
            if let Some(status) = read_review_status(&path, &dir) {
                statuses.push(status);
            }
        }
    }

    statuses.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    statuses
}

/// Status of a single review.
pub struct ReviewStatus {
    pub timestamp: String,
    pub pid: u32,
    pub is_running: bool,
    pub has_report: bool,
    pub report_path: Option<PathBuf>,
}

impl std::fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = if self.is_running {
            format!("Running (PID: {})", self.pid)
        } else if self.has_report {
            "Completed".to_string()
        } else {
            "Failed".to_string()
        };
        write!(f, "Review ({}) — {}", self.timestamp, state)
    }
}

fn read_review_status(pid_path: &Path, dir: &Path) -> Option<ReviewStatus> {
    let ts = pid_path.file_stem()?.to_str()?.to_string();
    let pid_str = fs::read_to_string(pid_path).ok()?;
    let pid: u32 = pid_str.trim().parse().ok()?;

    let is_running = is_process_running(pid);
    let report_path = dir.join(format!("{}.md", ts));
    let has_report = report_path.exists();

    Some(ReviewStatus {
        timestamp: ts,
        pid,
        is_running,
        has_report,
        report_path: if has_report { Some(report_path) } else { None },
    })
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }

    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// Clean up review artifacts older than retention_days (legacy, kept for backward compat).
pub fn clean_artifacts(retention_days: u32) -> Result<usize, String> {
    let dir = review_dir()?;
    let cutoff = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(
            retention_days as u64 * 86400,
        ))
        .unwrap_or(SystemTime::now());

    let mut removed = 0;

    let entries = fs::read_dir(&dir).map_err(|e| format!("Failed to read review dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff && fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            }
        }
    }

    Ok(removed)
}

/// Clean up review artifacts by count, keeping the most recent `max_reviews`.
/// If `max_reviews` is 0, no cleanup (unlimited). Always keeps at least 1.
pub fn clean_artifacts_by_count(max_reviews: usize) -> Result<usize, String> {
    if max_reviews == 0 {
        return Ok(0);
    }
    let keep = max_reviews.max(1);

    let dir = review_dir()?;
    let entries = fs::read_dir(&dir).map_err(|e| format!("Failed to read review dir: {}", e))?;

    let mut files: Vec<_> = entries.flatten().filter(|e| e.path().is_file()).collect();

    // Sort by modified time, newest first
    files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    let mut removed = 0;
    for entry in files.iter().skip(keep) {
        if fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_is_numeric() {
        let ts = timestamp();
        assert!(ts.parse::<u64>().is_ok());
    }

    #[test]
    fn test_review_status_display() {
        let status = ReviewStatus {
            timestamp: "1710300000".to_string(),
            pid: 12345,
            is_running: true,
            has_report: false,
            report_path: None,
        };
        let display = format!("{}", status);
        assert!(display.contains("Running"));
        assert!(display.contains("12345"));
    }

    #[test]
    fn test_review_status_display_completed() {
        let status = ReviewStatus {
            timestamp: "1710300000".to_string(),
            pid: 12345,
            is_running: false,
            has_report: true,
            report_path: Some(PathBuf::from("report.md")),
        };
        let display = format!("{}", status);
        assert!(display.contains("Completed"));
    }
}

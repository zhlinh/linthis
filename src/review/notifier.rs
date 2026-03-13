//! Notification system (system/webhook/custom).

use std::collections::HashMap;
use std::process::Command;
use crate::config::NotificationConfig;
use crate::review::ReviewResult;
use crate::review::report::generate_notification_summary;
use crate::review::Severity;

/// Template variables available for notification rendering.
pub fn build_template_vars(result: &ReviewResult, branch: &str, report_path: &str, pr_url: Option<&str>) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    vars.insert("status".to_string(), result.summary.assessment.to_string());
    vars.insert("branch".to_string(), branch.to_string());
    vars.insert("issues_count".to_string(), result.issues.len().to_string());
    let severity_counts = result.count_by_severity();
    vars.insert("critical_count".to_string(), severity_counts.get(&Severity::Critical).copied().unwrap_or(0).to_string());
    vars.insert("report_path".to_string(), report_path.to_string());
    vars.insert("pr_url".to_string(), pr_url.unwrap_or("").to_string());
    vars.insert("message".to_string(), generate_notification_summary(result));
    vars
}

/// Send notifications through all configured channels.
pub fn send_notifications(
    configs: &[NotificationConfig],
    vars: &HashMap<String, String>,
) -> Vec<Result<(), String>> {
    configs.iter().map(|config| {
        let result = match config {
            NotificationConfig::System => send_system_notification(vars),
            NotificationConfig::Webhook { url, method, headers, body_template } => {
                send_webhook(url, method, headers, body_template.as_deref(), vars)
            }
            NotificationConfig::Custom { command } => {
                send_custom(command, vars)
            }
        };

        if let Err(ref e) = result {
            eprintln!("Notification failed: {}", e);
        }

        result
    }).collect()
}

fn expand_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

fn send_system_notification(vars: &HashMap<String, String>) -> Result<(), String> {
    let message = vars.get("message").cloned().unwrap_or_default();
    let title = "linthis review";

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            message.replace('"', "\\\""),
            title
        );
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("macOS notification failed: {}", e))?;

        if !output.status.success() {
            return Err(format!("osascript failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("notify-send")
            .args([title, &message])
            .output()
            .map_err(|e| format!("Linux notification failed: {}", e))?;

        if !output.status.success() {
            return Err(format!("notify-send failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
    }

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!(
            "[System.Windows.Forms.MessageBox]::Show('{}', '{}')",
            message.replace('\'', "''"),
            title
        );
        let output = Command::new("powershell")
            .args(["-Command", &format!("Add-Type -AssemblyName System.Windows.Forms; {}", ps_cmd)])
            .output()
            .map_err(|e| format!("Windows notification failed: {}", e))?;

        if !output.status.success() {
            return Err(format!("PowerShell notification failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
    }

    Ok(())
}

fn send_webhook(
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body_template: Option<&str>,
    vars: &HashMap<String, String>,
) -> Result<(), String> {
    let body = body_template
        .map(|t| expand_template(t, vars))
        .unwrap_or_else(|| {
            format!(r#"{{"text": "{}"}}"#, vars.get("message").cloned().unwrap_or_default())
        });

    let mut cmd = Command::new("curl");
    cmd.args(["-s", "-X", method, url, "-d", &body]);

    // Add default Content-Type if not specified
    let mut has_content_type = false;
    for (key, value) in headers {
        cmd.args(["-H", &format!("{}: {}", key, value)]);
        if key.eq_ignore_ascii_case("content-type") {
            has_content_type = true;
        }
    }
    if !has_content_type {
        cmd.args(["-H", "Content-Type: application/json"]);
    }

    let output = cmd.output()
        .map_err(|e| format!("Webhook request failed: {}", e))?;

    if !output.status.success() {
        return Err(format!("Webhook returned error: {}", String::from_utf8_lossy(&output.stderr)));
    }

    Ok(())
}

fn send_custom(command: &str, vars: &HashMap<String, String>) -> Result<(), String> {
    let expanded = expand_template(command, vars);

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", &expanded])
            .output()
    } else {
        Command::new("sh")
            .args(["-c", &expanded])
            .output()
    };

    let output = output.map_err(|e| format!("Custom notification command failed: {}", e))?;

    if !output.status.success() {
        return Err(format!("Custom command failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{ReviewResult, ReviewSummary, Assessment};

    #[test]
    fn test_build_template_vars() {
        let result = ReviewResult {
            files: vec![],
            issues: vec![],
            base_ref: "main".to_string(),
            head_ref: "feature".to_string(),
            summary: ReviewSummary {
                files_reviewed: 0,
                total_issues: 0,
                critical_count: 0,
                important_count: 0,
                minor_count: 0,
                assessment: Assessment::Ready,
                summary_text: String::new(),
            },
        };

        let vars = build_template_vars(&result, "main", "/path/report.md", Some("https://github.com/pr/1"));
        assert_eq!(vars["status"], "Ready");
        assert_eq!(vars["branch"], "main");
        assert_eq!(vars["issues_count"], "0");
        assert_eq!(vars["pr_url"], "https://github.com/pr/1");
    }

    #[test]
    fn test_expand_template() {
        let mut vars = HashMap::new();
        vars.insert("status".to_string(), "Ready".to_string());
        vars.insert("branch".to_string(), "main".to_string());

        let result = expand_template("Review {{status}} on {{branch}}", &vars);
        assert_eq!(result, "Review Ready on main");
    }

    #[test]
    fn test_expand_template_no_match() {
        let vars = HashMap::new();
        let result = expand_template("No vars here", &vars);
        assert_eq!(result, "No vars here");
    }
}

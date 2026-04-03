use crate::types::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parse a single session JSONL file into a SessionSummary.
/// Uses last-write-wins dedup on (messageId, requestId) to fix ccusage's accuracy issue.
pub fn parse_session(path: &Path) -> Option<SessionSummary> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_create: u64 = 0;
    let mut total_cache_read: u64 = 0;
    let mut tool_usage: HashMap<String, ToolStats> = HashMap::new();
    let mut model = String::new();
    let mut project = String::new();
    let mut start_time: Option<DateTime<Utc>> = None;
    let mut end_time: Option<DateTime<Utc>> = None;
    let mut turn_count: u32 = 0;

    // Dedup: track (request_id, uuid) -> last usage seen
    let mut seen_usage: HashMap<String, Usage> = HashMap::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };

        let entry: JournalEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Track timestamps
        if let Some(ts_str) = &entry.timestamp {
            if let Ok(ts) = ts_str.parse::<DateTime<Utc>>() {
                if start_time.is_none() || ts < start_time.unwrap() {
                    start_time = Some(ts);
                }
                if end_time.is_none() || ts > end_time.unwrap() {
                    end_time = Some(ts);
                }
            }
        }

        // Track project
        if let Some(cwd) = &entry.cwd {
            if project.is_empty() {
                project = simplify_path(cwd);
            }
        }

        // Only process assistant messages
        if entry.entry_type.as_deref() != Some("assistant") {
            continue;
        }

        let msg = match &entry.message {
            Some(m) => m,
            None => continue,
        };

        // Track model
        if let Some(m) = &msg.model {
            if model.is_empty() {
                model = m.clone();
            }
        }

        // Dedup usage by (request_id, uuid) — last write wins
        if let Some(usage) = &msg.usage {
            if usage.output_tokens.unwrap_or(0) > 0 || usage.input_tokens.unwrap_or(0) > 0 {
                let dedup_key = format!(
                    "{}:{}",
                    entry.request_id.as_deref().unwrap_or(""),
                    entry.uuid.as_deref().unwrap_or("")
                );
                seen_usage.insert(dedup_key, usage.clone());
            }
        }

        // Track tool usage from content
        if let Some(contents) = &msg.content {
            for content in contents {
                if content.content_type.as_deref() == Some("tool_use") {
                    if let Some(tool_name) = &content.name {
                        let stats = tool_usage.entry(tool_name.clone()).or_default();
                        stats.call_count += 1;
                        // Estimate output tokens for this tool call
                        if let Some(input) = &content.input {
                            let input_str = input.to_string();
                            stats.estimated_output_tokens +=
                                (input_str.len() as u64) / 4; // rough: 4 chars per token
                        }
                    }
                }
            }
        }

        turn_count += 1;
    }

    // Aggregate deduped usage
    for usage in seen_usage.values() {
        total_input += usage.input_tokens.unwrap_or(0);
        total_output += usage.output_tokens.unwrap_or(0);
        total_cache_create += usage.cache_creation_input_tokens.unwrap_or(0);
        total_cache_read += usage.cache_read_input_tokens.unwrap_or(0);
    }

    if total_input == 0 && total_output == 0 {
        return None;
    }

    let cost_usd = calculate_cost(&model, total_input, total_output, total_cache_create, total_cache_read);

    Some(SessionSummary {
        session_id,
        project,
        model,
        start_time,
        end_time,
        total_input,
        total_output,
        total_cache_create,
        total_cache_read,
        cost_usd,
        tool_usage,
        turn_count,
    })
}

/// Discover all session JSONL files under ~/.claude/projects/
pub fn discover_sessions() -> Vec<std::path::PathBuf> {
    let claude_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    let mut files = vec![];
    if let Ok(entries) = std::fs::read_dir(&claude_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                    for sub in sub_entries.flatten() {
                        let path = sub.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                            files.push(path);
                        }
                    }
                }
            }
        }
    }

    files
}

fn calculate_cost(
    model: &str,
    input: u64,
    output: u64,
    cache_create: u64,
    cache_read: u64,
) -> f64 {
    // Pricing per million tokens (as of 2026-04)
    let (input_price, output_price, cache_create_price, cache_read_price) = if model.contains("opus") {
        (15.0, 75.0, 18.75, 1.5)
    } else if model.contains("sonnet") {
        (3.0, 15.0, 3.75, 0.3)
    } else if model.contains("haiku") {
        (0.8, 4.0, 1.0, 0.08)
    } else {
        (3.0, 15.0, 3.75, 0.3) // default to sonnet pricing
    };

    (input as f64 / 1_000_000.0) * input_price
        + (output as f64 / 1_000_000.0) * output_price
        + (cache_create as f64 / 1_000_000.0) * cache_create_price
        + (cache_read as f64 / 1_000_000.0) * cache_read_price
}

fn simplify_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Some(home_str) = home.to_str() {
            if path.starts_with(home_str) {
                return format!("~{}", &path[home_str.len()..]);
            }
        }
    }
    path.to_string()
}

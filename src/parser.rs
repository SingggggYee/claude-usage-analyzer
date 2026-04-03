use crate::types::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parse a single session JSONL file into a SessionSummary.
/// Uses last-write-wins dedup on (requestId, uuid) to fix ccusage's accuracy issue.
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
    let mut turns: Vec<TurnInfo> = Vec::new();

    // Dedup: track (request_id, uuid) -> (usage, turn_tools, timestamp)
    struct DedupEntry {
        usage: Usage,
        tools: Vec<String>,
        timestamp: Option<DateTime<Utc>>,
    }
    let mut seen: HashMap<String, DedupEntry> = HashMap::new();

    // Track current request's tools
    let mut current_request_tools: Vec<String> = Vec::new();
    let mut current_request_id: Option<String> = None;

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
        if let Some(ts_str) = &entry.timestamp
            && let Ok(ts) = ts_str.parse::<DateTime<Utc>>() {
                if start_time.is_none() || ts < start_time.unwrap() {
                    start_time = Some(ts);
                }
                if end_time.is_none() || ts > end_time.unwrap() {
                    end_time = Some(ts);
                }
            }

        if let Some(cwd) = &entry.cwd
            && project.is_empty() {
                project = simplify_path(cwd);
            }

        if entry.entry_type.as_deref() != Some("assistant") {
            continue;
        }

        let msg = match &entry.message {
            Some(m) => m,
            None => continue,
        };

        if let Some(m) = &msg.model
            && model.is_empty() {
                model = m.clone();
            }

        // Track tools per request
        let req_id = entry.request_id.clone().unwrap_or_default();
        if current_request_id.as_deref() != Some(&req_id) {
            // New request — flush previous tools
            if !current_request_tools.is_empty() {
                for tool in &current_request_tools {
                    let stats = tool_usage.entry(tool.clone()).or_default();
                    stats.call_count += 1;
                }
            }
            current_request_tools.clear();
            current_request_id = Some(req_id.clone());
        }

        // Collect tool names from content
        if let Some(contents) = &msg.content {
            for content in contents {
                if content.content_type.as_deref() == Some("tool_use")
                    && let Some(tool_name) = &content.name {
                        current_request_tools.push(tool_name.clone());
                        // Estimate output tokens
                        if let Some(input) = &content.input {
                            let stats = tool_usage.entry(tool_name.clone()).or_default();
                            stats.estimated_output_tokens += (input.to_string().len() as u64) / 4;
                        }
                    }
            }
        }

        // Dedup usage by (request_id, uuid) — last write wins
        if let Some(usage) = &msg.usage
            && (usage.output_tokens.unwrap_or(0) > 0 || usage.input_tokens.unwrap_or(0) > 0) {
                let dedup_key = format!(
                    "{}:{}",
                    entry.request_id.as_deref().unwrap_or(""),
                    entry.uuid.as_deref().unwrap_or("")
                );
                let ts = entry
                    .timestamp
                    .as_deref()
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok());
                seen.insert(
                    dedup_key,
                    DedupEntry {
                        usage: usage.clone(),
                        tools: current_request_tools.clone(),
                        timestamp: ts,
                    },
                );
            }

        turn_count += 1;
    }

    // Flush last request's tools
    for tool in &current_request_tools {
        let stats = tool_usage.entry(tool.clone()).or_default();
        stats.call_count += 1;
    }

    // Aggregate deduped usage and build turns
    let mut sorted_entries: Vec<_> = seen.into_iter().collect();
    sorted_entries.sort_by_key(|(_, e)| e.timestamp);

    for (i, (_, entry)) in sorted_entries.into_iter().enumerate() {
        let u = &entry.usage;
        let inp = u.input_tokens.unwrap_or(0);
        let out = u.output_tokens.unwrap_or(0);
        let cc = u.cache_creation_input_tokens.unwrap_or(0);
        let cr = u.cache_read_input_tokens.unwrap_or(0);

        total_input += inp;
        total_output += out;
        total_cache_create += cc;
        total_cache_read += cr;

        turns.push(TurnInfo {
            turn_number: (i + 1) as u32,
            timestamp: entry.timestamp,
            input_tokens: inp,
            output_tokens: out,
            cache_create: cc,
            cache_read: cr,
            tools_used: entry.tools,
        });
    }

    if total_input == 0 && total_output == 0 {
        return None;
    }

    let cost_usd = calculate_cost(
        &model,
        total_input,
        total_output,
        total_cache_create,
        total_cache_read,
    );

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
        turns,
    })
}

pub fn discover_sessions() -> Vec<std::path::PathBuf> {
    let claude_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    let mut files = vec![];
    if let Ok(entries) = std::fs::read_dir(&claude_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                    for sub in sub_entries.flatten() {
                        let path = sub.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                            files.push(path);
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
    let (input_price, output_price, cache_create_price, cache_read_price) =
        if model.contains("opus") {
            (15.0, 75.0, 18.75, 1.5)
        } else if model.contains("sonnet") {
            (3.0, 15.0, 3.75, 0.3)
        } else if model.contains("haiku") {
            (0.8, 4.0, 1.0, 0.08)
        } else {
            (3.0, 15.0, 3.75, 0.3)
        };

    (input as f64 / 1_000_000.0) * input_price
        + (output as f64 / 1_000_000.0) * output_price
        + (cache_create as f64 / 1_000_000.0) * cache_create_price
        + (cache_read as f64 / 1_000_000.0) * cache_read_price
}

fn simplify_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir()
        && let Some(home_str) = home.to_str()
            && path.starts_with(home_str) {
                return format!("~{}", &path[home_str.len()..]);
            }
    path.to_string()
}

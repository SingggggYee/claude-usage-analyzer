use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct JournalEntry {
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub message: Option<Message>,
    pub timestamp: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    pub uuid: Option<String>,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Message {
    pub model: Option<String>,
    pub role: Option<String>,
    pub content: Option<Vec<Content>>,
    pub usage: Option<Usage>,
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Content {
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    pub text: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

// Analyzed data structures

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub project: String,
    pub model: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_input: u64,
    pub total_output: u64,
    pub total_cache_create: u64,
    pub total_cache_read: u64,
    pub cost_usd: f64,
    pub tool_usage: HashMap<String, ToolStats>,
    pub turn_count: u32,
    pub turns: Vec<TurnInfo>,
}

impl SessionSummary {
    pub fn total_tokens(&self) -> u64 {
        self.total_input + self.total_output + self.total_cache_create + self.total_cache_read
    }

    /// Tokens the user can actually control (output + input, excluding cache reads)
    pub fn controllable_tokens(&self) -> u64 {
        self.total_input + self.total_output + self.total_cache_create
    }

    pub fn duration_secs(&self) -> Option<i64> {
        match (&self.start_time, &self.end_time) {
            (Some(s), Some(e)) => Some((*e - *s).num_seconds()),
            _ => None,
        }
    }

    /// Tokens per minute burn rate
    pub fn burn_rate(&self) -> Option<f64> {
        self.duration_secs().and_then(|d| {
            if d > 0 {
                Some(self.total_tokens() as f64 / (d as f64 / 60.0))
            } else {
                None
            }
        })
    }

    pub fn is_peak_session(&self) -> bool {
        self.start_time.map(|t| is_peak_hour(&t)).unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TurnInfo {
    pub turn_number: u32,
    pub timestamp: Option<DateTime<Utc>>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_create: u64,
    pub cache_read: u64,
    pub tools_used: Vec<String>,
}

impl TurnInfo {
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_create + self.cache_read
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ToolStats {
    pub call_count: u32,
    pub estimated_output_tokens: u64,
}

#[derive(Debug, Serialize)]
pub struct OverallReport {
    pub sessions: Vec<SessionSummary>,
    pub total_tokens: u64,
    pub total_controllable: u64,
    pub total_cost: f64,
    pub by_project: HashMap<String, ProjectStats>,
    pub by_tool: HashMap<String, ToolStats>,
    pub by_model: HashMap<String, u64>,
    pub by_day: Vec<(String, u64, f64)>,
    pub controllable_sinks: Vec<TokenSink>,
    pub fixed_overhead: Vec<TokenSink>,
    pub suggestions: Vec<String>,
    pub anomaly_sessions: Vec<AnomalySession>,
    pub peak_vs_offpeak: Option<PeakComparison>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStats {
    pub path: String,
    pub tokens: u64,
    pub cost: f64,
    pub session_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenSink {
    pub description: String,
    pub tokens: u64,
    pub percentage: f64,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnomalySession {
    pub session_id: String,
    pub project: String,
    pub burn_rate: f64,
    pub avg_burn_rate: f64,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeakComparison {
    pub peak_sessions: u32,
    pub offpeak_sessions: u32,
    pub peak_avg_tokens: u64,
    pub offpeak_avg_tokens: u64,
}

/// Check if a timestamp falls in peak hours (Mon-Fri 05:00-11:00 PT)
pub fn is_peak_hour(time: &DateTime<Utc>) -> bool {
    use chrono::{Datelike, Timelike};
    // Convert UTC to PT (UTC-7 for PDT)
    let pt_hour = (time.hour() as i32 - 7).rem_euclid(24) as u32;
    let weekday = time.weekday().num_days_from_monday(); // 0=Mon, 6=Sun
    weekday < 5 && (5..11).contains(&pt_hour)
}

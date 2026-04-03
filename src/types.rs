use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
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
pub struct Message {
    pub model: Option<String>,
    pub role: Option<String>,
    pub content: Option<Vec<Content>>,
    pub usage: Option<Usage>,
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
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

impl Usage {
    pub fn total(&self) -> u64 {
        self.input_tokens.unwrap_or(0)
            + self.output_tokens.unwrap_or(0)
            + self.cache_creation_input_tokens.unwrap_or(0)
            + self.cache_read_input_tokens.unwrap_or(0)
    }

    pub fn output(&self) -> u64 {
        self.output_tokens.unwrap_or(0)
    }

    pub fn input_total(&self) -> u64 {
        self.input_tokens.unwrap_or(0)
            + self.cache_creation_input_tokens.unwrap_or(0)
            + self.cache_read_input_tokens.unwrap_or(0)
    }
}

// Analyzed data structures

#[derive(Debug, Clone)]
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
}

impl SessionSummary {
    pub fn total_tokens(&self) -> u64 {
        self.total_input + self.total_output + self.total_cache_create + self.total_cache_read
    }

    pub fn duration_secs(&self) -> Option<i64> {
        match (&self.start_time, &self.end_time) {
            (Some(s), Some(e)) => Some((*e - *s).num_seconds()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToolStats {
    pub call_count: u32,
    pub estimated_output_tokens: u64,
}

#[derive(Debug)]
pub struct OverallReport {
    pub sessions: Vec<SessionSummary>,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub by_project: HashMap<String, ProjectStats>,
    pub by_tool: HashMap<String, ToolStats>,
    pub by_model: HashMap<String, u64>,
    pub by_day: Vec<(String, u64, f64)>, // date, tokens, cost
    pub top_sinks: Vec<TokenSink>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectStats {
    pub path: String,
    pub tokens: u64,
    pub cost: f64,
    pub session_count: u32,
}

#[derive(Debug, Clone)]
pub struct TokenSink {
    pub description: String,
    pub tokens: u64,
    pub percentage: f64,
    pub suggestion: Option<String>,
}

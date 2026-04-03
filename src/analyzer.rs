use crate::types::*;
use std::collections::HashMap;

pub fn analyze(sessions: Vec<SessionSummary>) -> OverallReport {
    let mut by_project: HashMap<String, ProjectStats> = HashMap::new();
    let mut by_tool: HashMap<String, ToolStats> = HashMap::new();
    let mut by_model: HashMap<String, u64> = HashMap::new();
    let mut by_day_map: HashMap<String, (u64, f64)> = HashMap::new();
    let mut total_tokens: u64 = 0;
    let mut total_cost: f64 = 0.0;

    for session in &sessions {
        let tokens = session.total_tokens();
        total_tokens += tokens;
        total_cost += session.cost_usd;

        // By project
        let proj = by_project
            .entry(session.project.clone())
            .or_insert_with(|| ProjectStats {
                path: session.project.clone(),
                tokens: 0,
                cost: 0.0,
                session_count: 0,
            });
        proj.tokens += tokens;
        proj.cost += session.cost_usd;
        proj.session_count += 1;

        // By tool
        for (tool_name, stats) in &session.tool_usage {
            let t = by_tool.entry(tool_name.clone()).or_default();
            t.call_count += stats.call_count;
            t.estimated_output_tokens += stats.estimated_output_tokens;
        }

        // By model
        *by_model.entry(session.model.clone()).or_default() += tokens;

        // By day
        if let Some(start) = session.start_time {
            let day = start.format("%Y-%m-%d").to_string();
            let entry = by_day_map.entry(day).or_insert((0, 0.0));
            entry.0 += tokens;
            entry.1 += session.cost_usd;
        }
    }

    // Sort by_day
    let mut by_day: Vec<(String, u64, f64)> = by_day_map
        .into_iter()
        .map(|(d, (t, c))| (d, t, c))
        .collect();
    by_day.sort_by(|a, b| a.0.cmp(&b.0));

    // Top token sinks
    let top_sinks = identify_sinks(&sessions, &by_tool, total_tokens);

    // Suggestions
    let suggestions = generate_suggestions(&sessions, &by_tool, total_tokens);

    OverallReport {
        sessions,
        total_tokens,
        total_cost,
        by_project,
        by_tool,
        by_model,
        by_day,
        top_sinks,
        suggestions,
    }
}

fn identify_sinks(
    sessions: &[SessionSummary],
    by_tool: &HashMap<String, ToolStats>,
    total_tokens: u64,
) -> Vec<TokenSink> {
    let mut sinks = vec![];

    if total_tokens == 0 {
        return sinks;
    }

    // Cache creation as a sink
    let total_cache_create: u64 = sessions.iter().map(|s| s.total_cache_create).sum();
    if total_cache_create > 0 {
        let pct = (total_cache_create as f64 / total_tokens as f64) * 100.0;
        sinks.push(TokenSink {
            description: "Cache creation (CLAUDE.md, context, system prompt)".to_string(),
            tokens: total_cache_create,
            percentage: pct,
            suggestion: if pct > 30.0 {
                Some("Large CLAUDE.md or MCP context. Consider trimming unused rules.".to_string())
            } else {
                None
            },
        });
    }

    // Cache reads
    let total_cache_read: u64 = sessions.iter().map(|s| s.total_cache_read).sum();
    if total_cache_read > 0 {
        let pct = (total_cache_read as f64 / total_tokens as f64) * 100.0;
        sinks.push(TokenSink {
            description: "Cache reads (context re-reading across turns)".to_string(),
            tokens: total_cache_read,
            percentage: pct,
            suggestion: if pct > 80.0 {
                Some("High cache reads indicate long sessions. Use /compact more often.".to_string())
            } else {
                None
            },
        });
    }

    // Output tokens
    let total_output: u64 = sessions.iter().map(|s| s.total_output).sum();
    if total_output > 0 {
        let pct = (total_output as f64 / total_tokens as f64) * 100.0;
        sinks.push(TokenSink {
            description: "Model output (Claude's responses)".to_string(),
            tokens: total_output,
            percentage: pct,
            suggestion: if pct > 5.0 {
                Some(
                    "High output ratio. Consider adding 'be concise' to CLAUDE.md.".to_string(),
                )
            } else {
                None
            },
        });
    }

    // Tool-specific sinks
    let tool_total_calls: u32 = by_tool.values().map(|s| s.call_count).sum();
    for (tool, stats) in by_tool {
        if tool_total_calls > 0 {
            let call_pct = (stats.call_count as f64 / tool_total_calls as f64) * 100.0;
            if call_pct > 20.0 {
                let suggestion = match tool.as_str() {
                    "Read" => Some("Heavy file reading. Use offset/limit params to read only needed sections.".to_string()),
                    "Bash" => Some("Many shell commands. Long outputs consume tokens. Consider piping to head/tail.".to_string()),
                    "Grep" | "Glob" => None, // These are usually fine
                    "Write" => Some("Many full file writes. Prefer Edit for partial changes.".to_string()),
                    "Agent" => Some("Subagent spawning. Each subagent duplicates context. Minimize agent depth.".to_string()),
                    _ => None,
                };
                sinks.push(TokenSink {
                    description: format!("Tool: {} ({} calls, {:.0}% of all tool calls)", tool, stats.call_count, call_pct),
                    tokens: stats.estimated_output_tokens,
                    percentage: call_pct,
                    suggestion,
                });
            }
        }
    }

    sinks.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    sinks
}

fn generate_suggestions(
    sessions: &[SessionSummary],
    by_tool: &HashMap<String, ToolStats>,
    total_tokens: u64,
) -> Vec<String> {
    let mut suggestions = vec![];

    if total_tokens == 0 {
        return suggestions;
    }

    // Long sessions
    let long_sessions: Vec<_> = sessions
        .iter()
        .filter(|s| s.turn_count > 30)
        .collect();
    if !long_sessions.is_empty() {
        suggestions.push(format!(
            "{} session(s) exceeded 30 turns. Use /compact to reduce context buildup.",
            long_sessions.len()
        ));
    }

    // High cache creation ratio
    let total_cache_create: u64 = sessions.iter().map(|s| s.total_cache_create).sum();
    let cache_pct = (total_cache_create as f64 / total_tokens as f64) * 100.0;
    if cache_pct > 40.0 {
        suggestions.push(
            "Cache creation is >40% of total tokens. Your CLAUDE.md or MCP context may be too large. Trim unused sections.".to_string()
        );
    }

    // Read tool dominance
    if let Some(read_stats) = by_tool.get("Read") {
        let total_calls: u32 = by_tool.values().map(|s| s.call_count).sum();
        if total_calls > 0 && (read_stats.call_count as f64 / total_calls as f64) > 0.4 {
            suggestions.push(
                "Read tool accounts for >40% of tool calls. Use Grep to find specific content instead of reading entire files.".to_string()
            );
        }
    }

    // Subagent usage
    if let Some(agent_stats) = by_tool.get("Agent") {
        if agent_stats.call_count > 5 {
            suggestions.push(format!(
                "{} subagent calls detected. Each subagent duplicates the full context. Consider using Grep/Glob directly for simple searches.",
                agent_stats.call_count
            ));
        }
    }

    // Write vs Edit ratio
    let writes = by_tool.get("Write").map(|s| s.call_count).unwrap_or(0);
    let edits = by_tool.get("Edit").map(|s| s.call_count).unwrap_or(0);
    if writes > 0 && edits > 0 && writes > edits * 2 {
        suggestions.push(
            "Write calls outnumber Edit calls 2:1. Edit sends only the diff and uses fewer tokens.".to_string()
        );
    }

    suggestions
}

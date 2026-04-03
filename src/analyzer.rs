use crate::types::*;
use std::collections::HashMap;

pub fn analyze(sessions: Vec<SessionSummary>) -> OverallReport {
    let mut by_project: HashMap<String, ProjectStats> = HashMap::new();
    let mut by_tool: HashMap<String, ToolStats> = HashMap::new();
    let mut by_model: HashMap<String, u64> = HashMap::new();
    let mut by_day_map: HashMap<String, (u64, f64)> = HashMap::new();
    let mut total_tokens: u64 = 0;
    let mut total_controllable: u64 = 0;
    let mut total_cost: f64 = 0.0;

    for session in &sessions {
        let tokens = session.total_tokens();
        total_tokens += tokens;
        total_controllable += session.controllable_tokens();
        total_cost += session.cost_usd;

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

        for (tool_name, stats) in &session.tool_usage {
            let t = by_tool.entry(tool_name.clone()).or_default();
            t.call_count += stats.call_count;
            t.estimated_output_tokens += stats.estimated_output_tokens;
        }

        *by_model.entry(session.model.clone()).or_default() += tokens;

        if let Some(start) = session.start_time {
            let day = start.format("%Y-%m-%d").to_string();
            let entry = by_day_map.entry(day).or_insert((0, 0.0));
            entry.0 += tokens;
            entry.1 += session.cost_usd;
        }
    }

    let mut by_day: Vec<(String, u64, f64)> = by_day_map
        .into_iter()
        .map(|(d, (t, c))| (d, t, c))
        .collect();
    by_day.sort_by(|a, b| a.0.cmp(&b.0));

    let controllable_sinks = identify_controllable_sinks(&sessions, &by_tool, total_controllable);
    let fixed_overhead = identify_fixed_overhead(&sessions, total_tokens);
    let suggestions = generate_suggestions(&sessions, &by_tool);
    let anomaly_sessions = detect_anomalies(&sessions);
    let peak_vs_offpeak = compare_peak_offpeak(&sessions);

    OverallReport {
        sessions,
        total_tokens,
        total_controllable,
        total_cost,
        by_project,
        by_tool,
        by_model,
        by_day,
        controllable_sinks,
        fixed_overhead,
        suggestions,
        anomaly_sessions,
        peak_vs_offpeak,
    }
}

fn identify_controllable_sinks(
    sessions: &[SessionSummary],
    by_tool: &HashMap<String, ToolStats>,
    total_controllable: u64,
) -> Vec<TokenSink> {
    let mut sinks = vec![];
    if total_controllable == 0 {
        return sinks;
    }

    // Output tokens
    let total_output: u64 = sessions.iter().map(|s| s.total_output).sum();
    if total_output > 0 {
        let pct = (total_output as f64 / total_controllable as f64) * 100.0;
        sinks.push(TokenSink {
            description: "Model output (Claude's responses)".to_string(),
            tokens: total_output,
            percentage: pct,
            suggestion: if pct > 60.0 {
                Some("High output ratio. Add 'be concise, no preamble' to CLAUDE.md.".to_string())
            } else {
                None
            },
        });
    }

    // Cache creation (context/CLAUDE.md/MCP — partially controllable)
    let total_cache_create: u64 = sessions.iter().map(|s| s.total_cache_create).sum();
    if total_cache_create > 0 {
        let pct = (total_cache_create as f64 / total_controllable as f64) * 100.0;
        sinks.push(TokenSink {
            description: "Cache creation (CLAUDE.md, MCP, system prompt)".to_string(),
            tokens: total_cache_create,
            percentage: pct,
            suggestion: if pct > 50.0 {
                Some(
                    "Large initial context. Trim unused CLAUDE.md rules or MCP tools.".to_string(),
                )
            } else {
                None
            },
        });
    }

    // Input tokens (direct prompt input)
    let total_input: u64 = sessions.iter().map(|s| s.total_input).sum();
    if total_input > 0 {
        let pct = (total_input as f64 / total_controllable as f64) * 100.0;
        sinks.push(TokenSink {
            description: "Direct input (prompts, tool results fed back)".to_string(),
            tokens: total_input,
            percentage: pct,
            suggestion: None,
        });
    }

    // Tool-specific sinks
    let tool_total_calls: u32 = by_tool.values().map(|s| s.call_count).sum();
    for (tool, stats) in by_tool {
        if tool_total_calls > 0 {
            let call_pct = (stats.call_count as f64 / tool_total_calls as f64) * 100.0;
            if call_pct > 15.0 {
                let suggestion = match tool.as_str() {
                    "Read" => Some(
                        "Heavy file reading. Use offset/limit to read only needed sections."
                            .to_string(),
                    ),
                    "Bash" => Some(
                        "Many shell commands. Long outputs eat tokens. Pipe to head/tail."
                            .to_string(),
                    ),
                    "Write" => Some(
                        "Many full file writes. Prefer Edit (sends only the diff).".to_string(),
                    ),
                    "Agent" => Some(
                        "Subagents duplicate the full context. Use Grep/Glob directly for simple searches."
                            .to_string(),
                    ),
                    _ => None,
                };
                if suggestion.is_some() {
                    sinks.push(TokenSink {
                        description: format!(
                            "Tool: {} ({} calls, {:.0}% of tool calls)",
                            tool, stats.call_count, call_pct
                        ),
                        tokens: stats.estimated_output_tokens,
                        percentage: call_pct,
                        suggestion,
                    });
                }
            }
        }
    }

    sinks.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    sinks
}

fn identify_fixed_overhead(sessions: &[SessionSummary], total_tokens: u64) -> Vec<TokenSink> {
    let mut overhead = vec![];
    if total_tokens == 0 {
        return overhead;
    }

    let total_cache_read: u64 = sessions.iter().map(|s| s.total_cache_read).sum();
    if total_cache_read > 0 {
        let pct = (total_cache_read as f64 / total_tokens as f64) * 100.0;
        overhead.push(TokenSink {
            description: "Cache reads (context re-read every turn — normal, cheap at $1.5/M)"
                .to_string(),
            tokens: total_cache_read,
            percentage: pct,
            suggestion: if pct > 95.0 {
                Some("Dominates token count but costs little. Focus on controllable sinks above.".to_string())
            } else {
                None
            },
        });
    }

    overhead
}

fn detect_anomalies(sessions: &[SessionSummary]) -> Vec<AnomalySession> {
    let burn_rates: Vec<f64> = sessions.iter().filter_map(|s| s.burn_rate()).collect();

    if burn_rates.is_empty() {
        return vec![];
    }

    let avg_burn = burn_rates.iter().sum::<f64>() / burn_rates.len() as f64;

    sessions
        .iter()
        .filter_map(|s| {
            let rate = s.burn_rate()?;
            let ratio = rate / avg_burn;
            if ratio > 2.0 {
                Some(AnomalySession {
                    session_id: s.session_id.clone(),
                    project: s.project.clone(),
                    burn_rate: rate,
                    avg_burn_rate: avg_burn,
                    ratio,
                })
            } else {
                None
            }
        })
        .collect()
}

fn compare_peak_offpeak(sessions: &[SessionSummary]) -> Option<PeakComparison> {
    let peak: Vec<_> = sessions.iter().filter(|s| s.is_peak_session()).collect();
    let offpeak: Vec<_> = sessions.iter().filter(|s| !s.is_peak_session()).collect();

    if peak.is_empty() || offpeak.is_empty() {
        return None;
    }

    let peak_avg = peak.iter().map(|s| s.total_tokens()).sum::<u64>() / peak.len() as u64;
    let offpeak_avg =
        offpeak.iter().map(|s| s.total_tokens()).sum::<u64>() / offpeak.len() as u64;

    Some(PeakComparison {
        peak_sessions: peak.len() as u32,
        offpeak_sessions: offpeak.len() as u32,
        peak_avg_tokens: peak_avg,
        offpeak_avg_tokens: offpeak_avg,
    })
}

fn generate_suggestions(
    sessions: &[SessionSummary],
    by_tool: &HashMap<String, ToolStats>,
) -> Vec<String> {
    let mut suggestions = vec![];

    let long_sessions: Vec<_> = sessions.iter().filter(|s| s.turn_count > 30).collect();
    if !long_sessions.is_empty() {
        suggestions.push(format!(
            "{} session(s) exceeded 30 turns. Use /compact to reduce context buildup.",
            long_sessions.len()
        ));
    }

    if let Some(read_stats) = by_tool.get("Read") {
        let total_calls: u32 = by_tool.values().map(|s| s.call_count).sum();
        if total_calls > 0 && (read_stats.call_count as f64 / total_calls as f64) > 0.35 {
            suggestions.push(
                "Read tool is >35% of tool calls. Use Grep to find specific content instead of reading entire files."
                    .to_string(),
            );
        }
    }

    if let Some(agent_stats) = by_tool.get("Agent") {
        if agent_stats.call_count > 5 {
            suggestions.push(format!(
                "{} subagent calls. Each duplicates full context. Use Grep/Glob directly for simple searches.",
                agent_stats.call_count
            ));
        }
    }

    let writes = by_tool.get("Write").map(|s| s.call_count).unwrap_or(0);
    let edits = by_tool.get("Edit").map(|s| s.call_count).unwrap_or(0);
    if writes > 0 && edits > 0 && writes > edits * 2 {
        suggestions.push(
            "Write calls outnumber Edit 2:1. Edit sends only the diff and uses fewer tokens."
                .to_string(),
        );
    }

    // Peak hours warning
    let peak_sessions = sessions.iter().filter(|s| s.is_peak_session()).count();
    let total = sessions.len();
    if total > 0 && (peak_sessions as f64 / total as f64) > 0.5 {
        suggestions.push(format!(
            "{}% of sessions during peak hours (Mon-Fri 5-11am PT). Token burn rate is ~2x during peak.",
            (peak_sessions * 100) / total
        ));
    }

    suggestions
}

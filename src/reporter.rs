use crate::types::*;
use colored::Colorize;

pub fn print_report(report: &OverallReport) {
    println!();
    println!("{}", "  claude-usage-analyzer — Claude Code Usage Debugger".bold());
    println!(
        "{}",
        "  Why did your tokens burn? What to do about it.".dimmed()
    );
    println!();

    // Overview
    println!("{}", "  Overview".bold().underline());
    println!(
        "  Sessions: {}  |  Total tokens: {}  |  Equivalent cost: {}",
        format!("{}", report.sessions.len()).cyan(),
        format_tokens(report.total_tokens).cyan(),
        format!("${:.2}", report.total_cost).yellow(),
    );
    println!(
        "  {}",
        "(Cost shown is API-equivalent. Max subscribers pay a flat rate.)".dimmed()
    );
    println!();

    // Controllable sinks (what you can actually reduce)
    if !report.controllable_sinks.is_empty() {
        println!(
            "{}",
            "  Controllable Token Sinks (what you can reduce)".bold().underline()
        );
        println!(
            "  {}",
            format!(
                "Total controllable: {} ({:.1}% of all tokens)",
                format_tokens(report.total_controllable),
                if report.total_tokens > 0 {
                    (report.total_controllable as f64 / report.total_tokens as f64) * 100.0
                } else {
                    0.0
                }
            )
            .dimmed()
        );
        println!();
        for sink in &report.controllable_sinks {
            let bar = make_bar(sink.percentage, 20);
            println!(
                "  {} {:5.1}%  {:>8}  {}",
                bar,
                sink.percentage,
                format_tokens(sink.tokens).dimmed(),
                sink.description
            );
            if let Some(suggestion) = &sink.suggestion {
                println!("  {}  → {}", " ".repeat(28), suggestion.yellow());
            }
        }
        println!();
    }

    // Fixed overhead
    if !report.fixed_overhead.is_empty() {
        println!(
            "{}",
            "  Fixed Overhead (normal, not actionable)".bold().underline()
        );
        for sink in &report.fixed_overhead {
            println!(
                "  {:5.1}%  {:>8}  {}",
                sink.percentage,
                format_tokens(sink.tokens).dimmed(),
                sink.description
            );
            if let Some(suggestion) = &sink.suggestion {
                println!("  {}  {}", " ".repeat(16), suggestion.dimmed());
            }
        }
        println!();
    }

    // Anomaly sessions
    if !report.anomaly_sessions.is_empty() {
        println!(
            "{}",
            format!(
                "  Anomaly Sessions ({} sessions burning >2x average rate)",
                report.anomaly_sessions.len()
            )
            .bold()
            .underline()
        );
        for a in report.anomaly_sessions.iter().take(5) {
            println!(
                "  {} {:.0} tok/min ({:.1}x avg)  {}",
                "⚡".red(),
                a.burn_rate,
                a.ratio,
                a.project.dimmed()
            );
        }
        println!();
    }

    // Peak vs off-peak
    if let Some(peak) = &report.peak_vs_offpeak {
        println!("{}", "  Peak vs Off-Peak Hours".bold().underline());
        println!(
            "  Peak (Mon-Fri 5-11am PT):  {} sessions, avg {} tokens/session",
            peak.peak_sessions,
            format_tokens(peak.peak_avg_tokens)
        );
        println!(
            "  Off-peak:                  {} sessions, avg {} tokens/session",
            peak.offpeak_sessions,
            format_tokens(peak.offpeak_avg_tokens)
        );
        if peak.peak_avg_tokens > peak.offpeak_avg_tokens {
            let ratio = peak.peak_avg_tokens as f64 / peak.offpeak_avg_tokens.max(1) as f64;
            println!(
                "  {}",
                format!("Peak sessions use {:.1}x more tokens on average.", ratio).yellow()
            );
        }
        println!();
    }

    // By Project (top 10)
    println!("{}", "  By Project (top 10)".bold().underline());
    let mut projects: Vec<_> = report.by_project.values().collect();
    projects.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));
    for proj in projects.iter().take(10) {
        let pct = if report.total_tokens > 0 {
            (proj.tokens as f64 / report.total_tokens as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "  {:5.1}%  ${:>8.2}  {:>3} sessions  {}",
            pct,
            proj.cost,
            proj.session_count,
            proj.path.dimmed()
        );
    }
    println!();

    // By Tool
    if !report.by_tool.is_empty() {
        println!("{}", "  By Tool".bold().underline());
        let mut tools: Vec<_> = report.by_tool.iter().collect();
        tools.sort_by(|a, b| b.1.call_count.cmp(&a.1.call_count));
        for (name, stats) in tools.iter().take(10) {
            println!(
                "  {:>6} calls  {}",
                stats.call_count.to_string().cyan(),
                name
            );
        }
        println!();
    }

    // By Model
    if !report.by_model.is_empty() {
        println!("{}", "  By Model".bold().underline());
        let mut models: Vec<_> = report.by_model.iter().collect();
        models.sort_by(|a, b| b.1.cmp(a.1));
        for (model, tokens) in &models {
            let t = **tokens;
            let pct = if report.total_tokens > 0 {
                (t as f64 / report.total_tokens as f64) * 100.0
            } else {
                0.0
            };
            println!("  {:5.1}%  {:>10}  {}", pct, format_tokens(t), model);
        }
        println!();
    }

    // Daily trend (last 7 days)
    if report.by_day.len() > 1 {
        println!("{}", "  Daily Trend (recent)".bold().underline());
        let days_to_show = report.by_day.len().min(7);
        let start = report.by_day.len() - days_to_show;
        let max_tokens = report.by_day[start..]
            .iter()
            .map(|(_, t, _)| *t)
            .max()
            .unwrap_or(1);

        for (day, tokens, cost) in &report.by_day[start..] {
            let bar_len = ((*tokens as f64 / max_tokens as f64) * 20.0) as usize;
            let bar = "█".repeat(bar_len);
            println!(
                "  {}  {:>10}  ${:>8.2}  {}",
                day,
                format_tokens(*tokens),
                cost,
                bar.green()
            );
        }
        println!();
    }

    // Suggestions
    if !report.suggestions.is_empty() {
        println!("{}", "  Suggestions".bold().underline());
        for (i, suggestion) in report.suggestions.iter().enumerate() {
            println!("  {}. {}", i + 1, suggestion.yellow());
        }
        println!();
    }
}

pub fn print_session_detail(session: &SessionSummary) {
    println!();
    println!("{}", format!("  Session: {}", session.session_id).bold());
    println!("  Project:  {}", session.project);
    println!("  Model:    {}", session.model);
    println!("  Turns:    {}", session.turn_count);

    if let Some(dur) = session.duration_secs() {
        let mins = dur / 60;
        let secs = dur % 60;
        println!("  Duration: {}m {}s", mins, secs);
    }

    if let Some(rate) = session.burn_rate() {
        println!("  Burn rate: {:.0} tokens/min", rate);
    }

    if session.is_peak_session() {
        println!("  {}", "⚠ Started during peak hours (Mon-Fri 5-11am PT)".yellow());
    }

    println!();
    println!("  {}", "Token Breakdown".bold());
    println!("    Input:          {:>10}", format_tokens(session.total_input));
    println!("    Output:         {:>10}", format_tokens(session.total_output));
    println!("    Cache create:   {:>10}", format_tokens(session.total_cache_create));
    println!("    Cache read:     {:>10}  {}", format_tokens(session.total_cache_read), "(fixed overhead)".dimmed());
    println!("    {}", "─".repeat(30));
    println!(
        "    Controllable:   {:>10}",
        format_tokens(session.controllable_tokens()).bold()
    );
    println!(
        "    Total:          {:>10}",
        format_tokens(session.total_tokens())
    );
    println!("    Cost:           ${:.4}", session.cost_usd);

    // Tools
    if !session.tool_usage.is_empty() {
        println!();
        println!("  {}", "Tools Used".bold());
        let mut tools: Vec<_> = session.tool_usage.iter().collect();
        tools.sort_by(|a, b| b.1.call_count.cmp(&a.1.call_count));
        for (name, stats) in &tools {
            println!("    {:>4} calls  {}", stats.call_count, name);
        }
    }

    // Per-turn breakdown
    if !session.turns.is_empty() {
        println!();
        println!("  {}", "Per-Turn Token Consumption".bold());
        let header = format!(
            "  {:>5}  {:>8}  {:>8}  {:>8}  {:>8}  {}",
            "Turn", "Input", "Output", "Cache↑", "Cache↓", "Tools"
        );
        println!("{header}");
        println!("  {}", "─".repeat(70));

        let max_turn_total = session.turns.iter().map(|t| t.total()).max().unwrap_or(1);

        for turn in &session.turns {
            let total = turn.total();
            let bar_len = ((total as f64 / max_turn_total as f64) * 15.0) as usize;
            let bar = "█".repeat(bar_len);

            let tools_str = if turn.tools_used.is_empty() {
                String::new()
            } else {
                turn.tools_used.join(", ")
            };

            // Highlight turns that are >3x the average
            let avg = session.total_tokens() / session.turns.len().max(1) as u64;
            let is_spike = total > avg * 3;

            let line = format!(
                "  {:>5}  {:>8}  {:>8}  {:>8}  {:>8}  {}  {}",
                turn.turn_number,
                format_tokens(turn.input_tokens),
                format_tokens(turn.output_tokens),
                format_tokens(turn.cache_create),
                format_tokens(turn.cache_read),
                bar,
                tools_str
            );

            if is_spike {
                println!("{}", line.red());
            } else {
                println!("{}", line);
            }
        }
        println!();
    }
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000_000 {
        format!("{:.1}B", tokens as f64 / 1_000_000_000.0)
    } else if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        format!("{}", tokens)
    }
}

fn make_bar(percentage: f64, width: usize) -> String {
    let filled = ((percentage / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

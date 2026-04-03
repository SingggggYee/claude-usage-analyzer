use crate::types::*;
use colored::Colorize;

pub fn print_report(report: &OverallReport) {
    println!();
    println!("{}", "  ccwhy — Claude Code Usage Debugger".bold());
    println!("{}", "  Why did your tokens burn? What to do about it.".dimmed());
    println!();

    // Overview
    println!("{}", "  Overview".bold().underline());
    println!(
        "  Sessions: {}  |  Tokens: {}  |  Cost: {}",
        format!("{}", report.sessions.len()).cyan(),
        format_tokens(report.total_tokens).cyan(),
        format!("${:.2}", report.total_cost).yellow(),
    );
    println!();

    // Top Token Sinks
    if !report.top_sinks.is_empty() {
        println!("{}", "  Top Token Sinks".bold().underline());
        for sink in &report.top_sinks {
            let bar = make_bar(sink.percentage, 20);
            println!(
                "  {} {:5.1}%  {}  {}",
                bar,
                sink.percentage,
                format_tokens(sink.tokens).dimmed(),
                sink.description
            );
            if let Some(suggestion) = &sink.suggestion {
                println!("  {}  {}", " ".repeat(28), suggestion.yellow());
            }
        }
        println!();
    }

    // By Project (top 10)
    println!("{}", "  By Project (top 10)".bold().underline());
    let mut projects: Vec<_> = report.by_project.values().collect();
    projects.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    for proj in projects.iter().take(10) {
        let pct = if report.total_tokens > 0 {
            (proj.tokens as f64 / report.total_tokens as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "  {:5.1}%  {:>10}  ${:>7.2}  {} sessions  {}",
            pct,
            format_tokens(proj.tokens),
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
                "  {}  {:>10}  ${:>6.2}  {}",
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
    println!("  Project: {}", session.project);
    println!("  Model: {}", session.model);
    println!("  Turns: {}", session.turn_count);

    if let Some(dur) = session.duration_secs() {
        let mins = dur / 60;
        let secs = dur % 60;
        println!("  Duration: {}m {}s", mins, secs);
    }

    println!();
    println!("  Tokens:");
    println!("    Input:          {:>10}", format_tokens(session.total_input));
    println!(
        "    Output:         {:>10}",
        format_tokens(session.total_output)
    );
    println!(
        "    Cache create:   {:>10}",
        format_tokens(session.total_cache_create)
    );
    println!(
        "    Cache read:     {:>10}",
        format_tokens(session.total_cache_read)
    );
    println!(
        "    Total:          {:>10}",
        format_tokens(session.total_tokens()).bold()
    );
    println!("    Cost:           ${:.4}", session.cost_usd);

    if !session.tool_usage.is_empty() {
        println!();
        println!("  Tools:");
        let mut tools: Vec<_> = session.tool_usage.iter().collect();
        tools.sort_by(|a, b| b.1.call_count.cmp(&a.1.call_count));
        for (name, stats) in &tools {
            println!("    {:>4} calls  {}", stats.call_count, name);
        }
    }

    println!();
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

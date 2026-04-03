mod analyzer;
mod parser;
mod reporter;
mod types;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ccwhy",
    about = "Claude Code usage debugger. Why did your tokens burn?",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show full report: sinks, projects, tools, suggestions
    Report {
        /// Number of days to analyze (0 = all)
        #[arg(short, long, default_value = "30")]
        days: u32,
    },
    /// Show details for a specific session (by ID prefix)
    Session {
        /// Session ID or prefix
        id: String,
    },
    /// List all sessions sorted by cost
    Sessions {
        /// Number of days to analyze (0 = all)
        #[arg(short, long, default_value = "7")]
        days: u32,
        /// Show top N sessions
        #[arg(short = 'n', long, default_value = "20")]
        top: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    let paths = parser::discover_sessions();
    if paths.is_empty() {
        eprintln!("No Claude Code session data found in ~/.claude/projects/");
        eprintln!("Make sure you have used Claude Code at least once.");
        std::process::exit(1);
    }

    let mut sessions: Vec<types::SessionSummary> = paths
        .iter()
        .filter_map(|p| parser::parse_session(p))
        .collect();

    sessions.sort_by(|a, b| {
        b.cost_usd
            .partial_cmp(&a.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    match cli.command {
        None | Some(Commands::Report { .. }) => {
            let days = match &cli.command {
                Some(Commands::Report { days }) => *days,
                _ => 30,
            };

            let filtered = if days > 0 {
                let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
                sessions
                    .into_iter()
                    .filter(|s| s.start_time.map(|t| t > cutoff).unwrap_or(false))
                    .collect()
            } else {
                sessions
            };

            let report = analyzer::analyze(filtered);
            reporter::print_report(&report);
        }
        Some(Commands::Session { id }) => {
            let found = sessions.iter().find(|s| s.session_id.starts_with(&id));
            match found {
                Some(session) => reporter::print_session_detail(session),
                None => {
                    eprintln!("Session not found: {}", id);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Sessions { days, top }) => {
            let cutoff = if days > 0 {
                Some(chrono::Utc::now() - chrono::Duration::days(days as i64))
            } else {
                None
            };

            let filtered: Vec<_> = sessions
                .iter()
                .filter(|s| {
                    cutoff
                        .map(|c| s.start_time.map(|t| t > c).unwrap_or(false))
                        .unwrap_or(true)
                })
                .take(top)
                .collect();

            println!();
            println!("{}", colored::Colorize::bold("  Top Sessions by Cost"));
            println!();
            for (i, s) in filtered.iter().enumerate() {
                let dur = s
                    .duration_secs()
                    .map(|d| format!("{}m", d / 60))
                    .unwrap_or_else(|| "?".to_string());
                println!(
                    "  {:>3}. ${:>7.4}  {:>10} tokens  {:>5}  {:>3} turns  {}  {}",
                    i + 1,
                    s.cost_usd,
                    format_tokens(s.total_tokens()),
                    dur,
                    s.turn_count,
                    s.model,
                    colored::Colorize::dimmed(s.project.as_str()),
                );
            }
            println!();
        }
    }
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        format!("{}", tokens)
    }
}

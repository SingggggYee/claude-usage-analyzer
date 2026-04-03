# ccwhy

> ccusage tells you how much. ccwhy tells you **why**, and what to do about it.

A Claude Code usage debugger written in Rust. Parses your local session data, identifies where your tokens actually went, and gives you actionable suggestions to reduce waste.

## Example Output

```
  ccwhy — Claude Code Usage Debugger
  Why did your tokens burn? What to do about it.

  Overview
  Sessions: 186  |  Tokens: 3.3B  |  Cost: $6,721

  Top Token Sinks
  ███████████████████░  97.2%  3.2B   Cache reads (context re-reading across turns)
                                      → Use /compact more often.
  █░░░░░░░░░░░░░░░░░░   2.6%  86.6M  Cache creation (CLAUDE.md, context, system prompt)
  ████████░░░░░░░░░░░░  40.2%         Tool: Bash (7,066 calls, 40% of all tool calls)
                                      → Long outputs consume tokens. Pipe to head/tail.
  ████░░░░░░░░░░░░░░░░  21.3%         Tool: Read (3,744 calls, 21% of all tool calls)
                                      → Use offset/limit params to read only needed sections.

  Suggestions
  1. 86 session(s) exceeded 30 turns. Use /compact to reduce context buildup.
  2. 840 subagent calls detected. Each duplicates the full context.
```

## Install

```bash
cargo install --git https://github.com/SingggggYee/ccwhy
```

Or build from source:

```bash
git clone https://github.com/SingggggYee/ccwhy
cd ccwhy
cargo build --release
./target/release/ccwhy
```

## Usage

```bash
# Full report (last 30 days)
ccwhy

# Last 7 days
ccwhy report --days 7

# All time
ccwhy report --days 0

# Top sessions by cost
ccwhy sessions

# Session detail
ccwhy session <session-id-prefix>
```

## What It Tells You

### Top Token Sinks
Where your tokens actually go. Not just totals — broken down by:
- Cache reads vs cache creation vs output
- Tool usage (Bash, Read, Edit, Agent, etc.)
- Per-tool call counts and estimated token impact

### By Project
Which project is burning the most tokens. Sort by cost, see session counts.

### By Tool
How many times each tool was called. Highlights if Read or Bash dominates (common waste pattern).

### By Model
Token split between Opus, Sonnet, Haiku.

### Daily Trend
Visual bar chart of daily consumption.

### Actionable Suggestions
Based on your actual usage patterns:
- "86 sessions exceeded 30 turns — use /compact"
- "Read tool is 40% of calls — use Grep to find specific content"
- "840 subagent calls — each duplicates full context"
- "Write calls outnumber Edit 2:1 — Edit sends only the diff"

## How Is This Different from ccusage?

| | ccusage | ccwhy |
|---|---------|-------|
| **Question** | "How much did I spend?" | "Why did I spend it? How do I spend less?" |
| **Output** | Token counts, cost tables | Token sinks, tool attribution, optimization suggestions |
| **Accuracy** | Known dedup issues (#313, #455) | Last-write-wins dedup on (requestId, uuid) |
| **Performance** | Node.js, can timeout on large data | Rust, streaming parser |
| **Scope** | Observability | Optimization |

ccwhy is not a replacement for ccusage. Use ccusage for daily/monthly cost tracking. Use ccwhy to understand **why** and **what to change**.

## How It Works

1. Reads all `~/.claude/projects/*/*.jsonl` session files
2. Deduplicates usage entries using last-write-wins on (requestId, uuid)
3. Aggregates by project, tool, model, and day
4. Identifies top token sinks with percentage breakdown
5. Generates suggestions based on usage patterns

No network access. No API keys. Everything runs locally on your session data.

## Requirements

- Rust 1.75+ (for building)
- Claude Code session data in `~/.claude/projects/`

## License

MIT

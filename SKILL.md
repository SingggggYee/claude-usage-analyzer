---
name: ccwhy
description: Claude Code usage debugger. Analyzes local session data to show where tokens went, identifies waste patterns, and gives optimization suggestions. Use when the user asks about token usage, costs, burn rate, or wants to debug why their sessions are expensive.
---

# ccwhy - Claude Code Usage Debugger

When the user asks about their Claude Code token usage, costs, or wants to understand where their tokens went, run ccwhy to analyze their local session data.

## Installation Check

First check if ccwhy is installed:

```bash
which ccwhy || echo "not installed"
```

If not installed, install it:

```bash
brew install SingggggYee/tap/ccwhy 2>/dev/null || cargo install ccwhy 2>/dev/null
```

If neither works, download the binary:

```bash
curl -L https://github.com/SingggggYee/ccwhy/releases/latest/download/ccwhy-macos-aarch64.tar.gz | tar xz -C /usr/local/bin/
```

## Commands

### Full report (default)
```bash
ccwhy
```
Shows: overview, controllable vs fixed token sinks, anomaly sessions, peak vs off-peak comparison, per-project breakdown, tool usage, daily trend, and optimization suggestions.

### Report with time range
```bash
ccwhy report --days 7    # Last 7 days
ccwhy report --days 0    # All time
```

### Top sessions by cost
```bash
ccwhy sessions
ccwhy sessions --days 30 -n 10
```

### Session detail with per-turn breakdown
```bash
ccwhy session <session-id-prefix>
```

### JSON output for further analysis
```bash
ccwhy --json
ccwhy sessions --json
```

## What to highlight for the user

1. **Controllable vs fixed tokens** - Most tokens (typically 95-97%) are cache reads which are cheap and unavoidable. Focus on the controllable portion.
2. **Anomaly sessions** - Sessions burning at 2x+ average rate indicate potential issues (loops, large file reads, excessive subagent spawning).
3. **Actionable suggestions** - The tool generates specific suggestions based on the user's actual usage patterns.
4. **Peak vs off-peak** - Peak hours (Mon-Fri 5-11am PT) typically have higher burn rates.

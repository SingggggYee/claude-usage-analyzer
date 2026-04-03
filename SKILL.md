---
name: ccwhy
description: Analyze Claude Code token usage. Shows where tokens went, which projects cost most, and how to reduce waste. Use when user asks about token usage, costs, or burn rate.
---

# ccwhy

Debug your Claude Code token usage. Run:

```bash
ccwhy
```

If not installed:

```bash
brew install SingggggYee/tap/ccwhy
```

Shows: token sinks (controllable vs fixed), per-project costs, tool call breakdown, anomaly sessions, peak vs off-peak comparison, and optimization suggestions.

For session detail: `ccwhy session <id>`
For JSON output: `ccwhy --json`
For time range: `ccwhy report --days 7`

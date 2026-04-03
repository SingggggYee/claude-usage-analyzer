---
title: I analyzed 187 Claude Code sessions. $6,744 worth of tokens. Here's where they went.
tags: claudecode, ai, rust, devtools
---

I've been using Claude Code heavily for the past month. Building trading bots, automation tools, side projects.

I knew I was burning through tokens but never looked at the numbers.

So I built a small CLI to parse my local session data. The result: **187 sessions. 3.3 billion tokens. $6,744 equivalent API cost.**

(I'm on Max so I'm not paying per token. This is what it would've cost at standard API input/output pricing. The real number that matters for Max users is quota burn rate, not dollars.)

## 97% of my tokens were something I couldn't control

That was the first surprise. 97% were cache reads. Every turn, Claude re-reads the entire conversation context. Think of it like re-reading an entire book every time you turn a page.

The good news: cache reads are cheap ($1.5/M tokens) and completely normal. The bad news: it means the part you can actually control is tiny.

**Only 2.8% of my tokens were controllable.** Of that, 92.5% was cache creation (CLAUDE.md, MCP tools, system prompt loading), 6.6% was Claude's actual output, 0.9% was my input.

## What I wouldn't have caught from /cost

This was the most useful part:

- **86 sessions** over 30 turns without /compact, each one letting context balloon to 2-3x what it needed to be
- **840 subagent calls**, every single one duplicating the full conversation context just to do a search
- **35 anomaly sessions** burning tokens at 2-3x my normal rate
- **Bash was 40% of all tool calls**, pumping long command outputs back into context every time
- Peak hours (Mon-Fri 5-11am PT) used **1.3x more tokens** on average than off-peak

## What I actually changed

After seeing the data, three things:

1. I use /compact after ~20 turns now instead of letting sessions run endlessly
2. I stopped defaulting to Agent for codebase searches and use Grep/Glob directly
3. I try to keep heavy sessions out of peak hours when possible

Small changes, but the anomaly sessions have mostly stopped showing up.

## The tool

Open sourced it. Called **ccwhy**, written in Rust, runs completely offline on your local ~/.claude/ data. No API keys needed.

```bash
brew install SingggggYee/tap/ccwhy
```

Or: `cargo install ccwhy`

Or: [grab the binary](https://github.com/SingggggYee/ccwhy/releases)

It's not a replacement for ccusage. ccusage tells you how much you spent. ccwhy tells you why, and what to change.

[GitHub](https://github.com/SingggggYee/ccwhy)

Curious what other people's breakdowns look like. Is 97% cache reads normal, or is my setup unusually heavy?

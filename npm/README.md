# ccwhy

> ccusage tells you how much. ccwhy tells you **why**, and what to do about it.

Claude Code usage debugger. Parses your local session history, identifies the biggest token sinks (long contexts, repeated tool calls, verbose configs), and recommends specific optimizations.

```bash
npm install -g ccwhy
ccwhy
```

This package downloads the prebuilt Rust binary for your platform (macOS or Linux, arm64/x64) from [GitHub releases](https://github.com/SingggggYee/ccwhy/releases). No Rust toolchain needed.

## Other install options

```bash
brew install SingggggYee/tap/ccwhy   # via Homebrew
```

Works with [cclint](https://github.com/SingggggYee/cclint): ccwhy tells you where tokens went, cclint prevents the waste up front.

Full docs: [github.com/SingggggYee/ccwhy](https://github.com/SingggggYee/ccwhy)

MIT

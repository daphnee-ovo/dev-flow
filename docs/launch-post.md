# Launch Post Draft

Use this as the first public announcement. Keep it concrete and demo-driven.

## Short Version

I built dev-flow, a lightweight workflow layer for Claude Code and Codex CLI.

Coding agents are strong at editing code, but long tasks still drift: requirements get lost, tests get skipped, and delivery state lives only in chat. dev-flow adds a small `.dev-doc/` workspace, phase commands, and hooks so agent work stays traceable from requirement to test to iteration archive.

Install:

```bash
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash
# follow the interactive dev-flow setup
cd your-project
```

Repo: https://github.com/daphnee-ovo/dev-flow

## Suggested Title Options

- dev-flow: lightweight engineering discipline for Claude Code and Codex CLI
- I built a small workflow layer to make coding agents less chaotic
- From prompt-only coding to traceable agent delivery

## Where To Post

- GitHub repository README and release page
- X / Twitter
- Hacker News Show HN
- Reddit: r/ClaudeAI, r/OpenAI, r/LocalLLaMA if the post is framed around workflow rather than promotion
- Claude Code and Codex CLI community channels

## Demo Checklist

- Show `/init` creating `.dev-doc/`
- Show `/task` creating a real task file
- Show a hook warning or block
- Show `/iterate` archiving an iteration
- Keep the demo under 90 seconds

# Launch Post

## GitHub Discussion

Title:

```text
dev-flow: lightweight engineering discipline for Claude Code and Codex CLI
```

Body:

````markdown
I built dev-flow, a lightweight workflow layer for Claude Code and Codex CLI.

Coding agents are strong at editing code, but longer tasks still drift:

- requirements get lost in chat
- task plans stop matching the code
- tests are skipped or reported vaguely
- delivery state is hard to recover across sessions

dev-flow adds a small `.dev-doc/` workspace, phase commands, and hooks so agent work stays traceable from requirement to task to test to iteration archive.

## Quick Start

```bash
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash
# follow the interactive dev-flow setup
cd your-project
```

Then ask your coding agent:

```text
/init
/task
```

dev-flow creates `.dev-doc/`, tracks phase state in `STATUS.yaml`, writes structured task files, and uses hooks to make skipped checks or unsafe writes visible.

## Who It Is For

- people using Claude Code or Codex CLI for feature work
- projects where agent work spans multiple files or sessions
- refactors, audits, and fixes that need traceable requirements and verification

It is probably too much for one-line edits or throwaway scripts.

Repo: https://github.com/daphnee-ovo/dev-flow
Quickstart walkthrough: https://github.com/daphnee-ovo/dev-flow/blob/main/examples/quickstart-demo.md
````

## External Post

````markdown
I built dev-flow, a lightweight workflow layer for Claude Code and Codex CLI.

Coding agents are great at editing code, but long tasks still drift: requirements get lost, task plans stop matching implementation, tests are skipped, and delivery state lives only in chat.

dev-flow adds a small `.dev-doc/` workspace, phase commands, and hooks so agent work stays traceable from requirement to task to test to iteration archive.

Install:

```bash
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash
```

Then enter a project and ask the agent:

```text
/init
/task
```

Repo: https://github.com/daphnee-ovo/dev-flow
````

## Suggested Channels

- GitHub Discussions: `Show and tell`
- X / Twitter
- Hacker News: `Show HN: dev-flow, a workflow layer for Claude Code and Codex CLI`
- Reddit: `r/ClaudeAI`, `r/OpenAI`, or agent-tooling communities
- Claude Code and Codex CLI community channels

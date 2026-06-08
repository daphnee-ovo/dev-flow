**Language:** English | [中文](README.zh-CN.md)

---

<div align="center">

# dev-flow

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/daphnee-ovo/dev-flow?style=flat)](https://github.com/daphnee-ovo/dev-flow/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/daphnee-ovo/dev-flow?style=flat)](https://github.com/daphnee-ovo/dev-flow/network)

**Engineering discipline for coding agents.**

Small, focused, and opinionated. dev-flow gives coding agents structure — lightweight documents, disciplined phases, and hard constraints that turn raw coding ability into reliable engineering delivery.
</div>

## Quick Start

### One-Line Install

```bash
# macOS arm64
brew install daphnee-ovo/tap/dev-flow

# Linux / macOS / WSL
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.ps1 | iex
```

Homebrew currently supports macOS arm64. The install scripts support Linux, macOS, WSL, and Windows. Setup registers dev-flow with your preferred agent (Claude Code, Codex, or both). Project initialization happens later with `/init` inside the target project.

### First Run

```bash
cd your-project
```

Then ask your coding agent:

```text
/init
/task
```

dev-flow creates a `.dev-doc/` workspace, tracks the current phase in `STATUS.yaml`, generates structured task files, and uses hooks to remind or block the agent when workflow rules are violated.

See [examples/quickstart-demo.md](examples/quickstart-demo.md) for a concrete before/after walkthrough.

If you installed with Homebrew, run `dow setup` once before using `/init`.

---

## Why dev-flow

Coding agents are good at editing code, but weak at keeping requirements, implementation, tests, and delivery state aligned over a long task. dev-flow adds a small workflow layer around Claude Code and Codex CLI:

- clarify requirements before changing code
- split design, task planning, implementation, and QA into explicit phases
- keep `.dev-doc/` synchronized with the real project state
- use hooks to stop agents from skipping checks, writing to unsafe temp paths, or losing changelog context
- archive each delivery so later iterations have traceable history

Use it when an agent is doing feature work, refactors, audits, or multi-step fixes that need more discipline than a single prompt.

## When Not To Use It

dev-flow is intentionally opinionated. It is probably too much for one-line edits, throwaway scripts, or projects where you do not want workflow files in the repository. It is useful when the cost of agent drift is higher than the cost of lightweight process.

---

## Supported Agents

| Agent | Status | Manual setup |
|-------|--------|---------|
| **Claude Code** | Supported | `dow setup --agent claude` |
| **Codex CLI** | Supported | `dow setup --agent codex` |
| **Kiro CLI** | Testing | `dow setup --agent kiro` |

---

## Philosophy

dev-flow is not trying to become a large all-in-one project management system. Its job is to stay lightweight while helping agents clarify ideas before implementation and giving them enough structure and constraints to work like a disciplined engineering team.

Core principles:

- **Think before building** — clarify goals, boundaries, approach, and acceptance criteria before changing code.
- **Lightweight** — keep only the documents and commands that move delivery forward.
- **Structured** — PRD, SPEC, TASK, TEST, issues, and archives use stable formats that are easy to inspect and reuse.
- **Constrained** — phases, hooks, checks, and task loops prevent agents from skipping requirements, specs, verification, and delivery gates.
- **Goal-necessary** — every capability must answer whether it serves the current goal. Keep necessary constraints; do not import ceremony.
- **Synchronized** — process documents must stay aligned with the real project state: code, tasks, versions, tests, and iterations. Once management docs drift, they become noise.
- **Mode-aware** — rapid validation and long-term engineering need different gates. MVP work can focus on running functionality and obvious bugs; standard development can raise testing, review, and release requirements.

---

## Commands

| Command | Description |
|---------|-------------|
| `/init` | Initialize project (create dev-doc, select mode, validate specs) |
| `/brainstorm` | Collaborative requirement exploration & design before implementation |
| `/prd` | Launch PRD agent — produce PRD.md |
| `/spec` | Launch SPEC agent — produce SPEC.md |
| `/task` | Launch TASK agent — produce task files |
| `/issue` | Manually create issue files |
| `/devtest` | Routine dev testing (task-level verification) |
| `/fix` | Auto-read open issues and fix them |
| `/test` | Full TEST agent (project-level verification) |
| `/status` | Report current project status & progress |
| `/check` | Check if dev work is synced with .dev-doc |
| `/iterate` | Start new iteration after delivery (archive + reset) |
| `/mode` | Select development mode (full/quick/fast/mvp; audit is auto-triggered) |

---

## Development Modes

| Mode | Flow | Use Case |
|------|------|----------|
| `full` | prd → spec → task → dev → test → iterate | New projects, unclear requirements |
| `quick` | spec → task → dev → test → iterate | Clear requirements, feature development |
| `fast` | task → dev → test → iterate | Small changes, known technical approach |
| `mvp` | spec → task → dev → iterate | Quick idea validation, skip TEST |

> `audit` mode is triggered automatically when issues are created outside DEV phase. Format: `audit/<previous>`. Auto-restores after iterate.

---

## Core Features

### Role Isolation

Each phase is executed by an independent agent to avoid cognitive bias:

| Phase | Role |
|-------|------|
| PRD | Senior product manager with technical background |
| SPEC | Senior architect |
| TASK | Experienced tech lead |
| DEV | Main agent (direct execution) |
| TEST | Strict QA engineer |

### Automated Hooks

No manual operations needed:

- **Context injection** — injects current phase status and spec reminders on every message
- **Auto devtest** — triggers routine testing when a task is marked complete
- **Doc sync check** — reminds you to sync documentation when code changes
- **Changelog save** — automatically saves changelog on conversation end
- **System temp blocking** — prevents writing to system temp directories; project-local `tmp` and `temp` are allowed, and new projects default to `tmp`

### Document-Driven Development

The plugin maintains a `.dev-doc/` directory in your project:

```
.dev-doc/
├── STATUS.yaml            # Project status
├── CHANGELOG.md           # Session changelog (append-only)
├── BRAINSTORM.md          # Brainstorming notes
├── PRD.md                 # Product requirements
├── SPEC.md                # Technical specification
├── TEST.md                # Test report
├── task/                  # Task files (task_<date>_<seq>.md)
├── issue/                 # Issue tracking (issue_<source>_<date>_<seq>.md)
└── archive/               # Historical iterations (v<N>-<topic>/)
```

### Iteration Management

`/iterate` archives the current version and starts a new development cycle. All documents are versioned under `archive/v<N>-<topic>/`.

---

## Cross-Platform Support

dev-flow supports both **Claude Code** and **OpenAI Codex CLI** through a shared plugin core with per-agent adapters:

| Component | Claude Code | Codex CLI |
|-----------|-------------|-----------|
| Plugin manifest | `.claude-plugin/plugin.json` | `.codex-plugin/plugin.json` |
| Hooks config | `hooks/hooks.json` | `hooks.json` (root) |
| Project instructions | `CLAUDE.md` | `AGENTS.md` |
| Sub-agent API | `Agent({...})` | `spawn_agent` |

Commands, skills, and agents are shared across platforms. Hooks call the global `dow` CLI directly.

### dow CLI

`dow` is the unified dispatcher that powers all hooks and automation:

| Command | Description |
|---------|-------------|
| `dow setup [--agent claude\|codex\|all]` | Register plugin with agents (interactive TUI) |
| `dow update` | Self-update binary + plugins |
| `dow self-check` | Show install status and health |
| `dow status` | Read/write STATUS.yaml |
| `dow iterate` | Delivery: archive + commit + tag + bump |
| `dow doc <type>` | Generate/query document templates |
| `dow hooks ...` | Hook dispatch (context, guard, post-write) |

---

## Project Structure

```
dev-flow/
├── dow/                        # Rust CLI source (the dow binary)
│   ├── src/
│   │   ├── main.rs
│   │   ├── cli.rs
│   │   ├── commands/           # Subcommand implementations
│   │   │   ├── setup.rs        # dow setup
│   │   │   ├── update.rs       # dow update
│   │   │   └── self_check.rs   # dow self-check
│   │   ├── hooks/              # Hook implementations
│   │   └── core/               # Shared libraries
│   │       ├── config.rs       # ~/.config/dow/config.toml
│   │       ├── platform.rs     # XDG paths, platform detection
│   │       ├── github.rs       # Release API, self-update
│   │       └── agent_registry.rs # Plugin deployment
│   └── Cargo.toml
├── plugin/                     # Shared plugin content (agent-agnostic)
│   ├── skills/
│   ├── commands/
│   └── agents/
├── targets/                    # Per-agent adapter layer
│   ├── claude/
│   │   ├── plugin.json
│   │   └── hooks.json
│   └── codex/
│       ├── plugin.json
│       └── hooks.json
├── install/                    # One-line install scripts
│   ├── install.sh              # curl | bash
│   └── install.ps1             # irm | iex
├── examples/                   # Quickstart and workflow walkthroughs
├── devtools/                   # Development helpers
│   ├── assemble.sh             # Assemble dist/<agent>/
│   └── deploy-local.sh         # Build + deploy locally
├── .github/workflows/
│   └── release.yml             # CI: tag → build → GitHub Release
├── VERSION
├── CLAUDE.md
├── AGENTS.md
├── README.md
└── LICENSE
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for local development setup and conventions.

---

## Credits

The `/brainstorm` command is inspired by [superpowers](https://github.com/obra/superpowers).

---

## License

[MIT](LICENSE)

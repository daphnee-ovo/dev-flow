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

---

## Supported Agents

| Agent | Status | Install |
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

## Quick Start

### One-Line Install

```bash
# Linux / macOS / WSL
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.ps1 | iex
```

The installer downloads `dow` (the CLI), then launches an interactive setup to register with your preferred agent (Claude Code, Codex, or both).


### First Run

```
/init          → Initialize project, select development mode
/brainstorm    → Collaborative requirement exploration (optional)
/prd           → Produce requirements document
/spec          → Produce technical specification
/task          → Break down task list
               → Development (auto-triggers /devtest on task completion)
/test          → Full test suite
```

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

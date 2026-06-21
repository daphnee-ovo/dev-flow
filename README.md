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
# npm (Node.js 16+, auto-downloads platform binary + runs dow setup)
npm install -g @xin_yue/dev-flow

# Cargo (Rust toolchain required)
cargo install dev-flow && dow setup

# macOS arm64 / Linux x86_64 / Linux aarch64
brew install daphnee-ovo/tap/dev-flow && dow setup

# Linux / macOS / WSL
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.ps1 | iex
```

The install scripts run `dow setup` automatically. For Cargo and Homebrew installs, `dow setup` registers dev-flow with your preferred agent (Claude Code, Codex, or Kiro). Project initialization happens later with `/init` inside the target project.

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

See [examples/quickstart-demo.md](examples/quickstart-demo.md) for a concrete before/after walkthrough, or inspect [examples/sample-project](examples/sample-project/) for static `.dev-doc` output.

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
| **Kiro** | Testing | `dow setup --agent kiro` |

### Agent Compatibility Matrix

#### Install & Setup

| Capability | Claude Code | Codex CLI | Kiro |
|------------|:-----------:|:---------:|:----:|
| `dow setup` registration | Yes | Yes | Yes |
| `dow self-check` validation | Yes | Yes | Yes |
| Plugin manifest | `plugin.json` | `plugin.json` | `config.json` |
| Project instructions file | `CLAUDE.md` | `AGENTS.md` | `.kiro/steering/` |

#### Hook Support

| Hook | Claude Code | Codex CLI | Kiro |
|------|:-----------:|:---------:|:----:|
| UserPromptSubmit (context injection) | Yes | Yes | Yes |
| PreToolUse — Write/Edit guard | Yes | Yes | Yes |
| PreToolUse — Bash guard | Yes | Yes | Yes |
| PostToolUse — Write/Edit sync | Yes | Yes | Yes |
| PostToolUse — Bash sync | Yes | Yes | Yes |
| Stop (changelog save) | Yes | Yes | Yes |

#### Command Support

| Command | Claude Code | Codex CLI | Kiro |
|---------|:-----------:|:---------:|:----:|
| `/init` | Slash command | Skill | Agent command |
| `/brainstorm` | Slash command | Skill | Agent command |
| `/prd` | Slash command | Skill | Agent command |
| `/spec` | Slash command | Skill | Agent command |
| `/task` | Slash command | Skill | Agent command |
| `/issue` | Slash command | Skill | Agent command |
| `/devtest` | Slash command | Skill | Agent command |
| `/fix` | Slash command | Skill | Agent command |
| `/test` | Slash command | Skill | Agent command |
| `/status` | Slash command | Skill | Agent command |
| `/check` | Slash command | Skill | Agent command |
| `/iterate` | Slash command | Skill | Agent command |
| `/mode` | Slash command | Skill | Agent command |

#### Sub-Agent Support

| Capability | Claude Code | Codex CLI | Kiro |
|------------|:-----------:|:---------:|:----:|
| PRD agent | Yes (`Agent`) | Yes (`spawn_agent`) | Yes (subagent) |
| SPEC agent | Yes (`Agent`) | Yes (`spawn_agent`) | Yes (subagent) |
| TASK agent | Yes (`Agent`) | Yes (`spawn_agent`) | Yes (subagent) |
| TEST agent | Yes (`Agent`) | Yes (`spawn_agent`) | Yes (subagent) |

#### Known Limitations

| Agent | Limitations |
|-------|-------------|
| **Claude Code** | None known |
| **Codex CLI** | No native slash commands — commands are exposed as skills (`SKILL.md`). Hook protocol uses JSON envelope (`--codex-hook`). |
| **Kiro** | Testing status — not yet validated in production workflows. No native slash commands — commands are handled through agent configuration. Hook protocol uses `--kiro-hook` flag. |

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
├── archive.db             # SQLite archive queried by `dow archive ...`
└── main/                  # Current branch workflow documents
    ├── STATUS.yaml        # Project status
    ├── CHANGELOG.md       # Session changelog (append-only)
    ├── BRAINSTORM.md      # Brainstorming notes
    ├── PRD.md             # Product requirements
    ├── SPEC.md            # Technical specification
    ├── TEST.md            # Test report
    ├── task/              # Task files (task_<date>_<seq>.md)
    └── issue/             # Issue tracking (issue_<source>_<date>_<seq>.md)
```

### Iteration Management

`/iterate` archives completed tasks, closed issues, test reports, changelog entries, and phase documents into `.dev-doc/archive.db`, then starts a new development cycle. Use `dow archive list/show/tasks/issues/doc` to query historical iterations.

If `.dev-doc/preIterate.yaml` exists, `dow iterate --confirm` runs its steps before archive, commit, tag, and bump. A failing step stops the whole iteration. Supported steps are `sync-version: <path>` for explicit Cargo/npm/uv project manifests and `run: <command>` for project-local checks, lockfile updates, or generators.

```yaml
sync-version: dow/Cargo.toml
sync-version: npm/dev-flow/package.json
run: cargo update -p dev-flow --manifest-path dow/Cargo.toml
run: npm run build
```

`dow revoke --version <v>` is the inverse of iterate — it restores archived tasks, issues, and documents from the database, handles file sequence conflicts, and marks the iteration as revoked. Use `dow revoke --list` to see revokable versions.

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
| `dow revoke --version <v>` | Undo an iteration: restore tasks/issues/docs from archive |
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

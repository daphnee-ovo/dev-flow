**Language:** English | [中文](README.zh-CN.md)

---

<div align="center">

# dev-flow

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/daphnee-ovo/dev-flow?style=flat)](https://github.com/daphnee-ovo/dev-flow/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/daphnee-ovo/dev-flow?style=flat)](https://github.com/daphnee-ovo/dev-flow/network)

**Full lifecycle project management plugin for Claude Code & Codex CLI.**

From brainstorming to delivery — structured phases, role isolation, and automated hooks.

</div>

---

## Quick Start

### Claude Code

```bash
# Add marketplace
/plugin marketplace add daphnee-ovo/dev-flow

# Install plugin
/plugin install dev-flow@daphnee-ovo
```

### Codex CLI

```bash
# Add marketplace
codex plugin marketplace add daphnee-ovo/dev-flow
```

Then open `/plugins` in Codex, search for `Dev-Flow` and install. Run `/init` to initialize your project.

> For local development, you can also add the repo directly:
> ```bash
> codex plugin marketplace add .
> ```

### First Run

```
/init          → Initialize project, select development mode
/brainstorm    → Collaborative requirement exploration (optional)
/prd           → Produce requirements document
/spec          → Produce technical specification
/task          → Break down task list
               → Development (auto-triggers /devtest on task completion)
/test          → Full test suite
/done          → Delivery confirmation
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
| `/done` | Delivery checklist |
| `/status` | Report current project status & progress |
| `/check` | Check if dev work is synced with dev-doc |
| `/iterate` | Start new iteration after delivery (archive + reset) |
| `/mode` | Select development mode (full/quick/fast/mvp) |

---

## Development Modes

| Mode | Flow | Use Case |
|------|------|----------|
| `full` | brainstorm → prd → spec → task → dev → test → done | New projects, unclear requirements |
| `quick` | spec → task → dev → test → done | Clear requirements, feature development |
| `fast` | task → dev → test → done | Small changes, known technical approach |
| `mvp` | brainstorm → spec → dev | Quick idea validation, prototyping |

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
- **`/tmp/` blocking** — prevents writing to system temp directories (enforces project-local tmp/)

### Document-Driven Development

The plugin maintains a `dev-doc/` directory in your project:

```
dev-doc/
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

dev-flow supports both **Claude Code** and **OpenAI Codex CLI** with platform-specific configurations:

| Component | Claude Code | Codex CLI |
|-----------|-------------|-----------|
| Plugin manifest | `.claude-plugin/plugin.json` | `.codex-plugin/plugin.json` |
| Skill entry | `.claude/skills/dev-flow/SKILL.md` | `skills/dev-flow/SKILL.md` |
| Hooks config | `hooks/hooks.json` | `hooks.json` (root) |
| Project instructions | `CLAUDE.md` | `AGENTS.md` |
| Sub-agent API | `Agent({...})` | `spawn_agent` |

Commands in `commands/` are written in a runtime-neutral style that works across both platforms.

---

## Project Structure

```
dev-flow/
├── .claude-plugin/
│   ├── plugin.json            # Claude Code plugin config
│   └── marketplace.json       # Marketplace metadata
├── .codex-plugin/
│   └── plugin.json            # Codex CLI plugin manifest
├── .claude/skills/dev-flow/
│   └── SKILL.md               # Claude Code skill trigger
├── skills/dev-flow/
│   └── SKILL.md               # Codex CLI skill entry
├── commands/                   # Slash command definitions
│   ├── init.md
│   ├── brainstorm.md
│   ├── prd.md
│   ├── spec.md
│   ├── task.md
│   ├── devtest.md
│   ├── fix.md
│   ├── test.md
│   ├── done.md
│   ├── status.md
│   ├── check.md
│   ├── iterate.md
│   └── mode.md
├── agents/                     # Agent prompt templates
│   ├── prd-agent.md
│   ├── spec-agent.md
│   ├── task-agent.md
│   └── test-agent.md
├── hooks/
│   └── hooks.json              # Claude Code hook registration
├── hooks.json                  # Codex CLI hook registration
├── scripts/
│   ├── hooks/                  # Hook scripts
│   │   ├── inject-context.sh
│   │   ├── block-system-tmp.sh
│   │   ├── check-task-completion.sh
│   │   ├── check-doc-sync.sh
│   │   ├── check-phase-completion.sh
│   │   ├── update-status.sh
│   │   └── save-changelog.sh
│   ├── commands/               # Scripted commands
│   │   ├── status.sh
│   │   ├── check.sh
│   │   ├── mode.sh
│   │   └── iterate.sh
│   └── init/                   # Init command scripts
│       ├── scan-project.sh
│       ├── validate.sh
│       └── migrate.sh
├── references/                 # Internal spec references
│   ├── dev-flow-spec.md
│   └── dev-doc/                # Document format templates
│       ├── STATUS.yaml
│       ├── TASK-FILE.md
│       ├── CHANGELOG.md
│       ├── TEST.md
│       └── ISSUE.md
├── CLAUDE.md
├── AGENTS.md
├── README.md
├── README.zh-CN.md
├── CONTRIBUTING.md
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

---
name: dev-flow
description: "Full project lifecycle management. Commands: /init (project initialization), /brainstorm (brainstorming), /prd (requirements exploration), /spec (technical specification), /task (task decomposition), /issue (create issue), /devtest (routine testing), /fix (auto-fix issues), /test (complete testing), /status (status report), /check (doc sync check), /iterate (start new iteration), /mode (dev mode). Triggered when user mentions create project, start project, initialization, project status, next step, start development, new version, iteration, brainstorming, ideas, mode."
---

# Dev-Flow: Full Project Lifecycle Management

## Skill Load Confirmation

When this skill is triggered, output:

```
[dev-flow] skill loaded | Current phase: <read from STATUS.yaml, show "new project" if doesn't exist>
```

## Codex Runtime Conventions

- Codex doesn't support directly registering slash commands; `init`, `status` etc. processes are carried and triggered by `skills/<command>/SKILL.md` in Codex package.
- When command requires "independent agent" or "subagent", use `spawn_agent` in Codex. This is explicit subagent request from dev-flow commands.
- When command requires updating project-level agent instructions, Codex projects prioritize updating `AGENTS.md`, Claude Code projects prioritize updating `CLAUDE.md`; if both exist, keep both synced.
- Codex file editing uses currently available editing tools; don't treat Claude Code's `Agent({...})` examples as API that must be executed verbatim.

## Process Overview

```
[Brainstorm(BRAINSTORM)] → Requirements(PRD) → Spec(SPEC) → Tasks(TASK) → Dev(DEV) → Test(TEST) → Iterate(ITERATE) → Next Round
      Optional                                                     │                ↑
                                                                   └── Routine TEST ──→│
```

> `/brainstorm` available in any mode, always optional.

## Command Mapping

| Command | Phase | Role |
|---------|-------|------|
| `/init` | Initialization | Create .dev-doc, select mode |
| `/brainstorm` | PRD precursor | Collaborative design exploration |
| `/prd` | PRD | Tech-savvy senior product manager |
| `/spec` | SPEC | Senior architect |
| `/task` | TASK | Experienced technical lead |
| `/issue` | DEV/TEST | Manually create issue |
| `/devtest` | DEV (inner loop) | Lightweight QA |
| `/fix` | DEV/TEST | Auto-fix issues |
| `/test` | TEST | Strict QA engineer |
| `/status` | Any | Status report |
| `/check` | Any | Doc sync check |
| `/iterate` | Any (when delivery conditions met) | Archive + commit & tag + bump version |
| `/mode` | Any | Mode selection (full/quick/fast/mvp; audit auto-triggered) |

## Development Modes (/mode)

| Mode | Flow | Description |
|------|------|-------------|
| `full` | PRD → SPEC → TASK → DEV → TEST → ITERATE | Complete process |
| `quick` | SPEC → TASK → DEV → TEST → ITERATE | Skip PRD |
| `fast` | TASK → DEV → TEST → ITERATE | Minimal design |
| `mvp` | SPEC → TASK → DEV → ITERATE | Skip TEST |
| `audit` | (Auto-triggered, cannot manually set) | See below |

### audit Mode

audit mode for handling urgent issues found in non-DEV phases, **cannot manually set**, only auto-triggered by system.

**Trigger condition**: When creating issue file (`issue/issue_*.md`) in non-DEV phase, `dow hooks post-write` auto-switches mode to `audit/<original_mode>`.

**Behavior**:
- mode field written as `audit/<previous>` (e.g., `audit/quick`), keeps original mode info
- phase forced to DEV, directly enter fix process
- iterate skips task completion check (because audit round only cares about issue fixes)
- `dow hooks context` outputs audit-specific prompt: `issue → /fix → /iterate restore original mode`

**Restore**: After executing `/iterate`, auto-restores original mode (from `audit/quick` restores to `quick`), phase resets to original mode's start phase.

## VERSION File Mechanism

`VERSION` file in project root records current semantic version number (format: `MAJOR.MINOR.PATCH`).

**Version operations in iterate process**: auto-completed by `dow iterate` — archive, bump VERSION, commit, tag, one commit contains all changes.

**Supported bump types**: `major` (major version), `minor` (feature version, default), `patch` (patch version)

## Role Isolation

Different phases executed by independent agents, avoid context interference. Each agent only receives minimal input needed for that phase.

| Phase | Execution Method | Input |
|-------|------------------|-------|
| PRD | Independent agent | User description + BRAINSTORM.md (if any) |
| SPEC | Independent agent | PRD.md (or BRAINSTORM/description by mode) + project context |
| TASK | Independent agent | SPEC.md (or description by fast mode) + project context |
| DEV | Main agent executes directly | task/*.md + SPEC.md |
| TEST | Independent agent | SPEC.md + task/*.md + project context |

## DEV Phase Rules

Dev phase executed by main agent, follows:
- **[BLOCKED] blocking rule**: when hook output contains `[BLOCKED]`, prohibit any dev operations (edit code, run commands), only allow executing `/task`, `/issue`, `/iterate` to create tasks or issues
- **Before starting task**: first read task file to get complete context (done_when, files, refs), if task has `refs` field then read corresponding SPEC sections as implementation basis
- Only do tasks listed in task/, no more no less
- Check task immediately after completion, immediately trigger `/devtest`
- Docs updated in real-time, "fix later" not allowed
- After all tasks complete auto-enter `/test`

## Directory Structure

```
.dev-doc/
├── STATUS.yaml
├── CHANGELOG.md
├── BRAINSTORM.md
├── PRD.md
├── SPEC.md
├── TEST.md
├── task/
│   ├── task_2026-05-15_1.md
│   └── done_task_2026-05-14_1.md
├── issue/
│   ├── issue_test_2026-05-15_1.md
│   └── closed_issue_test_2026-05-14_1.md
└── archive.db          ← SQLite archive (dow archive queries)
```

## dow CLI

`scripts/bin/dow` is unified dispatcher written in Rust, all hooks and scripted operations execute through it.

| Subcommand | Function |
|------------|----------|
| `dow status` | Read/write STATUS.yaml (`--phase`/`--mode`/`--exec-mode`/`--name`/`--field`) |
| `dow check` | Doc spec check |
| `dow issue --list` | List unclosed issues |
| `dow iterate --topic <t> --type <type> [--files f1 f2...] [-v minor] [--confirm]` | Iteration delivery |
| `dow scan` | Project scan |
| `dow validate` | Validate .dev-doc structure |
| `dow doc <type> [--md\|--json] [-n N] [--source X]` | Generate doc template / query doc spec |
| `dow devtest [--task <id>]` | Task-level testing |
| `dow test [--file <x>]` | Comprehensive testing |
| `dow archive list [--branch <b>]` | List all archived versions |
| `dow archive show <version>` | Specific version archive details |
| `dow archive tasks [--version v] [--priority P0]` | Query archived tasks |
| `dow archive issues [--version v] [--severity P0]` | Query archived issues |
| `dow archive doc <version> <PRD\|SPEC\|TEST>` | Output archived doc original text |
| `dow archive migrate [--delete-originals]` | Migrate from directory to SQLite |
| `dow archive stats` | Archive statistics |
| `dow hooks context` | hook: inject context |
| `dow hooks guard <file>` | hook: file write guard |
| `dow hooks post-write <file>` | hook: post-write linkage |
| `dow hooks save-changelog` | hook: save CHANGELOG |
| `dow version [--set X.Y.Z] [--bump major\|minor\|patch]` | Read/write VERSION |
| `dow init --name <name> --mode <mode>` | Initialize dev-flow workflow management |
| `dow inbox context` | Internal common library: generate project context |

Default JSON output, `-H` switches to human-friendly format. Build: `bash dow/build.sh`.

## Format Spec Queries

Query doc format specs via `dow doc <type>` command, format definitions embedded in binary at compile time:

| Command | Definition Content |
|---------|-------------------|
| `dow doc task --md/--json` | task file format (priority/refs/files/done_when etc.) |
| `dow doc spec --md/--json` | SPEC.md format (includes mode degradation rules) |
| `dow doc prd --md/--json` | PRD.md format (MoSCoW priority) |
| `dow doc issue --md/--json` | issue file format |
| `dow doc test --md/--json` | test report format |
| `dow doc brainstorm --md/--json` | brainstorm doc format |
| `dow doc changelog --md/--json` | changelog format |

Dispatch layer (commands/*.md) assembles subagent prompts by getting structured format definition via `dow doc <type> --json`.

## Flexibility

- Small projects can merge phases (e.g., PRD+SPEC in one step)
- When user clearly knows what they want, don't force complete process
- Process serves project, not project serves process

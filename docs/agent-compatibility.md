# Agent Compatibility

dev-flow shares one workflow core across agents, but each agent has a different plugin, hook, and sub-agent surface.

## Summary

| Agent | Status | Setup | Commands / Skills | Hooks | Sub-agents | Project instructions |
|-------|--------|-------|-------------------|-------|------------|----------------------|
| Claude Code | Supported | `dow setup --agent claude` | Slash commands from `plugin/commands/` | `targets/claude/hooks.json` | `plugin/agents/*.md` via Claude agent flow | `CLAUDE.md` |
| Codex CLI | Supported | `dow setup --agent codex` | Codex skills generated from shared commands | `targets/codex/hooks.json` | Uses `spawn_agent` semantics in command prompts | `AGENTS.md` |
| Kiro CLI | Testing | `dow setup --agent kiro` | Kiro skills generated from shared commands | `targets/kiro/agents/dev-flow/config.json` | Kiro agent config under `targets/kiro/agents/` | `.kiro/steering/` |

## Claude Code

Claude Code is a supported target.

- Plugin manifest: `targets/claude/plugin.json`
- Hook config: `targets/claude/hooks.json`
- Shared commands: `plugin/commands/*.md`
- Shared phase agents: `plugin/agents/*.md`
- Setup behavior: deploys the Claude bundle, injects global dev-flow instructions, and registers the local plugin marketplace.

Known limitations:
- Requires the `claude` CLI for automatic marketplace update/install during setup.
- Hook behavior depends on Claude Code hook execution.

## Codex CLI

Codex CLI is a supported target.

- Plugin manifest: `targets/codex/plugin.json`
- App manifest: `targets/codex/app.json`
- Hook config: `targets/codex/hooks.json`
- Shared commands are exposed as Codex skills during bundle assembly.
- Setup behavior: deploys the Codex bundle, resolves the runtime from `CODEX_BIN`, Codex/ChatGPT App bundles, or PATH, writes local marketplace metadata, enables plugin hooks, and installs the plugin.

Known limitations:
- Codex does not register slash commands directly; dev-flow maps command prompts into skills.
- Hook-visible edits matter. Prefer Codex edit/write tools for source and docs so hooks can observe changes.

## Kiro CLI

Kiro CLI support is in testing.

- Agent config: `targets/kiro/agents/dev-flow/config.json`
- Setup behavior: deploys generated skills under `~/.kiro/skills/`, deploys agent config under `~/.kiro/agents/`, and injects steering content when a `.kiro` environment is present.
- Hook commands use `--kiro-hook` output mode.

Known limitations:
- Kiro support has less validation coverage than Claude Code and Codex CLI.
- `dow doctor` performs the unified project and installation checks; Kiro has less plugin-integrity coverage than Claude/Codex today.
- Users may need to run `/agent set-default dev-flow` after setup.

## Shared Workflow Surface

All supported agents use the same `dow` CLI backend:

- `dow init`
- `dow status` and `dow status set ...`
- `dow doctor`
- `dow task/issue/prd/spec/brainstorm/changelog schema`
- `dow hooks ...`
- `dow test`
- `dow iterate`
- `dow archive ...`

The durable workflow state is stored in `.dev-doc/`, with historical iterations written to `.dev-doc/archive.db`.

## Verification Status

Current project checks cover:

- shared command documentation
- Claude plugin command registration
- dev-flow document format references
- SQLite archive documentation

Kiro-specific end-to-end setup and hook behavior should be treated as testing until a dedicated compatibility test is added.

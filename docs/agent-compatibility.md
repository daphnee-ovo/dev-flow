# Agent Compatibility

dev-flow provides identical workflow experience across all supported agents. The `dow` CLI backend, hook behavior, workflow state (`.dev-doc/`), and all commands work the same way regardless of which agent you use.

## Support Status

| Agent | Status | Setup |
|-------|--------|-------|
| Claude Code | Supported | `dow setup --agent claude` |
| Codex CLI / Codex App | Supported | `dow setup --agent codex` |
| Kiro | Supported | `dow setup --agent kiro` |
| Pi | Testing | `dow setup --agent pi` |

> Codex App tends to follow command instructions even more closely than Codex CLI.

## Shared File-Scope Input

All agents use the same nested file-scope contract for task and issue
create/update commands. CLI calls pass one JSON object through `--file`, for
example `--file '{"modify":["src/main.rs"]}'`; stdin JSON places the object
under the top-level `files` key. The `create` and `modify` lists are
individually optional, but at least one must contain a non-empty path. Legacy
flat file fields and `--files-*` flags are not part of the shared contract.

## Implementation Differences

These are not limitations — just how each agent platform surfaces the same capabilities:

| Aspect | Claude Code | Codex | Kiro | Pi |
|--------|-------------|-------|------|----|
| Command interface | Slash commands (`/init`, `/spec`...) | Skill commands | Skill commands | Skill commands |
| Sub-agent invocation | `Agent` tool | `spawn_agent` | subagent | `Agent` tool |
| Project instructions | `CLAUDE.md` | `AGENTS.md` | `.kiro/steering/` | `AGENTS.md` |
| Hook config | `targets/claude/hooks.json` | `targets/codex/hooks.json` | `targets/kiro/agents/dev-flow/config.json` | Pi Extension |

Assembly (`devtools/assemble.sh <agent>`) transforms shared `plugin/commands/*.md` into each agent's native format automatically.

## Known Limitation: Kiro Default Agent

Kiro's built-in default agent does not support hook configuration. After setup, you must set the dev-flow agent as default:

```bash
kiro-cli agent set-default --name dev-flow
```

Without this, hooks (context injection, file guards, changelog save) will not fire. `dow setup --agent kiro` reminds you of this step after registration completes.

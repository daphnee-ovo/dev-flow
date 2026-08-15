# GUIDES.md

This file is an on-demand guide for the dev-flow workflow. It is not per-turn discipline.

## Slash Commands
| Command | Phase | Purpose |
|---------|-------|---------|
| `/init` | any | Initialize project |
| `/status` | any | Report status |
| `/mode` | any | Set dev mode |
| `/issue` | any | Create issue |
| `/brainstorm` | BRAINSTORM | Collaborative exploration |
| `/prd` | PRD | Write requirements |
| `/spec` | SPEC | Technical design |
| `/task` | TASK | Decompose into tasks |
| `/fix` | DEV | User-triggered workflow to read, claim, fix, verify, and close open issues |
| `/test` | TEST | Full project test; Task tests use dow test TASK-ID |
| `/iterate` | ITERATE | Archive + commit + tag + bump |

## Mode flows

| Mode | Flow |
|------|------|
| `full` | prd → spec → task → dev → test → iterate |
| `quick` | spec → task → dev → test → iterate |
| `fast` | task → dev → test → iterate |
| `mvp` | spec → task → dev → iterate |

`/brainstorm` is optional in every mode.

When unsure about command syntax, run `dow <command> --help` or `dow task schema`. Do not guess.

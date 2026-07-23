---
description: Iteration delivery — delivery check + archive + commit & tag + bump version
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# ITERATE — Iteration Delivery

## General Principles

`/iterate` is dev-flow's iteration wrap-up command. After execution completes current version delivery and starts next iteration.
Includes all responsibilities of original `/done` (delivery check) and original `/iterate` (archive + reset).

## Pre-checks (blocking)

`dow iterate` auto-executes, stops if any fails:
1. All tasks in task files must be checked `[x]` (audit mode skips)
2. No unclosed P0 issues
3. VERSION file exists and format legal
4. All CI steps in `.dev-doc/preIterate.ci` must succeed (if file exists)

## Parameters

| Parameter     | Description                                                                                                                                                                 | Default  |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| `--topic`     | Archive topic (for archive directory naming)                                                                                                                                | Required |
| `--type`      | commit type (feat/fix/refactor/docs/perf/test/style/workflow)                                                                                                               | Required |
| `--files`     | Additional source code files/directories to commit (space-separated). **No need to pass .dev-doc/ files** — they're auto-managed by iterate (archive delete + `git add -u`) | Optional |
| `-v`/`--bump` | Version increment type: major/minor/patch                                                                                                                                   | patch    |
| `--tag`       | Force create git tag for patch bump                                                                                                                                         | -        |
| `--confirm`   | Confirmation token (`ITR-xxxxxx`, from preview output)                                                                                                                      | -        |

## Execution Flow

### Phase 1: Preview (without --confirm)

```bash
dow iterate --topic <topic> --type <type> [--files f1 f2...] [-v patch]
```

Outputs preview info: archive content, version number, tag to be created, commit file list, confirmation token.

### Phase 2: Confirm Execution (with --confirm token)

```bash
dow iterate --topic <topic> --type <type> [--files f1 f2...] --confirm ITR-xxxxxx
```

Token from preview output, valid for 5 minutes. After confirmation executes in sequence:

1. **preIterate CI** — if `.dev-doc/preIterate.ci` exists, execute its steps in order first; any step fails stops entire iterate, no archive, no commit, no tag, no bump
2. **Archive** — parse task_*, done_task_*, closed_issue_*, PRD.md, SPEC.md, BRAINSTORM.md, CHANGELOG.md and write to `.dev-doc/archive.db` (SQLite), then delete the archived source files
3. **Reset CHANGELOG** — clear to `# Changelog\n`
4. **git commit + tag** — `git add -u` + explicitly add specified files and archive.db, commit message format `<type>: Release v<version> <topic>`, CHANGELOG entries as commit body
5. **bump version** — increment version number write to VERSION
6. **reset phase** — determine new iteration initial phase by mode

## preIterate CI

Can create `.dev-doc/preIterate.ci` in project root:

```yaml
sync-version: dow/Cargo.toml
sync-version: npm/dev-flow/package.json
run: cargo update -p dev-flow --manifest-path dow/Cargo.toml
run: npm run build
```

Supports two step types:

| Step                   | Description                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------- |
| `sync-version: <path>` | Sync explicitly declared Cargo, npm, uv/pyproject manifest version to this delivery version |
| `run: <command>`       | Execute command in project root; exit code non-0 blocks entire iterate                      |

preIterate always executes before `git commit`. File changes produced by steps go into same iterate commit. `dow` doesn't auto-scan or guess which manifests, lockfiles, artifacts need syncing; project-specific syncing must be explicitly written in `preIterate.ci`.

## Commit Message Format

```
<type>: v<version> <topic>

- <CHANGELOG entry 1>
- <CHANGELOG entry 2>
...
```

## Bump Type Decision

The agent MUST evaluate bump level by comparing delivered scope against project goals:

1. Run `dow status` to read `goals_minor` and `goals_major`
2. Compare the CHANGELOG entries / completed tasks against these goals:
   - If delivered work **fulfills or substantially completes** a `goals_minor` → recommend `minor`
   - If delivered work **fulfills or substantially completes** a `goals_major` → recommend `major`
   - If delivered work is incremental progress toward goals (partial, not a milestone) → `patch`
3. Also consider overall change magnitude:
   - Patch: ≤20 tasks, no new user-facing features, docs/fix/style only
   - Minor: new features, behavioral changes, significant improvements
   - Major: breaking changes, architecture refactor, dependency overhaul
4. Present the recommendation with reasoning to the user for confirmation

Override rules:
- User explicitly specifies `-v minor/major` → use that
- Patch + `--tag` → patch with git tag

## Execution Method

```bash
# Preview
dow iterate --topic "<topic>" --type <type> --files <file1> <file2>

# Confirm execution (token from preview output)
dow iterate --topic "<topic>" --type <type> --files <file1> <file2> --confirm ITR-xxxxxx
```

**CRITICAL: Preview and confirm MUST happen in separate turns.**

Before agent calls:
1. Ask user for this iteration's topic and commit type
2. Run `dow status` to get goals; evaluate bump type (compare delivered scope vs goals + change magnitude)
3. Run preview, display summary to the user
4. **STOP. Wait for explicit user confirmation.** Do NOT run the confirm command in the same turn as the preview. The user must see the preview output and approve before proceeding.
5. Only after the user confirms (next message), execute with the token

## audit Mode Behavior

When `mode` is `audit/xxx` format (i.e., entered audit mode via `/mode audit`):

1. **Skip task completion check** — audit mode allows iterate when tasks not all complete
2. **P0 issue check still retained** — even in audit mode, must close all P0 issues before iterate
3. **Auto-restore original mode after iterate completes** — extract original mode `xxx` from `audit/xxx`, write back to STATUS.yaml mode field, and determine new iteration start phase by that mode (e.g., `audit/quick` → restore to `quick`, phase resets to SPEC)
4. If original mode invalid or empty, default restore to `quick`

## Notes

- Archive written to SQLite (`.dev-doc/archive.db`); PRD.md, SPEC.md, and BRAINSTORM.md source files are deleted after archiving, while CHANGELOG.md is reset for the next session
- If SQLite already has same version record, means duplicate operation, INSERT OR IGNORE skips
- `git add -u` handles tracked file modifications/deletions; `--files` and preIterate-produced files explicitly added to commit
- Query historical archives use `dow archive` subcommands (list/show/tasks/issues/doc/stats)

## After Completion Output

```
[dev-flow] Iteration Complete
━━━━━━━━━━━━━━━━━━━━━━
Delivered Version: v2.2.0 (tagged)
New Version: v2.3.0
Phase Reset: SPEC
```

---
description: Initialize dev-flow project — scan current state, create/align .dev-doc, update agent instruction file
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# INIT — Project Initialization

## General Principles

`/init` is the entry command for dev-flow. After execution, guarantee:
1. `.dev-doc/` directory structure complies with spec
2. All document formats pass validation
3. Persistent docs (`docs/`) initialized and populated
4. Project-level agent instruction file correctly reflects project info
5. Project state consistent with reality

## Execution Flow

### Phase 1: Environment Detection + Project Scan

Run scan to get project info:

```bash
dow scan
```

Outputs project name, tech stack, commands, directory structure, git status, existing .dev-doc etc.

Judge path based on output:
- Output shows `dev_doc: none` → Path A (brand new project)
- Otherwise → Path B (existing project)

---

### Phase 2A: Brand New Project Initialization

1. Ask for project name and dev mode (skip asking if scan output has clear info)
2. Execute initialization:
   ```bash
   dow init --name <project_name> --mode <mode>
   ```
   Auto-creates directory structure (.dev-doc/{issue,task,archive}, tests, tmp), writes STATUS.yaml and project root VERSION (multi-branch format). Also generates persistent doc skeleton (README.md + docs/{structure,decisions,usage}.md) and registers in STATUS.yaml.
   If project already has `temp` directory, reuse it, don't create `tmp`. Existing doc files won't be overwritten.
3. Jump to Phase 4

---

### Phase 2B: Existing Project Alignment

#### 2B-1. State Inference

Judge actual phase based on script scan results:

| Condition | Inferred Phase |
|-----------|----------------|
| Has code + has passing tests + has TEST.md | DONE or TEST |
| Has code + has TASK.md partially complete | DEV |
| Has SPEC.md but no/little code | TASK or SPEC |
| Has PRD.md but no SPEC.md | PRD → SPEC |
| Only README or scattered code | Determine initial phase by mode |

Initial phase by mode:

| Mode | Initial Phase | Description |
|------|---------------|-------------|
| `full` | PRD | Start from requirements definition |
| `quick` | SPEC | Skip requirement exploration |
| `fast` | TASK | Directly decompose tasks |
| `mvp` | SPEC | Quick validation path (brainstorm → spec → dev) |

#### 2B-2. Report to User

Output scan summary, ask for confirmation:
- Project name
- Development mode
- Whether inferred phase is correct

---

### Phase 2C: Old Format Migration (execute in Path B)

Run migration detection script:

```bash
dow validate
```

Script auto-detects and migrates:
- `TASK.md` → `task/task_<today>_<seq>.md` (keeps `.bak`)
- `session/` → extract summary to generate `CHANGELOG.md`
- `phase: MVP` in `STATUS.yaml` → `phase: DEV`

Skip if output shows `status: no_migration_needed`.

---

### Phase 3: Spec Validation

Run validation script:

```bash
dow validate
```

Script auto-completes:
- Create missing directories
- Check STATUS.yaml field completeness
- Check task/ file format and Done when
- Check issue file naming and frontmatter
- Complete .gitignore

Script outputs report in three categories:
- `auto_fixed`: auto-fixed (directory creation, gitignore etc.)
- `needs_confirm`: needs agent confirmation before handling (file renaming)
- `warnings`: needs agent to fix (missing fields, format errors)

**Agent only handles `needs_confirm` and `warnings`**:
- `needs_confirm` → ask user confirmation then execute (e.g., rename issue/task files)
- `warnings` → fix directly (e.g., complete missing STATUS.yaml fields, complete issue yaml header)
  - `issue_nums_mismatch` → directly correct nums value in frontmatter to actual item count
  - `issue_bad_item_format` → correct item format to `- [ ] I<N>: <title>`
  - `issue_missing_required_fields` → ask user to supplement missing fields or mark placeholders
  - `issue_invalid_severity` → correct to legal values P0/P1/P2
- `auto_fixed` → only inform user in final report

**Spec Reference**: When handling `warnings`, agent must obtain corresponding doc format spec via `dow doc <type> --json` (e.g., use `dow doc issue --json` for issue format issues, `dow doc task --json` for task format issues), ensure fix content complies with spec definition. Don't infer correct format just from warning type name.

---

### Phase 3.5: Persistent Documentation

Initialize and populate project persistent docs (`docs/` directory).

#### 3.5-1. Generate Skeleton

```bash
dow doc init
```

Auto-creates `docs/{structure.md, decisions.md, usage.md}` (existing files won't be overwritten).
Also creates `README.md` if it doesn't exist.

#### 3.5-2. Fill Content

Based on Phase 1 scan results, fill placeholders (`<to be filled>`) with actual project info:

- **`docs/structure.md`**: directory tree (main directories, no more than 15 lines) + module responsibility table
- **`docs/usage.md`**: dev environment (build/test/start commands) + common tasks
- **`docs/decisions.md`**: known key design decisions (extract from git history, README, SPEC; keep placeholders if no clear info)

#### 3.5-3. Rules

- Idempotent: existing files with non-placeholder content → skip, don't overwrite
- Only fill info confirmable from scan/code, don't speculate
- Path A (new project): generate skeleton + minimal fill (likely most remain placeholders)
- Path B (existing project): generate skeleton + fill as much as possible based on existing code and docs

---

### Phase 4: Update Agent Instruction File

Goal: let agent immediately understand "how to work in this project" in subsequent sessions.

Choose by current runtime priority:
- Codex: prioritize updating `AGENTS.md`
- Claude Code: prioritize updating `CLAUDE.md`
- If both files exist, update both
- If neither exists, create file corresponding to current runtime

#### 4-1. Write Content

Based on Phase 1 scan results, write:

```markdown
# <Project Name>

<One-line description>

## Development

- Build: `<build command>`
- Test: `<test command>`
- Start: `<dev server command>`

## Tech Stack

<Language/framework/key dependencies>

## Project Structure

<Main directories and purposes, no more than 10 lines>

## Code Style

<Discovered style conventions, omit this section if no explicit config>
```

#### 4-2. Update Rules

- Existing content non-dev-flow produced parts → keep unchanged
- Has section but info outdated → update
- Target file doesn't exist → generate entirely
- **Don't write mode/phase** — managed by STATUS.yaml + hooks
- Has `.cursorrules` / `.windsurfrules` → read and integrate

---

### Phase 5: Output Confirmation

```
[dev-flow] Initialization Complete
━━━━━━━━━━━━━━━━━━━━━━
Project Name: <name>
Dev Mode: <mode>
Current Phase: <phase>
Iteration Version: v<N>
Auto-fixed: <N> items
Needs Confirm: <N> items (handled)
Agent Instructions: Updated

Next Step: <corresponding command>
```

## Idempotency

- `/init` can be executed repeatedly
- Existing directories won't be deleted or have content overwritten
- STATUS.yaml will update according to actual situation
- Persistent docs (`docs/`): existing files with non-placeholder content won't be overwritten
- Agent instruction file only updates project info paragraphs, doesn't affect other content
- Re-scans and validates each execution

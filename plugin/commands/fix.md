---
description: Auto-read unclosed issues and fix them
allowed-tools: Agent, Bash, Read, Write, Edit
---

# FIX — Auto-fix Unclosed Issues

## Pre-checks

1. Confirm `.dev-doc/` exists
2. Confirm STATUS is DEV
3. Scan `.dev-doc/issue/` directory, confirm unclosed issues exist (i.e., `issue_*.md` without `closed_` prefix)
4. If no unclosed issues, tell user "no issues to fix currently" and exit

## Mode Detection
doc_root obtained via `dow status --field doc_root` (no need to manually detect branch mode).

## Execution Steps

1. List all unclosed issues: `dow issue --list`

2. Read each issue file content one by one

3. Declare work association: `dow claim <ISSUE-ID>...` (pass all issue IDs to fix, guard allows writes based on this)

4. Generate project context: `dow inbox context`

5. Launch independent Agent to fix each issue (if issues have no dependencies, can parallelize)

6. After fix complete, verify and close issue

7. Release claim: `dow claim --revoke`

## Agent Dispatch (Isolation Template)

**Launch independent fix subagent for each unclosed issue. Dispatch by current runtime: Claude Code uses `Agent`, Codex uses `spawn_agent`. Subagent prompt must use following content:**

```
description: "FIX - Fix issue: <issue title>"
prompt: `You are a senior developer. Your task is to fix the following issue.

## Issue Content

<paste complete issue file content>

## Related Specs

<extract parts from SPEC.md related to this issue>

## Project Context

<execute dow inbox context output, paste as-is>

## Fix Requirements

1. Locate root cause, don't just fix surface symptoms
2. Fixed code must comply with technical specs in SPEC.md
3. Must actually run verification after fix (start service/execute command/run tests)
4. Ensure fix doesn't introduce new problems (regression)
5. Minimize fix scope, don't refactor unrelated code along the way

## Output Format

Conclusion: Fixed / Cannot fix
Modified files: <list modified file paths>
Verification method: <how to verify fix success>
Reason: <if cannot fix, explain reason and suggestions>

## Prohibited

- Don't read unrelated historical files
- Don't modify code unrelated to issue
- Don't add new features not required by SPEC
- Don't modify code related to other issues (avoid conflicts)
- Prohibited to write to system temp directories; project-internal tmp and temp both allowed, prioritize existing directories, new projects default to tmp`
```

## Input Isolation Rules

| Allowed Input | Prohibited Input |
|---------------|------------------|
| That issue's complete content | Other issues' content |
| Related parts from SPEC.md | Dev process conversation history |
| Related task descriptions from task/ | Unrelated historical records |
| Project context (context.sh output) | PRD.md |

## Result Handling

- **Fixed** → Check corresponding item in issue file as `[x]`, fill in fix description in fix field. When all items in file are `[x]`, rename with `closed_` prefix (e.g., `issue_test_2026-05-15_1.md` → `closed_issue_test_2026-05-15_1.md`)

- **Cannot fix** → Keep issue open, report reason and suggestions to user

## audit Mode

When in `audit/<original_mode>` (e.g., `audit/quick`), /fix behavior unchanged, but completion prompt different:

- After all issues fixed, prompt user to execute `/iterate`
- After iterate completes will auto-restore original mode (no need for manual `/mode`)
- Typical flow: audit finds issues → `/fix` fixes → `/iterate` → auto-restore

## After Completion

Summarize handling results for all issues:
```
[dev-flow] Issue Fix Report
━━━━━━━━━━━━━━━━━━━━━━
Fixed: N items
Cannot fix: M items

Details:
  ✓ <issue-1>: <one-sentence fix description>
  ✓ <issue-2>: <one-sentence fix description>
  ✗ <issue-3>: <cannot fix reason>

Next step suggestion: <if unfixable issues exist, give suggestions>
```

## Why Independent Agent per Issue

- Avoid fixes interfering with each other (one fix introduces another bug)
- Isolate context, let each Agent focus on single problem
- Support parallel fixes, improve efficiency
- Fix failure doesn't affect other issues' handling

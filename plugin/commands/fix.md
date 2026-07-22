---
description: User-triggered workflow to automatically fix open issues
disable-model-invocation: true
allowed-tools: Agent, Bash, Read, Write, Edit
---

# FIX — User-triggered automatic issue repair

Only run this workflow when the user explicitly invokes `/fix`. Do not invoke
it automatically from hooks, `/test`, `/check`, task completion, or issue
creation.

## Execution

1. List open issues with `dow issue list`. The CLI resolves the active branch's
   document root. If there are no open issues, report that and stop.

2. Read each issue with `dow issue show <ISSUE-ID>` and understand its scope,
   reproduction steps, affected files, and any related task or specification.

3. Claim every issue before making project changes:

   ```bash
   dow claim <ISSUE-ID>...
   ```

4. Fix each claimed issue. Locate the root cause, keep the change scoped to
   the issue, and run appropriate verification. Bug fixes require a regression
   test; if that is technically infeasible, explain the alternative
   verification and ask the user before closing the issue.

5. For a verified fix, record the result and close the issue:

   ```bash
   dow issue update <ISSUE-ID> --fix "<concise fix summary>"
   dow issue close <ISSUE-ID>
   ```

   `dow issue close` checks the issue, renames a fully closed issue file, and
   revokes its claim. Do not edit the issue checkbox or rename issue files
   manually. If update or close fails, do not report the issue as fixed.

6. Leave unfixable issues open. Report the reason, verification performed,
   and the suggested next step.

## Completion Report

Summarize every issue handled:

```text
[dev-flow] Issue Fix Report
Fixed: N
Cannot fix: M
Blocked: K

Details:
  ✓ <issue-id>: <one-sentence fix and verification summary>
  ✗ <issue-id>: <reason and next step>
  ! <issue-id>: <claim or workflow blocker>
```

## Audit Mode

When the mode is `audit/<original_mode>` (for example, `audit/quick`), the fix
workflow is unchanged. After all issues are fixed, prompt the user to run
`/iterate`; `/iterate` restores the original mode after completion.

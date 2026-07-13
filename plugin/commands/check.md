---
description: Check if development work is synced to .dev-doc documentation
allowed-tools: Bash, Read
---

# CHECK — Doctor compatibility entry

## Execution Method

`/check` is a workflow alias for the current `dow doctor` command:

```bash
dow doctor
```

`dow doctor` performs the current structure, schema and documentation checks.
Use `dow doctor --fix` when automatic repair is explicitly requested.

## Notes

- `dow check` and `dow validate` are not current CLI commands.
- Doctor may initialize missing workflow directories or `.gitignore` entries
  during its normal checks. Do not describe it as strictly read-only.

---
description: Check if development work is synced to .dev-doc documentation
allowed-tools: Bash, Read
---

# CHECK — Documentation Sync Check

## Execution Method

Run script directly, display output:

```bash
dow lint
```

Script auto-checks CHANGELOG, task completion vs phase match, issue status, code changes vs doc update time, phase required files. Agent only needs to run and display results.

## Notes

- This is read-only check, doesn't modify any files
- Outputs suggestions but doesn't auto-execute

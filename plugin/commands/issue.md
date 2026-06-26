---
description: Manually create issue
allowed-tools: Bash, Read, Write, Edit, AskUserQuestion
---

# ISSUE — Manual Issue Creation

## Mode Detection

`DOC_ROOT` obtained via `dow status --field doc_root`.

## Execution Steps

### 1. Collect Information

Ask user (can get from parameters):
- Issue title
- Severity (P0/P1/P2)
- Location found (file_path:line_number)
- Description

### 2. Determine File

Check if today's `other` source issue file already exists:

```bash
dow issue list | grep 'other'
```

- If exists and file has reasonable issue count (<10) → append to existing file
- Otherwise → create new file

### 3. Create Issue

```bash
dow issue create --title "..." --severity P1 --location "file:line" --desc "..." --source other
```

Also supports stdin JSON:
```bash
echo '{"title":"bug","severity":"P0","location":"main.rs:10","desc":"crash on startup"}' | dow issue create
```

### 4. Write Format

Execute `dow issue schema` to get structured format definition.

### 5. Prompt Next Step

```
[dev-flow] Issue created: .dev-doc/issue/<filename>
Need to fix immediately? Execute /fix to auto-fix unclosed issues.
```

## Append to Existing File

If appending to existing file:
1. Read existing file's `nums` value
2. New issue number is `I<nums+1>`
3. Append checkbox item to file end
4. Update `nums` in frontmatter

## Notes

- Main agent executes directly, doesn't launch subagent
- source fixed as `other` (distinguish from test/devtest auto-creation)
- After creation doesn't auto-fix, user decides whether to /fix

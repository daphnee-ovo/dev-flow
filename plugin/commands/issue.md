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

Also supports stdin JSON. The input can be one object or an array; an array is
created as one batch and receives IDs in input order:
```bash
echo '{"title":"bug","severity":"P0","location":"main.rs:10","desc":"crash on startup","reproduce":"run command","source":"other"}' | dow issue create
echo '[{"title":"bug 1","severity":"P1","location":"a.rs:1","desc":"...","reproduce":"...","source":"other"},{"title":"bug 2","severity":"P1","location":"b.rs:2","desc":"...","reproduce":"...","source":"other"}]' | dow issue create
```

The required fields are `title`, `severity`, `location`, `desc`, `reproduce`
and `source`. Creation also accepts optional string arrays `files_modify` and
`files_create`, either as JSON fields or CLI flags:

```bash
dow issue create --files-modify "src/a.rs,src/b.rs" --files-create "tests/a.rs"
```

Use `dow issue schema` for the authoritative field definition. `files_test`
is a Task field and is not an ISSUE field.

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

## Updating Issues

Array fields (`files_modify`, `files_create`) support incremental syntax:
```bash
dow issue update I001 --files-modify "+new.rs,-old.rs"
```
Without `+`/`-` prefix = full replacement.

## Notes

- Main agent executes directly, doesn't launch subagent
- Manual creation normally uses `source: other`; test failures use
  `source: test` and are created by `dow test`.
- After creation doesn't auto-fix, user decides whether to /fix

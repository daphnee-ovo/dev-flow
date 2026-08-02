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
dow issue create --title "..." --severity P1 --location "file:line" --desc "..." --source other \
  --file '{"modify":["src/bug.rs"]}'
```

Also supports stdin JSON. The input can be one object or an array; an array is
created as one batch and receives IDs in input order:
```bash
echo '{"title":"bug","severity":"P0","location":"main.rs:10","desc":"crash on startup","reproduce":"run command","source":"other","files":{"modify":["main.rs"]}}' | dow issue create
echo '[{"title":"bug 1","severity":"P1","location":"a.rs:1","desc":"...","reproduce":"...","source":"other","files":{"modify":["a.rs"]}},{"title":"bug 2","severity":"P1","location":"b.rs:2","desc":"...","reproduce":"...","source":"other","files":{"create":["b.rs"]}}]' | dow issue create
```

The required fields are `title`, `severity`, `location`, `desc`, `reproduce`,
`source`, and `files`. Within `files`, `create` and `modify` are individually
optional, but at least one must contain a non-empty path:

Create and update failures report all missing or invalid fields in one response.
JSON batch errors include the record index, and malformed JSON reports its line
and column. Use 'dow issue schema' for the authoritative field names, types, and
allowed values.

```bash
dow issue create --file '{"modify":["src/a.rs","src/b.rs"],"create":["tests/a.rs"]}'
```

Use `dow issue schema` for the authoritative field definition. `files_test`,
flat `files_modify`/`files_create`, and `--files-*` flags are not accepted.

### 4. Write Format

Execute `dow issue schema` to get structured format definition.

### 5. Prompt Next Step

```
[dev-flow] Issue created: .dev-doc/issue/<filename>
To repair open issues, explicitly invoke /fix when ready.
```

## Append to Existing File

If appending to existing file:
1. Read existing file's `nums` value
2. New issue number is `I<nums+1>`
3. Append checkbox item to file end
4. Update `nums` in frontmatter

## Updating Issues

Nested `files` arrays support incremental syntax:
```bash
dow issue update I001 --file '{"modify":["+new.rs","-old.rs"]}'
```
Without `+`/`-` prefix = full replacement. An update cannot remove the last
create/modify path.

The `fix` field is not accepted during issue creation. After resolving the
issue, record the resolution before closing it:

```bash
dow issue update I001 --fix "Describe the resolution"
dow issue close I001
```

Closing an issue without a recorded `fix` fails and repeats this
resolve → update → close sequence.

## Notes

- Main agent executes directly, doesn't launch subagent
- Manual creation normally uses `source: other`; test failures use
  `source: test` and are created by `dow test`.
- Creation does not auto-fix; the user explicitly invokes /fix when repair is wanted.

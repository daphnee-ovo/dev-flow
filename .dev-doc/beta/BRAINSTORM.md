# Brainstorm Notes — Clear CLI Input Validation Diagnostics

**Date**: 2026-08-02

## Background & Purpose

`dow task create` and `dow issue create` currently stop at the first missing
field. This is especially costly for stdin JSON: Serde reports one missing
field at a time, so users must repeatedly edit and retry the same payload.
The update commands and nested `--file` JSON have related, fragmented error
messages.

The goal is to make the CLI identify all actionable input problems in one
attempt, explain the expected shape and allowed values, and preserve the
existing behavior for valid input.

## Key Decisions

| Decision Point | Choice | Rationale |
|----------------|--------|-----------|
| Initial surface | CLI commands only | Dashboard API and hook protocols have separate contracts and are explicitly deferred. |
| Commands | `task create/update` and `issue create/update` | These commands share the multi-field and nested-JSON failure pattern. |
| Input sources | CLI flags, stdin JSON, and `--file` JSON | All user-facing forms should provide the same validation quality. |
| Error strategy | Aggregate diagnostics before returning | Users should not need a fix-and-retry loop for each missing field. |
| JSON strategy | Validate JSON values before business deserialization | Ordinary Serde deserialization stops at the first structural error. |
| Task schema | Enforce the existing `type`, `priority`, and `complexity` schema values | `task create` currently fails to reject invalid `type` and `priority` values. |
| Issue `fix` lifecycle | Record `fix` after resolving the issue and before closing it | A create payload describes the problem; resolution belongs to `update`, followed by `close`. |
| Compatibility | Keep valid input and successful output unchanged | The change targets diagnostics and schema consistency, not the file format or success contract. |

## Design Approach

### Architecture

Add a small shared diagnostic/formatting layer under the command modules. It
will collect ordered field diagnostics without owning task or issue business
rules. Each resource command retains its own schema validator and translates
CLI flags, stdin JSON, and nested `--file` JSON into the validator's input.

JSON input is first parsed as `serde_json::Value`. The validator checks the
container shape, allowed keys, required keys, value types, enum values, and
nested file-scope constraints. Only a fully valid value is converted to the
existing business structure.

### Components

- **Diagnostic collector**: stores stable, path-aware messages and renders one
  `Input validation failed` error with all findings and a schema hint.
- **Task validator**: validates create fields (`title`, `type`, `priority`,
  `refs`, `files`, `depends_on`, `parallel`, `complexity`, `done_when`) and
  update fields, including task enums and file scope.
- **Issue validator**: validates create fields (`title`, `severity`,
  `location`, `desc`, `reproduce`, `source`, `files`) and update fields,
  including issue enums and file scope.
- **Input adapters**: normalize CLI names such as `--done-when` to JSON paths
  such as `done_when`, and report JSON array item paths such as `[1].priority`.
- **Schema/document hints**: point users to `dow task schema` or
  `dow issue schema` and show the relevant CLI/JSON shape when needed.

### Data Flow

1. Select exactly one input source: CLI flags or stdin JSON.
2. Report an explicit source error when neither source is supplied, or a
   conflict error when both are supplied.
3. Parse JSON syntax and report line/column when syntax is invalid.
4. For an object or array, validate every record and collect all diagnostics.
5. Normalize valid nested file lists and apply existing brace expansion.
6. Run existing business constraints, including non-empty create/modify file
   scope and valid enum values.
7. If any diagnostic exists, return without creating directories or writing
   files. Otherwise use the existing creation/update path and output.

### Error Handling

- Missing fields are reported together, with both the CLI option and JSON path
  where they differ.
- Type errors identify the exact path and expected type, including array item
  types.
- Unknown keys are reported together with the accepted keys.
- Invalid enum values include the received value and the allowed values.
- `files` errors explain that it must be an object and that at least one
  non-empty `create` or `modify` list is required; optional `test` remains
  task-only.
- Batch JSON diagnostics include the record index. A single invalid record
  prevents the entire batch from being written.
- A `fix` field in issue creation reports the lifecycle explicitly: resolve
  the issue first, then run `dow issue update <id> --fix "..."` to record the
  resolution, and finally run `dow issue close <id>`.
- Closing an issue without a recorded fix uses the same resolve → update →
  close guidance.
- Error output remains on stderr with the existing failure behavior; valid
  success output is unchanged.

## Constraints & Boundaries

- Do not change Dashboard API validation, hook JSON protocols, or unrelated
  single-purpose commands in this iteration.
- Do not change the Markdown task/issue file format.
- Do not silently accept legacy flat file fields or unknown JSON fields.
- Do not add new non-empty requirements for fields whose current contract
  permits an empty string/list, except for the existing file-scope rule.
- Add regression coverage for aggregation and schema alignment before closing
  the implementation task.
- Update the task command documentation so its batch JSON example contains all
  fields required by the actual schema.

## Next Steps

After user review, enter `/spec` to define the concrete Rust module boundaries,
diagnostic data model, and focused regression tests before implementation.

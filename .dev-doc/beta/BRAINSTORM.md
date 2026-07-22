# Brainstorm Notes — Nested File Scope Input for Tasks and Issues

**Date**: 2026-07-23

## Background & Purpose

The task schema already describes file scope as a nested `files` object, but
the task and issue input parsers still accept flat `files_modify`,
`files_create`, and `files_test` fields and the corresponding `--files-*`
flags. This mismatch makes agents retry malformed input and wastes context
tokens.

The change aligns the public create/update input contract with the nested
schema while preserving the existing Markdown and archive output formats.

## Key Decisions
| Decision Point | Choice | Rationale |
|----------------|--------|-----------|
| Public file input shape | Nested `files` object in JSON and one `--file` JSON option | Matches the schema and gives agents one predictable interface |
| Legacy flat input | Remove `files_*` JSON fields and `--files-*` flags | Avoid two competing contracts and future schema drift |
| Task file categories | `create`, `modify`, and optional `test` | Preserves the existing task model |
| Issue file categories | `create` and `modify` only | Keeps issue semantics unchanged; issues do not gain task test scope |
| Create validation | `create` and `modify` are individually optional, but at least one must be present; `test` is optional | Allows create-only or modify-only scope while preventing an unscoped item |
| Update validation | `files` is optional when other fields are updated; when supplied, it follows the same create/modify invariant | Preserves partial updates without accepting an unusable file-only payload |
| Generated artifacts | Keep existing task and issue Markdown representations | Downstream claim, dashboard, archive, and guard behavior remain stable |
| CLI versus stdin shape | `--file` receives the file object itself; stdin JSON wraps it under top-level `files` | Removes ambiguity while keeping each transport idiomatic |
| Public JSON output | Task and issue detail output uses nested `files` | Keeps read surfaces aligned with the input schema; Markdown remains unchanged |
| Mixed input sources | Reject simultaneous CLI and stdin payloads | Prevents silent precedence and hidden agent errors |
| Batch validation | Validate every item before writing any item | Avoids partial task/issue creation |

## Design Approach

### Architecture

Normalize input at the command boundary. Task and issue create/update payloads
will use explicit nested input structs, while rendering continues to use the
existing canonical data needed by the Markdown format. Shared file-list
operations such as brace expansion and incremental updates remain reusable;
storage and downstream readers do not need a format migration.

### Components

- `cli.rs`: replace the flat file flags with `--file <JSON-object>` on task
  and issue create/update arguments.
- `task.rs`: deserialize `files.create`, `files.modify`, and `files.test`,
  validate the create/modify invariant after normalization, expand braces,
  retain incremental update behavior, and emit nested detail JSON.
- `issue.rs`: deserialize `files.create` and `files.modify`, apply the same
  validation rule, emit nested detail JSON, and continue rendering
  `files_modify`/`files_create` in issue Markdown.
- Schema output: describe the nested object and its conditional requirement
  explicitly instead of exposing flat fields.
- Guard/claim guidance: use the nested `--file` syntax in generated commands
  and error messages.
- Tests and documentation: cover CLI JSON, stdin JSON, update semantics,
  validation errors, schema/show output, mixed-input rejection, batch
  atomicity, and unchanged generated artifacts.

### Data Flow

1. Parse scalar CLI flags and the single `--file` JSON string, or read the
   nested `files` object from stdin JSON. If both sources contain input,
   reject the command instead of choosing a silent precedence.
2. Reject unknown file keys, malformed JSON, missing required categories, and
   the legacy flat input names.
3. Normalize lists and expand brace expressions. A create payload must have
   at least one non-empty `create` or `modify` path; `test` alone is invalid.
4. For updates, apply incremental operations where applicable. A supplied
   `files` object must include a non-empty `create` or `modify` operation and
   must not leave the resulting item without any create/modify path.
5. Validate every batch item before passing any normalized data to the
   existing task/issue creation or update/rendering code.

### Error Handling

- Invalid `--file` JSON reports a direct parse error with the expected object
  shape.
- A create payload with neither `files.create` nor `files.modify` is rejected.
- Empty arrays and values removed by normalization do not satisfy the
  create/modify requirement.
- A supplied update `files` object with neither `create` nor `modify` is
  rejected; updates to unrelated scalar fields do not require `files`.
- Task-only `files.test` is rejected for issues.
- An update cannot remove the last create/modify path from an existing item.
- Unknown keys and legacy flat fields are rejected rather than silently
  ignored.
- Simultaneous CLI and stdin input is rejected.
- Mixed-validity batches fail before any item is written.
- Validation occurs before any task or issue file is written.

## Constraints & Boundaries

- No change to generated task Markdown, issue Markdown, SQLite archive data,
  dashboard parsing, claim scope checks, or guard behavior.
- No compatibility alias for `files_modify`, `files_create`, `files_test`, or
  `--files-*` after this change.
- `--file` carries one JSON object such as `{"modify":["src/a.rs"]}`;
  stdin JSON uses `{"files":{"modify":["src/a.rs"]}}`. Repeated file
  options are not part of this contract.
- Issue input does not accept a `test` category.
- Public JSON detail output uses nested `files`; generated Markdown keeps its
  existing task/issue field spelling.
- Documentation and generated agent/plugin artifacts must be synchronized
  with the source command definitions.

## Next Steps

After review, create and claim a dedicated DEV task, then implement the
parser/schema/documentation changes and focused regression tests. Run the
task-level verification before marking the task complete; request the full
project TEST phase separately.

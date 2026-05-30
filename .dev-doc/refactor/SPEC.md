# SPEC: plugin audit and conformance iteration

## 1. Goal

This iteration audits the whole dev-flow plugin and fixes defects that make the plugin drift from its own rules, Claude Code usage, Codex plugin structure, hook behavior, or documented command flow.

The target is not to add a larger process. The target is to keep dev-flow lightweight while making the existing constraints accurate, testable, and synchronized with the current project.

## 2. Scope

Must fix:

- Plugin package metadata and documented structure must match files that actually exist.
- Claude Code and Codex entry files must describe the correct runtime behavior without contradicting each other.
- Hook behavior must be testable and must not block harmless read-only commands.
- Temporary-file policy must allow project-local `tmp/` and `temp/`; follow the directory already used by the project, and default to `tmp/` for new projects.
- Command docs must match script behavior, especially `/iterate`.
- Current dev-doc state must describe the v2.6.0 audit iteration, not the archived v2.5.0 work.

Non-goals:

- Do not introduce a large controller or new external service.
- Do not add GUI-only workflow features.
- Do not redesign all command prompts if a local consistency fix is enough.

## 3. Evidence From Audit

Confirmed findings:

- `README.md` and `README.zh-CN.md` list removed hook scripts: `check-task-completion.sh`, `check-doc-sync.sh`, `check-phase-completion.sh`, `update-status.sh`.
- `commands/iterate.md` says active task files remain unarchived, while `scripts/commands/iterate.sh` now archives `task_*.md` once all tasks are complete.
- `dev-doc/refactor/SPEC.md` still described v2.5.0 and referenced `task_2026-05-25_1.md` as active, but v2.6.0 had no active task yet.
- Claude Code plugin docs confirm hook commands should use `${CLAUDE_PLUGIN_ROOT}` for plugin-relative scripts; current hook commands need quoted plugin-root paths.
- `.claude-plugin/plugin.json` does not list `commands/issue.md`, so the Claude plugin command set does not match README and the skill command table.
- Codex plugin validator rejects `.codex-plugin/plugin.json` field `hooks`; root `hooks.json` should remain for default discovery, but the manifest must omit unsupported fields.
- A read-only `rg` command containing a system temp path pattern was blocked by `block-system-tmp.sh`.
- User clarified the intended policy: project-local `tmp/` and `temp/` are both valid; existing project convention wins; new projects should prefer `tmp/`.
- Codex plugin validation script could not run in this environment because `yaml` is missing from the system Python.

## 4. 验收契约

- `bash scripts/commands/status.sh` reports phase `DEV` while fixes are active, then `TEST`, then `/iterate`.
- `bash scripts/commands/check.sh` reports no blocking errors before final iterate.
- Hook tests prove:
  - Write/Edit to system temp paths are blocked.
  - Read-only Bash commands that merely mention system temp paths are allowed.
  - Bash commands that write/create/copy/move into system temp paths are blocked.
  - Project-local `tmp/` and `temp/` are allowed.
- README, command docs, references, tests, and scripts consistently describe the `tmp/` or `temp/` policy.
- README project structure lists only existing hook scripts.
- Claude and Codex skill entries do not contradict their runtime.
- `/iterate` docs match script behavior for completed active tasks.
- `bash tests/test_all.sh` passes.
- A final `/iterate` archives this audit iteration and starts the next version.

## 5. Risks

- Over-tightening hooks can block legitimate work.
  - Mitigation: add explicit negative and positive hook tests.
- Allowing both project-local temp directory names can weaken enforcement if implemented loosely.
  - Mitigation: tests must prove system temp paths remain blocked while project-local `tmp/` and `temp/` are allowed.
- Plugin-standard validation may depend on external Python packages.
  - Mitigation: record the environment blocker and keep JSON validation covered by `jq`.

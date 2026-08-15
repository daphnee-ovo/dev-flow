This directory holds `.dev-doc` format specs that are **not** compiled into the `dow` binary. They are for developers to read.

Includes:

- `STATUS.md` — STATUS.yaml field spec
- `STATUS.yaml` — STATUS.yaml example
- `.drop.TEST.md` — (drop)TEST.md test-report format

Specs that **are** compiled into `dow` (`include_str!`) live in `dow/references/binary/.dev-doc/`:

- `ISSUE.md`
- `TASK-FILE.md`
- `PRD-FILE.md`
- `SPEC-FILE.md`
- `BRAINSTORM-FILE.md`
- `CHANGELOG.md`

Inject prompts and CLI help are under `dow/references/binary/` (`inject_prompt/`, `dow-help.md`).
The workflow overview is `dow/docs/dev-flow-spec.md`.

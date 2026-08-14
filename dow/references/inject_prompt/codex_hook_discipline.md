<!-- dev-flow-codex-hooks -->
## DEV FLOW Discipline

keep source and documentation file changes on hook-visible paths:

- Prefer Codex file edit/write tools for source and documentation edits.
- Do not use Bash redirection, `tee`, `sed -i`, `perl -i`, `cp`, `mv`, or ad-hoc scripts to create or modify source/docs unless the command is an explicit build or generation step.
- Before running `/iterate` or `dow iterate`, request elevated sandbox permissions first, wait for approval, and only then execute it; otherwise the sandbox may block the delivery operation.
- If Bash must generate files, limit it to `tmp/`, build artifacts, or clearly generated outputs, and state why Bash is required.
- Treat `dow hooks guard` blocks as authoritative. When blocked, stop the file-changing action and use `/task`, `/issue`, `/iterate`, or the indicated dev-flow command.
- Do not use external/direct execution channels to bypass Codex hooks.
- Treat Codex hooks as workflow guards, not as permission to use an unhooked path.

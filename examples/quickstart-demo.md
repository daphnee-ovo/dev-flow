# Quickstart Demo

This demo shows what dev-flow adds to a normal coding-agent session.

## Starting Point

You have a project and want an agent to implement a feature without losing the requirement, task list, test status, or iteration notes.

Without dev-flow, the prompt usually has to carry everything:

```text
Add a settings page. Think through requirements, make a plan, implement it, test it, and remember what changed.
```

That works for small changes, but it becomes fragile when the task spans many files or multiple sessions.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash
# follow the interactive dev-flow setup
cd your-project
```

## Run The Workflow

Ask the agent:

```text
/init
```

dev-flow creates:

```text
.dev-doc/
├── archive.db
└── main/
    ├── STATUS.yaml
    ├── CHANGELOG.md
    ├── PRD.md
    ├── SPEC.md
    ├── TEST.md
    ├── task/
    └── issue/
```

Then ask for a task breakdown:

```text
/task
```

The agent writes structured task files under `.dev-doc/task/` instead of keeping the plan only in chat.

During development, dev-flow hooks keep the agent aligned with the workflow:

- context injection reminds the agent of current phase and document rules
- guard hooks block unsafe writes such as system temp paths
- post-write hooks detect doc/code drift
- stop hooks preserve changelog context

## What You Get

After one feature loop, the repository contains both code changes and a lightweight delivery record:

```text
.dev-doc/
├── archive.db       # SQLite history queried by dow archive ...
└── main/
    ├── STATUS.yaml  # current phase, mode, version, active task
    ├── CHANGELOG.md # session-level change notes
    ├── task/        # planned work before iterate
    └── issue/       # discovered defects before iterate
```

The point is not ceremony. The point is to make long-running agent work inspectable, recoverable, and harder to fake.

For concrete sample output, inspect [sample-project](sample-project/).

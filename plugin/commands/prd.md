---
description: Start PRD phase — formalize exploration results into formal requirements document
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# PRD — Product Requirements Definition

## Execution Steps

1. Detect project mode, determine `DOC_ROOT` (see script below)
2. If `.dev-doc/` doesn't exist, create directory structure
3. Check if `BRAINSTORM.md` exists (determines working mode)
4. Read this plugin's `agents/prd-agent.md`
5. **Launch independent Agent (strictly follow template)**
6. After Agent completes, update `STATUS.yaml`

## Mode Detection

`DOC_ROOT` obtained via `dow status --field doc_root`.

## New Project Initialization

```bash
dow init --name <project_name> --mode <mode>
```

## Two Working Modes

### Mode A: Has BRAINSTORM.md (from /brainstorm)
- PRD agent reads BRAINSTORM.md, extracts structured requirements
- Identifies missing info, confirms with user one by one
- Outputs formal PRD.md

### Mode B: No BRAINSTORM.md (directly enter /prd)
- PRD agent directly explores requirements through dialogue with user
- Equivalent to original deep questioning mode
- Outputs formal PRD.md

## Agent Dispatch (Isolation Template)

**Must launch independent subagent, not allowed to pass additional context. Dispatch by current runtime: Claude Code uses `Agent`, Codex uses `spawn_agent`. Subagent prompt must use following content:**

```
description: "PRD agent - Product requirements definition"
prompt: `<read complete content of agents/prd-agent.md>

## Project Information

<only pass following content>
- User's project idea description this time (original text)
- Project name (if user mentioned)

## Existing Exploration Results

<if BRAINSTORM.md exists, paste complete content>
<if doesn't exist, write "None, need to explore requirements from scratch">

## Output Path

Create file via `dow prd create`, write to `<DOC_ROOT>/PRD.md`. Get format via `dow prd schema`.

## Prohibited

- Don't design technical solutions (that's SPEC's job)
- Don't decompose tasks (that's TASK's job)
- Don't read any existing code`
```

## Input Isolation Rules

| Allowed Input | Prohibited Input |
|---------------|------------------|
| User's project description original text | Non-requirement discussions from previous conversations |
| agents/prd-agent.md content | Existing code content |
| BRAINSTORM.md content (if exists) | Other phase docs (SPEC/TASK) |
| DOC_ROOT path | Unrelated session history |

## After Completion

1. Confirm PRD.md written
2. Update STATUS.yaml: current phase → PRD
3. Prompt user: after confirming PRD execute `/spec` to progress

---
description: Start TASK phase — task decomposition
allowed-tools: Agent, Bash, Read, Write, Edit, AskUserQuestion
---

# TASK — Task Decomposition

## Pre-checks (mode-aware)

1. Read development mode from STATUS.yaml
2. Decide input source by mode:
   - **full/quick mode**: check if `<DOC_ROOT>/SPEC.md` exists, if not stop, tell user to execute /spec first
   - **fast mode**: SPEC.md not required, use user description + project context as input
3. Generate project context: `dow hooks context`

## Complexity Routing

Main agent assesses SPEC complexity to choose execution mode:

### Low complexity (default)
Main agent decomposes tasks directly. Applies when:
- Single module or linear flow
- Interfaces are well-defined
- No circular dependencies or shared state

### High complexity (adversarial mode)
Spawn subagent adversarial mode. Signals:
- Multi-module cross-dependencies
- Interfaces not yet defined
- Shared state between modules
- Circular dependencies

## Low Complexity Execution

Main agent directly:
1. Read SPEC, understand technical solution
2. Decompose by vertical slices (end-to-end features)
3. Define done_when for each task (must be objectively verifiable)
4. Assess priorities and dependencies
5. Create tasks via `dow task create`
6. Present to user for confirmation

## High Complexity Execution (Adversarial Mode)

1. Spawn Agent A (task decomposer) — breaks down tasks based on SPEC
2. Spawn Agent B (task-challenger, read `plugin/agents/task-challenger-agent.md`) — reviews decomposition
3. Iteration loop:
   - A revises based on B's findings
   - B does full review of latest decomposition
   - Convergence: B outputs empty findings → done; max 5 rounds
4. Main agent receives final result, creates tasks via `dow task create`
5. Present to user for confirmation

## Decomposition Principles

**Prioritize vertical slices** (end-to-end features):
- "Implement user registration: form + validation + API + database + success message"
- After completion there's verifiable effect

**Infrastructure/config tasks can be split by module**:
- Scripts, templates, docs tasks split by file/component is reasonable
- Key is each task independently verifiable

**Red Flags**:
- done_when is "complete" or "implemented" → must specify as executable verification command or check condition
- Single task can't be independently verified after completion → either merge upstream/downstream or add verification means
- No dependencies marked → add them
- Circular dependencies exist → reorganize

## Task File Format

Get format via `dow task schema`. Create via `dow task create`.

## After Completion

1. Confirm task files written
2. Update STATUS.yaml: current phase → TASK
3. Prompt user: after confirming task list, STATUS will switch to DEV, start development

## Notes

- Main agent executes directly for low complexity (no subagent needed)
- High complexity uses adversarial subagents for quality assurance
- Task decomposition does not need audit step (SPEC audit already covered design quality; devtest/test verify execution)

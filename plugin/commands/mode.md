---
description: Select development mode — control flow phases
allowed-tools: Bash, Read, AskUserQuestion
---

# MODE — Development Mode Selection

## Mode Definitions

| Mode | Flow | Use Case |
|------|------|----------|
| `full` | prd → spec → task → dev → test → iterate | Brand new project, requirements unclear |
| `quick` | spec → task → dev → test → iterate | Requirements clear feature development |
| `fast` | task → dev → test → iterate | Small changes, technical solution known |
| `mvp` | spec → task → dev → iterate | Quick validation, skip TEST |
| `audit` | (Auto-triggered, cannot manually set) | Non-DEV phase issue creation auto-enters |

## Execution Method

If user specified mode (e.g., `/mode quick`), run script directly:

```bash
dow status --mode <mode>
```

If mode not specified, ask user to choose first, then run script.

## Mode-Specific Rules

### full

**prd → spec → task → dev(devtest loop) → test → iterate**

Full process, doesn't skip any phase. Suitable for brand new projects or features with unclear requirements. brainstorm is optional precursor.

Constraints:
- All tasks (regardless of priority) must all complete before iterate
- Each phase doc must meet phase-completion check standards

Next step: `/prd` (or `/brainstorm` first)

### quick

**spec → task → dev(devtest loop) → test → iterate**

Skip exploration and requirements definition, start directly from technical solution. Suitable for features with clear requirements.

Next step: `/spec`

### fast

**task → dev(devtest loop) → test → iterate**

Even skip technical solution, directly decompose tasks and start. Suitable for small changes, known solution scenarios.

Constraints:
- All tasks (regardless of priority) must all complete before iterate
- P0/P1 tasks must be implemented, P2 can be marked "postpone to next iteration" but cannot delete

Next step: `/task`

### mvp

**spec → task → dev → iterate**

Minimal validation path. Skip PRD and TEST, directly from spec to delivery. Goal is fastest runnable thing to validate idea.

Constraints:
- Output doesn't directly go to production
- If formal development needed after validation, switch mode and restart process
- After dev complete use `/iterate` to enter next round

Next step: `/spec` (or `/brainstorm` first)

## Command Availability

| Command | full | quick | fast | mvp |
|---------|:----:|:-----:|:----:|:---:|
| `/brainstorm` | ✓ | ✓ | ✓ | ✓ |
| `/prd` | ✓ | - | - | - |
| `/spec` | ✓ | ✓ | - | ✓ |
| `/task` | ✓ | ✓ | ✓ | ✓ |
| `/devtest` | ✓ | ✓ | ✓ | ✓ |
| `/fix` | ✓ | ✓ | ✓ | ✓ |
| `/test` | ✓ | ✓ | ✓ | - |
| `/check` | ✓ | ✓ | ✓ | ✓ |
| `/iterate` | ✓ | ✓ | ✓ | ✓ |
| `/status` | ✓ | ✓ | ✓ | ✓ |

`-` means this step not included in current mode flow, prompts "current mode doesn't need this step" when executed.

> Note: `/brainstorm` is free exploration tool, not required phase in any mode, but can be used anytime in all modes.

## audit Mode

audit mode is auto-triggered temporary override mode:
- Auto-enters when issue created in non-DEV phase
- Format is `audit/<original_mode>` (e.g., `audit/quick`)
- Auto-restores to original mode after iterate
- Cannot manually set via `/mode audit`

## Mode Switching

- Can switch anytime via `/mode <new_mode>`
- Existing docs kept, not deleted
- From low→high (e.g., fast → full): prompt user to supplement missing docs
- From high→low (e.g., full → fast): skip subsequent unneeded phases

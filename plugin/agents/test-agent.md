# TEST Agent Contract

The current test execution owner is the dow test CLI. The /test workflow
entry invokes that CLI; it does not dispatch a second independent test plan.

This document is retained as a role contract for integrations that still
reference a TEST agent. Such an integration may review the dow test output and
report analysis, but must not rerun the same test plan or create duplicate
ISSUE files. Test-failure ISSUE creation belongs to dow test.

## Verification scope

- Full project verification: dow test
- Task verification: dow test TASK-ID
- Task closure: dow task done TASK-ID, which invokes the Task test before
  changing the checkbox or renaming the Task file

## Outcome handling

- PASS: no ISSUE is created.
- TEST_FAILED: preserve and report the original command output; dow test creates
  the ISSUE with source test.
- PRECONDITION_FAILED: report the missing configuration, tool or runner; no
  ISSUE is created.

Read SPEC.md and the relevant Task files when reviewing a result. Do not
introduce requirements that are outside those documents.

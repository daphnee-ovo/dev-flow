---
source: other
nums: 1
---

- [x] ISSUE-I001：Guard hook returns ask instead of deny for unclaimed/out-of-scope writes
  - severity: P0
  - location：dow/src/hooks/guard.rs:check_phase_write
  - description：dow hooks guard returns ask (which Pi extension treats as allow) instead of deny when: (1) current agent has no claim, (2) file is outside claimed task's declared files scope. The check_claim_agent_mismatch and check_claim_file_scope functions use Ask which bypasses Pi isBlocked().
  - reproduce：In DEV phase with another agent holding a claim, run `dow hooks guard .gitlab-ci.yml` — returns ask instead of deny.
  - fix：Unified guard check: replaced ask with deny for no-claim, no-files, and out-of-scope writes. Added get_claims_for_current_agent in claim.rs.
  - files_modify: [dow/src/hooks/guard.rs, dow/src/core/claim.rs, dow/tests/test_guard_phase.rs, dow/tests/test_dow_branch.rs]
  - files_create: []

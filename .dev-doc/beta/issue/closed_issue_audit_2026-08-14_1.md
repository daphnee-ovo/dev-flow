---
source: audit
nums: 1
---

- [x] ISSUE-I004：session-stop revoke_by_agent uses exact string match instead of is_claim_owned_by
  - severity: P1
  - location：dow/src/core/claim.rs:280
  - description：revoke_by_agent compares agent_id with exact == but Stop hook runs in a different process tree than the original dow claim call, so detect_agent_id() returns a different pid:XXXX string. Claims are never revoked on session end.
  - reproduce：1. dow claim T001 (records pid:A as agent_id)
    2. Session ends, Stop hook fires dow hooks session-stop
    3. detect_agent_id() returns pid:B (different spawn)
    4. Exact match fails, claim.lock remains
  - fix：revoke_by_agent now uses is_claim_owned_by() for matching instead of exact string ==. Dead-process and ancestor-chain logic correctly identifies stale claims from the same session even when PID differs at Stop hook time.
  - files_modify: [dow/src/core/claim.rs, dow/src/hooks/session_stop.rs, dow/tests/test_dow_branch.rs]
  - files_create: []

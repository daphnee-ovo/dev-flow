---
source: audit
nums: 1
---

- [x] ISSUE-I003：Dashboard files section cannot expand due to SSE-triggered full DOM rebuild
  - severity: P1
  - location：dow/dashboard-frontend/app.js:1
  - description：When .dev-doc files change, the file watcher triggers SSE events which cause refreshData() → renderCurrentView() → full innerHTML rebuild. This resets all <details> elements (files sections) to closed state, making it impossible to keep them expanded. The declared-but-unused `lastDataJson` variable indicates data deduplication was planned but never implemented.
  - reproduce：1. Open dashboard
    2. Have an agent actively working (or trigger any .dev-doc file change)
    3. Click on files section <summary> to expand
    4. Within ~500ms the section closes due to SSE-triggered re-render
  - fix：Data dedup in app.js (skip re-render when data unchanged) + save/restore details open state in views.js
  - files_modify: [dow/dashboard-frontend/app.js, dow/dashboard-frontend/views.js]
  - files_create: []

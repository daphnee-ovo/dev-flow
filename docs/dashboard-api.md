# Dashboard API Reference

dev-flow dashboard exposes a RESTful API at `/api/v1/` for programmatic access to project management data. This API is designed for local integration — embedding dev-flow into other tools, editors, or automation scripts.

## Base URL

```
http://127.0.0.1:<port>/api/v1/
```

The port is auto-selected from 9800–9900 (or specified via `dow dashboard --port <port>`).

## Authentication

None. The API is local-only (binds to `127.0.0.1`).

## API Discovery

### `GET /api/v1/`

Returns a self-description of all available endpoints.

**Response 200:**
```json
{
  "name": "dev-flow",
  "api_version": "v1",
  "endpoints": [
    { "method": "GET", "path": "/api/v1/status", "description": "Get project status" },
    ...
  ]
}
```

---

## Status

### `GET /api/v1/status`

Get current project status.

**Response 200:**
```json
{
  "name": "dev-flow",
  "phase": "DEV",
  "mode": "fast",
  "exec_mode": "step",
  "version": "0.3.10",
  "goals_minor": "dashboard improvements",
  "goals_major": "",
  "updated": "2026-07-30 17:55",
  "started": "2026-06-30 11:20"
}
```

### `PATCH /api/v1/status`

Update one or more status fields. Only include fields you want to change.

**Request Body:**
```json
{
  "phase": "TEST",
  "mode": "full",
  "goals_minor": "new goal"
}
```

**Updatable fields:** `phase`, `mode`, `exec_mode`, `name`, `goals_minor`, `goals_major`

**Allowed values:**
- `phase`: `BRAINSTORM`, `PRD`, `SPEC`, `TASK`, `DEV`, `TEST`, `DONE`
- `mode`: `full`, `quick`, `fast`, `mvp`

**Response 200:** Updated status object  
**Response 400:** Invalid field value

---

## Tasks

### `GET /api/v1/tasks`

List all tasks with optional filters.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `status` | string | Filter: `pending`, `in_progress`, `done`, or `all` (default: all) |
| `priority` | string | Filter: `P0`, `P1`, `P2` |
| `type` | string | Filter: `feat`, `fix`, `refactor`, `docs`, `perf`, `test`, `style` |
| `complexity` | string | Filter: `S`, `M`, `L` |

**Response 200:**
```json
{
  "items": [
    {
      "id": "TASK-T001",
      "title": "Implement auth module",
      "type": "feat",
      "priority": "P0",
      "complexity": "M",
      "status": "pending",
      "depends_on": [],
      "done_when": ["Login endpoint passes tests"],
      "refs": "SPEC-AC-001",
      "files": {
        "create": ["src/auth.rs"],
        "modify": ["src/main.rs"],
        "test": ["tests/auth_test.rs"]
      }
    }
  ],
  "total": 12,
  "pending": 5,
  "in_progress": 2,
  "done": 5
}
```

### `POST /api/v1/tasks`

Create a new task.

**Request Body:**
```json
{
  "title": "Implement auth module",
  "type": "feat",
  "priority": "P0",
  "complexity": "M",
  "refs": "SPEC-AC-001",
  "files": {
    "create": ["src/auth.rs"],
    "modify": ["src/main.rs"],
    "test": ["tests/auth_test.rs"]
  },
  "depends_on": ["TASK-T001"],
  "parallel": false,
  "done_when": ["Login endpoint passes tests"]
}
```

**Required fields:** `title`, `type`, `priority`, `complexity`, `files` (at least one of `create` or `modify` must be non-empty)

**Response 201:** Created task object  
**Response 400:** Validation error

### `GET /api/v1/tasks/:id`

Get a single task by ID.

**Response 200:** Task object  
**Response 404:** Task not found

### `PATCH /api/v1/tasks/:id`

Update task fields. Include only fields you want to change.

**Status transitions:**
```json
{ "status": "done" }
{ "status": "pending" }
```

**Field updates:**
```json
{ "priority": "P0", "complexity": "L" }
```

**Updatable fields:** `status` (`done`/`pending`), `priority`, `type`, `complexity`

**Response 200:** Updated task object  
**Response 400:** Validation error  
**Response 404:** Task not found

### `DELETE /api/v1/tasks/:id`

Delete a task.

**Response 204:** Successfully deleted (no body)  
**Response 404:** Task not found

---

## Issues

### `GET /api/v1/issues`

List all issues with optional filters.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `status` | string | Filter: `open`, `in_progress`, `closed`, or `all` (default: all) |
| `severity` | string | Filter: `P0`, `P1`, `P2` |
| `source` | string | Filter: `test`, `audit`, `other` |

**Response 200:**
```json
{
  "items": [
    {
      "id": "ISSUE-I001",
      "title": "Login timeout unhandled",
      "severity": "P0",
      "status": "open",
      "location": "src/auth.rs:42",
      "description": "No error message on timeout",
      "reproduce": "Disconnect network, click login",
      "source": "test",
      "fix": null,
      "files": {
        "create": [],
        "modify": ["src/auth.rs"]
      }
    }
  ],
  "total": 8,
  "open": 3,
  "in_progress": 1,
  "closed": 4
}
```

### `POST /api/v1/issues`

Create a new issue.

**Request Body:**
```json
{
  "title": "Login timeout unhandled",
  "severity": "P0",
  "location": "src/auth.rs:42",
  "description": "No error message on timeout",
  "reproduce": "Disconnect network, click login",
  "source": "test",
  "files": {
    "create": [],
    "modify": ["src/auth.rs"]
  }
}
```

**Required fields:** `title`, `severity`, `files` (at least one of `create` or `modify` must be non-empty)  
**Optional with defaults:** `source` (default: `"other"`), `location`, `description`, `reproduce`

**Response 201:** Created issue object  
**Response 400:** Validation error

### `GET /api/v1/issues/:id`

Get a single issue by ID.

**Response 200:** Issue object  
**Response 404:** Issue not found

### `PATCH /api/v1/issues/:id`

Update issue fields. Include only fields you want to change.

**Close an issue (requires fix):**
```json
{ "status": "closed", "fix": "Added retry logic for timeouts" }
```

**Reopen:**
```json
{ "status": "open" }
```

**Update severity:**
```json
{ "severity": "P1" }
```

**Update fix without closing:**
```json
{ "fix": "description of the fix" }
```

**Rule:** Setting `status` to `"closed"` requires a non-empty `fix` value (either provided in the same request or already stored).

**Response 200:** Updated issue object  
**Response 400:** Validation error (e.g., close without fix)  
**Response 404:** Issue not found

### `DELETE /api/v1/issues/:id`

Delete an issue.

**Response 204:** Successfully deleted (no body)  
**Response 404:** Issue not found

---

## Documents

### `GET /api/v1/docs`

List available documents and their metadata.

**Response 200:**
```json
{
  "items": [
    { "name": "brainstorm", "exists": true, "size": 2048 },
    { "name": "prd", "exists": true, "size": 5120 },
    { "name": "spec", "exists": false }
  ]
}
```

### `GET /api/v1/docs/:name`

Get document content.

**Valid names:** `brainstorm`, `prd`, `spec`

**Response 200:**
```json
{
  "name": "spec",
  "content": "# Technical Specification\n\n...",
  "size": 5120
}
```

**Response 404:** Document does not exist

### `PUT /api/v1/docs/:name`

Create or replace a document.

**Request Body:**
```json
{
  "content": "# Technical Specification\n\n## Overview\n..."
}
```

**Response 200:** Updated document object  
**Response 400:** Invalid document name

---

## Changelog

### `GET /api/v1/changelog`

Get the full changelog content.

**Response 200:**
```json
{
  "content": "# Changelog\n\n## 2026-07-30\n- 14:30 fix-auth: Fixed auth logic\n..."
}
```

**Response 404:** CHANGELOG.md does not exist

### `POST /api/v1/changelog`

Add a new changelog entry (auto-timestamped).

**Request Body:**
```json
{
  "text": "implement-api: Implemented RESTful API v1"
}
```

**Response 201:**
```json
{
  "ok": true,
  "entry": "- 15:30 implement-api: Implemented RESTful API v1"
}
```

---

## Version

### `GET /api/v1/version`

Get current project version.

**Response 200:**
```json
{
  "version": "0.3.10",
  "branch": "beta"
}
```

---

## Events (Server-Sent Events)

### `GET /api/v1/events`

Subscribe to real-time change notifications via SSE.

**Event format:**
```
event: update
data: {"resource":"all"}
```

When a file change is detected in `.dev-doc/`, the server sends an `update` event. Clients should re-fetch the relevant resource endpoints to get current data.

**Connection management:**
- The server sends keepalive pings every 15 seconds
- If all SSE connections close and no new connection is established within 5 seconds, the server shuts down automatically

---

## Error Format

All error responses use a consistent JSON structure:

```json
{
  "error": "not_found",
  "message": "Task TASK-T099 not found"
}
```

| `error` value | HTTP Status | Meaning |
|---------------|-------------|---------|
| `validation_error` | 400 | Invalid input or field value |
| `not_found` | 404 | Resource does not exist |
| `conflict` | 409 | State conflict (e.g., already closed) |
| `internal_error` | 500 | Unexpected server error |

---

## Examples

### curl: List pending P0 tasks
```bash
curl http://127.0.0.1:9800/api/v1/tasks?status=pending&priority=P0
```

### curl: Create a task
```bash
curl -X POST http://127.0.0.1:9800/api/v1/tasks \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "Add user validation",
    "type": "feat",
    "priority": "P1",
    "complexity": "S",
    "files": { "modify": ["src/user.rs"] },
    "done_when": ["Validation rejects invalid emails"]
  }'
```

### curl: Mark task as done
```bash
curl -X PATCH http://127.0.0.1:9800/api/v1/tasks/TASK-T001 \
  -H 'Content-Type: application/json' \
  -d '{"status": "done"}'
```

### curl: Close an issue
```bash
curl -X PATCH http://127.0.0.1:9800/api/v1/issues/ISSUE-I001 \
  -H 'Content-Type: application/json' \
  -d '{"status": "closed", "fix": "Added timeout retry"}'
```

### curl: Update project phase
```bash
curl -X PATCH http://127.0.0.1:9800/api/v1/status \
  -H 'Content-Type: application/json' \
  -d '{"phase": "TEST"}'
```

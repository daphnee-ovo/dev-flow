// ─── Util ───
function esc(s) {
  if (!s) return '';
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

// ─── Confirmation Modal ───
function showConfirmModal({ title, message, confirmLabel, danger, onConfirm }) {
  const existing = document.querySelector('.confirm-modal-backdrop');
  if (existing) existing.remove();

  const backdrop = document.createElement('div');
  backdrop.className = 'confirm-modal-backdrop';
  backdrop.innerHTML = `
    <div class="confirm-modal">
      <h4 class="confirm-modal-title">${esc(title)}</h4>
      <p class="confirm-modal-message">${esc(message)}</p>
      <div class="confirm-modal-actions">
        <button class="confirm-modal-btn cancel">Cancel</button>
        <button class="confirm-modal-btn confirm ${danger ? 'danger' : ''}">${esc(confirmLabel)}</button>
      </div>
    </div>
  `;
  document.body.appendChild(backdrop);

  // Animate in
  requestAnimationFrame(() => backdrop.classList.add('visible'));

  const close = () => {
    backdrop.classList.remove('visible');
    setTimeout(() => backdrop.remove(), 200);
  };

  backdrop.querySelector('.cancel').addEventListener('click', close);
  backdrop.addEventListener('click', (e) => { if (e.target === backdrop) close(); });
  backdrop.querySelector('.confirm').addEventListener('click', () => {
    close();
    onConfirm();
  });

  // Escape key
  const onKey = (e) => { if (e.key === 'Escape') { close(); document.removeEventListener('keydown', onKey); } };
  document.addEventListener('keydown', onKey);
}

// ─── Action API ───
async function performAction(url, { successMsg } = {}) {
  try {
    const resp = await fetch(url, { method: 'POST' });
    const data = await resp.json();
    if (!data.ok) {
      showToast(data.error || 'Action failed', 'error');
      return false;
    }
    if (successMsg) showToast(successMsg, 'success');
    return true;
  } catch (e) {
    showToast('Network error: ' + e.message, 'error');
    return false;
  }
}

function showToast(message, type) {
  const existing = document.querySelectorAll('.toast');
  existing.forEach(t => t.remove());

  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;
  toast.textContent = message;
  document.body.appendChild(toast);
  requestAnimationFrame(() => toast.classList.add('visible'));
  setTimeout(() => {
    toast.classList.remove('visible');
    setTimeout(() => toast.remove(), 200);
  }, 3000);
}

// ─── Home View ───
let homeInitialized = false;
let selectedItemId = null;
const graphFilters = { status: 'active', priority: 'all' };

// Called by graph.js when a node is clicked
function selectItemFromGraph(taskId) {
  selectedItemId = taskId;
  if (appData) renderHome(appData);
}

function renderHome(data) {
  const el = document.getElementById('view-home');

  if (!homeInitialized) {
    el.innerHTML = `
      <div class="home-layout">
        <div class="sidebar">
          <div class="panel" id="status-panel"></div>
          <div class="panel" id="items-panel"></div>
        </div>
        <div class="graph-area">
          <div class="filter-bar graph-filter-bar" id="graph-filters"></div>
          <div class="graph-container" id="graph-panel"></div>
        </div>
      </div>
    `;
    homeInitialized = true;
  }

  // Status (simplified — name/phase already in nav)
  const s = data.status;
  document.getElementById('status-panel').innerHTML = `
    <h3>Overview</h3>
    <div class="status-item"><span class="label">Version</span><span class="value">${s.version || '—'}</span></div>
    <div class="status-item"><span class="label">Tasks</span><span class="value">${data.tasks.filter(t=>t.status==='done').length}/${data.tasks.length}</span></div>
    <div class="status-item"><span class="label">Issues</span><span class="value">${data.issues.filter(i=>i.status==='open').length} open</span></div>
    <div class="status-item"><span class="label">Updated</span><span class="value">${s.updated || '—'}</span></div>
  `;

  // Items panel — list view or detail view
  const items = [
    ...data.tasks.map(t => ({ ...t, kind: 'task' })),
    ...data.issues.filter(i => i.status === 'open').map(i => ({ ...i, kind: 'issue', priority: i.severity })),
  ];
  items.sort((a, b) => a.id.localeCompare(b.id));

  const panel = document.getElementById('items-panel');

  // Always render the list
  const itemsHtml = items.map(item => {
    const color = item.kind === 'task' ? (colorMap[item.priority] || '#D4C4BE') : (colorMap[item.severity] || '#D4C4BE');
    return `<li data-item-id="${item.id}"><span class="dot" style="background:${color}"></span><span class="id">${item.id}</span>${esc(item.title)}</li>`;
  }).join('');

  panel.innerHTML = `<h3>Tasks</h3><ul class="item-list">${itemsHtml || '<li style="color:var(--color-text-muted)">All clear</li>'}</ul>`;

  // If an item is selected, show overlay on top
  if (selectedItemId) {
    const item = items.find(i => i.id === selectedItemId);
    if (item) {
      const overlay = document.createElement('div');
      overlay.className = 'item-detail-overlay';
      overlay.innerHTML = `
        <button class="back-btn" id="back-to-list">← Back</button>
        <h4>${esc(item.title)}</h4>
        <div class="meta-row">
          <span class="badge badge-${(item.priority||'P1').toLowerCase()}">${item.priority}</span>
          <span class="badge" style="background:var(--color-surface-alt)">${item.complexity || 'S'}</span>
          <span class="badge" style="background:var(--color-surface-alt)">${item.status}</span>
        </div>
        ${item.refs ? `<div class="refs">refs: ${esc(item.refs)}</div>` : ''}
        ${(item.depends_on||[]).length ? `<div style="font-size:12px;color:var(--color-text-muted);margin:8px 0;">Depends: ${item.depends_on.map(esc).join(', ')}</div>` : ''}
        ${renderFilesSection(item)}
        ${(item.done_when||[]).length ? '<ul class="done-list">' + item.done_when.map(d => `<li>${esc(d)}</li>`).join('') + '</ul>' : ''}
      `;
      panel.appendChild(overlay);
      overlay.querySelector('#back-to-list').addEventListener('click', () => {
        selectedItemId = null;
        renderHome(data);
      });
    } else {
      selectedItemId = null;
    }
  }

  // Click handler for list items
  panel.querySelectorAll('.item-list li[data-item-id]').forEach(li => {
    li.addEventListener('click', () => {
      selectedItemId = li.dataset.itemId;
      renderHome(data);
    });
  });

  // Graph filters
  const gfEl = document.getElementById('graph-filters');
  gfEl.innerHTML = `
    <div class="filter-group">
      <span class="filter-label">Status</span>
      <button class="filter-btn ${graphFilters.status === 'all' ? 'active' : ''}" data-gf="status" data-value="all">All</button>
      <button class="filter-btn ${graphFilters.status === 'active' ? 'active' : ''}" data-gf="status" data-value="active">Active</button>
      <button class="filter-btn ${graphFilters.status === 'closed' ? 'active' : ''}" data-gf="status" data-value="closed">Closed</button>
    </div>
    <div class="filter-group">
      <span class="filter-label">Priority</span>
      <button class="filter-btn ${graphFilters.priority === 'all' ? 'active' : ''}" data-gf="priority" data-value="all">All</button>
      <button class="filter-btn ${graphFilters.priority === 'P0' ? 'active' : ''}" data-gf="priority" data-value="P0">P0</button>
      <button class="filter-btn ${graphFilters.priority === 'P1' ? 'active' : ''}" data-gf="priority" data-value="P1">P1</button>
      <button class="filter-btn ${graphFilters.priority === 'P2' ? 'active' : ''}" data-gf="priority" data-value="P2">P2</button>
    </div>
  `;
  gfEl.querySelectorAll('.filter-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      graphFilters[btn.dataset.gf] = btn.dataset.value;
      renderHome(data);
    });
  });

  // Filter graph data
  const filteredTasks = data.tasks.filter(t => {
    if (graphFilters.status === 'active' && t.status === 'done') return false;
    if (graphFilters.status === 'closed' && t.status !== 'done') return false;
    if (graphFilters.priority !== 'all' && t.priority !== graphFilters.priority) return false;
    return true;
  });
  const filteredIssues = data.issues.filter(i => {
    if (graphFilters.status === 'active' && i.status === 'closed') return false;
    if (graphFilters.status === 'closed' && i.status !== 'closed') return false;
    if (graphFilters.priority !== 'all' && i.severity !== graphFilters.priority) return false;
    return true;
  });

  // Graph
  renderGraph(document.getElementById('graph-panel'), filteredTasks, filteredIssues);
}

// ─── Docs View ───
function renderDocs(data) {
  const el = document.getElementById('view-docs');
  const docs = data.docs;
  const docList = [
    { key: 'brainstorm', label: 'Brainstorm' },
    { key: 'prd', label: 'PRD' },
    { key: 'spec', label: 'SPEC' },
  ].filter(d => docs[d.key] && docs[d.key].exists);

  if (docList.length === 0) {
    el.innerHTML = '<div class="docs-empty">No documents available yet.</div>';
    return;
  }

  const activeDoc = el.dataset.activeDoc || docList[0].key;
  // If activeDoc no longer exists, fall back
  const validActive = docList.find(d => d.key === activeDoc) ? activeDoc : docList[0].key;

  const navHtml = docList.map(d =>
    `<button class="${d.key === validActive ? 'active' : ''}" data-doc="${d.key}">${d.label}</button>`
  ).join('');

  const doc = docs[validActive];
  let contentHtml;
  if (doc && doc.exists && doc.content) {
    marked.setOptions({ breaks: true, gfm: true });
    contentHtml = `<div class="docs-content">${marked.parse(doc.content)}</div>`;
  } else {
    contentHtml = `<div class="docs-empty">Document is empty.</div>`;
  }

  el.innerHTML = `<div class="docs-nav">${navHtml}</div>${contentHtml}`;

  // Highlight code blocks
  el.querySelectorAll('pre code').forEach(block => {
    if (window.hljs) hljs.highlightElement(block);
  });

  // Doc nav click
  el.querySelectorAll('.docs-nav button').forEach(btn => {
    btn.addEventListener('click', () => {
      el.dataset.activeDoc = btn.dataset.doc;
      renderDocs(data);
    });
  });
}

// ─── Tasks View ───
const taskFilters = { priority: 'all', status: 'all' };

function renderTasks(data) {
  const el = document.getElementById('view-tasks');
  const tasks = data.tasks.filter(t => {
    if (taskFilters.priority !== 'all' && t.priority !== taskFilters.priority) return false;
    if (taskFilters.status !== 'all' && t.status !== taskFilters.status) return false;
    return true;
  });

  const groups = { pending: [], in_progress: [], done: [] };
  tasks.forEach(t => {
    const g = groups[t.status] || groups.pending;
    g.push(t);
  });
  const sortById = (a, b) => a.id.localeCompare(b.id);
  groups.pending.sort(sortById);
  groups.in_progress.sort(sortById);
  groups.done.sort(sortById);

  const filterBarHtml = `
    <div class="filter-bar">
      <div class="filter-group">
        <span class="filter-label">Priority</span>
        <button class="filter-btn ${taskFilters.priority === 'all' ? 'active' : ''}" data-filter="priority" data-value="all">All</button>
        <button class="filter-btn ${taskFilters.priority === 'P0' ? 'active' : ''}" data-filter="priority" data-value="P0">P0</button>
        <button class="filter-btn ${taskFilters.priority === 'P1' ? 'active' : ''}" data-filter="priority" data-value="P1">P1</button>
        <button class="filter-btn ${taskFilters.priority === 'P2' ? 'active' : ''}" data-filter="priority" data-value="P2">P2</button>
      </div>
      <div class="filter-group">
        <span class="filter-label">Status</span>
        <button class="filter-btn ${taskFilters.status === 'all' ? 'active' : ''}" data-filter="status" data-value="all">All</button>
        <button class="filter-btn ${taskFilters.status === 'in_progress' ? 'active' : ''}" data-filter="status" data-value="in_progress">In Progress</button>
        <button class="filter-btn ${taskFilters.status === 'pending' ? 'active' : ''}" data-filter="status" data-value="pending">Pending</button>
        <button class="filter-btn ${taskFilters.status === 'done' ? 'active' : ''}" data-filter="status" data-value="done">Done</button>
      </div>
    </div>
  `;

  const kanbanHtml = `
    <div class="kanban">
      ${renderKanbanCol('In Progress', groups.in_progress)}
      ${renderKanbanCol('Pending', groups.pending)}
      ${renderKanbanCol('Done', groups.done)}
    </div>
  `;

  const sortedTasks = [...tasks].sort(sortById);
  const detailHtml = sortedTasks.map(t => `
    <div class="detail-item ${t.status === 'done' ? 'status-done' : ''}" id="detail-${t.id}">
      <div class="detail-header">
        <h4><span class="task-id">${t.id}</span>${esc(t.title)}</h4>
        <div class="detail-actions">
          ${t.status === 'done'
            ? `<button class="action-btn action-reopen" data-id="${t.id}" data-kind="task" data-action="reopen">Reopen</button>`
            : `<button class="action-btn action-done" data-id="${t.id}" data-kind="task" data-action="done">Done</button>`
          }
        </div>
      </div>
      <div class="meta">
        <span class="badge badge-${(t.priority||'P1').toLowerCase()}">${t.priority}</span>
        <span class="badge" style="background:var(--color-surface-alt)">${t.complexity || 'S'}</span>
        <span class="badge" style="background:var(--color-surface-alt)">${t.type || 'feat'}</span>
      </div>
      ${t.refs ? `<div class="refs">refs: ${esc(t.refs)}</div>` : ''}
      ${(t.depends_on||[]).length ? `<div class="deps">← ${t.depends_on.map(esc).join(', ')}</div>` : ''}
      ${renderFilesSection(t)}
      ${(t.done_when||[]).length ? '<ul class="done-when">' + t.done_when.map(d => `<li>${esc(d)}</li>`).join('') + '</ul>' : ''}
    </div>
  `).join('');

  el.innerHTML = `<div class="tasks-layout">
    ${filterBarHtml}
    ${kanbanHtml}
    <div class="detail-section">${detailHtml}</div>
  </div>`;

  el.querySelectorAll('.filter-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      taskFilters[btn.dataset.filter] = btn.dataset.value;
      renderTasks(data);
    });
  });

  el.querySelectorAll('.kanban-card').forEach(card => {
    card.addEventListener('click', () => {
      el.querySelectorAll('.detail-item.highlighted').forEach(d => d.classList.remove('highlighted'));
      const target = document.getElementById('detail-' + card.dataset.id);
      if (target) {
        target.classList.add('highlighted');
        target.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    });
  });

  bindKanbanToggles(el);
  bindActionButtons(el);
}

// ─── Issues View ───
const issueFilters = { severity: 'all', status: 'all' };

function renderIssues(data) {
  const el = document.getElementById('view-issues');
  const issues = data.issues.filter(i => {
    if (issueFilters.severity !== 'all' && i.severity !== issueFilters.severity) return false;
    if (issueFilters.status !== 'all') {
      if (issueFilters.status === 'open' && i.status !== 'open' && i.status !== 'in_progress') return false;
      if (issueFilters.status === 'in_progress' && i.status !== 'in_progress') return false;
      if (issueFilters.status === 'closed' && i.status !== 'closed') return false;
    }
    return true;
  });

  const groups = { in_progress: [], open: [], closed: [] };
  issues.forEach(i => { (groups[i.status] || groups.open).push(i); });
  const sortById = (a, b) => a.id.localeCompare(b.id);
  groups.in_progress.sort(sortById);
  groups.open.sort(sortById);
  groups.closed.sort(sortById);

  const filterBarHtml = `
    <div class="filter-bar">
      <div class="filter-group">
        <span class="filter-label">Severity</span>
        <button class="filter-btn ${issueFilters.severity === 'all' ? 'active' : ''}" data-filter="severity" data-value="all">All</button>
        <button class="filter-btn ${issueFilters.severity === 'P0' ? 'active' : ''}" data-filter="severity" data-value="P0">P0</button>
        <button class="filter-btn ${issueFilters.severity === 'P1' ? 'active' : ''}" data-filter="severity" data-value="P1">P1</button>
        <button class="filter-btn ${issueFilters.severity === 'P2' ? 'active' : ''}" data-filter="severity" data-value="P2">P2</button>
      </div>
      <div class="filter-group">
        <span class="filter-label">Status</span>
        <button class="filter-btn ${issueFilters.status === 'all' ? 'active' : ''}" data-filter="status" data-value="all">All</button>
        <button class="filter-btn ${issueFilters.status === 'in_progress' ? 'active' : ''}" data-filter="status" data-value="in_progress">In Progress</button>
        <button class="filter-btn ${issueFilters.status === 'open' ? 'active' : ''}" data-filter="status" data-value="open">Open</button>
        <button class="filter-btn ${issueFilters.status === 'closed' ? 'active' : ''}" data-filter="status" data-value="closed">Closed</button>
      </div>
    </div>
  `;

  const kanbanHtml = `
    <div class="kanban">
      ${renderIssueKanbanCol('In Progress', groups.in_progress)}
      ${renderIssueKanbanCol('Open', groups.open)}
      ${renderIssueKanbanCol('Closed', groups.closed)}
    </div>
  `;

  const sortedIssues = [...issues].sort(sortById);
  const detailHtml = sortedIssues.map(i => `
    <div class="detail-item ${i.status === 'closed' ? 'status-done' : ''}" id="detail-${i.id}">
      <div class="detail-header">
        <h4><span class="task-id">${i.id}</span>${esc(i.title)}</h4>
        <div class="detail-actions">
          ${i.status === 'closed'
            ? `<button class="action-btn action-reopen" data-id="${i.id}" data-kind="issue" data-action="reopen">Reopen</button>`
            : `<button class="action-btn action-close" data-id="${i.id}" data-kind="issue" data-action="close">Close</button>`
          }
        </div>
      </div>
      <div class="meta">
        <span class="badge badge-${(i.severity||'P1').toLowerCase()}">${i.severity}</span>
      </div>
      ${i.description ? `<div class="docs-content" style="font-size:13px;margin-top:8px;color:var(--color-text);">${marked.parse(i.description)}</div>` : ''}
      ${renderIssueFiles(i)}
    </div>
  `).join('');

  el.innerHTML = `<div class="tasks-layout">
    ${filterBarHtml}
    ${kanbanHtml}
    <div class="detail-section">${detailHtml || '<div class="docs-empty">No issues</div>'}</div>
  </div>`;

  el.querySelectorAll('.filter-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      issueFilters[btn.dataset.filter] = btn.dataset.value;
      renderIssues(data);
    });
  });

  el.querySelectorAll('.kanban-card').forEach(card => {
    card.addEventListener('click', () => {
      el.querySelectorAll('.detail-item.highlighted').forEach(d => d.classList.remove('highlighted'));
      const target = document.getElementById('detail-' + card.dataset.id);
      if (target) {
        target.classList.add('highlighted');
        target.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    });
  });

  bindKanbanToggles(el);
  bindActionButtons(el);
}

// ─── Helpers ───
const KANBAN_FOLD_LIMIT = 5;

function renderKanbanCol(title, tasks) {
  const bgMap = { P0: '#FFF5F6', P1: '#FFF8F2', P2: '#F2FBF5' };
  const cards = tasks.map((t, i) => {
    const color = colorMap[t.priority] || '#D4C4BE';
    const bg = bgMap[t.priority] || '#FFF8F2';
    const hidden = i >= KANBAN_FOLD_LIMIT ? ' kanban-card-hidden' : '';
    return `<div class="kanban-card${hidden}" style="border-color:${color};background:${bg}" data-id="${t.id}">
      <div class="card-id">${t.id} · ${t.complexity || 'S'}</div>
      <div class="card-title">${esc(t.title)}</div>
    </div>`;
  }).join('');
  const empty = tasks.length === 0 ? '<div class="kanban-empty">No items</div>' : '';
  const toggle = tasks.length > KANBAN_FOLD_LIMIT
    ? `<button class="kanban-toggle" data-expanded="false">Show ${tasks.length - KANBAN_FOLD_LIMIT} more ▾</button>`
    : '';
  return `<div class="kanban-col"><h4>${title} <span class="count">${tasks.length}</span></h4>${cards}${empty}${toggle}</div>`;
}

function renderIssueKanbanCol(title, issues) {
  const bgMap = { P0: '#FFF5F6', P1: '#FFF8F2', P2: '#F2FBF5' };
  const cards = issues.map((i, idx) => {
    const color = colorMap[i.severity] || '#D4C4BE';
    const bg = bgMap[i.severity] || '#FFF8F2';
    const hidden = idx >= KANBAN_FOLD_LIMIT ? ' kanban-card-hidden' : '';
    return `<div class="kanban-card${hidden}" style="border-color:${color};background:${bg}" data-id="${i.id}">
      <div class="card-id">${i.id}</div>
      <div class="card-title">${esc(i.title)}</div>
    </div>`;
  }).join('');
  const empty = issues.length === 0 ? '<div class="kanban-empty">No items</div>' : '';
  const toggle = issues.length > KANBAN_FOLD_LIMIT
    ? `<button class="kanban-toggle" data-expanded="false">Show ${issues.length - KANBAN_FOLD_LIMIT} more ▾</button>`
    : '';
  return `<div class="kanban-col"><h4>${title} <span class="count">${issues.length}</span></h4>${cards}${empty}${toggle}</div>`;
}

function renderFilesSection(t) {
  const create = t.files_create || [];
  const modify = t.files_modify || [];
  const test = t.files_test || [];
  const total = create.length + modify.length + test.length;
  if (total === 0) return '';

  let inner = '';
  if (modify.length) inner += `<div class="files-group"><span class="files-label">modify:</span> ${modify.join(', ')}</div>`;
  if (create.length) inner += `<div class="files-group"><span class="files-label">create:</span> ${create.join(', ')}</div>`;
  if (test.length) inner += `<div class="files-group"><span class="files-label">test:</span> ${test.join(', ')}</div>`;

  return `<details class="files-section"><summary>files (${total})</summary>${inner}</details>`;
}

function renderIssueFiles(i) {
  const modify = i.files_modify || [];
  const create = i.files_create || [];
  const total = modify.length + create.length;
  if (total === 0) return '';

  let inner = '';
  if (modify.length) inner += `<div class="files-group"><span class="files-label">modify:</span> ${modify.join(', ')}</div>`;
  if (create.length) inner += `<div class="files-group"><span class="files-label">create:</span> ${create.join(', ')}</div>`;

  return `<details class="files-section"><summary>files (${total})</summary>${inner}</details>`;
}

function bindKanbanToggles(container) {
  container.querySelectorAll('.kanban-toggle').forEach(btn => {
    btn.addEventListener('click', () => {
      const col = btn.closest('.kanban-col');
      const isExpanded = col.classList.toggle('kanban-col-expanded');
      const count = col.querySelectorAll('.kanban-card-hidden').length;
      btn.textContent = isExpanded ? 'Show less ▴' : `Show ${count} more ▾`;
    });
  });
}

function bindActionButtons(container) {
  container.querySelectorAll('.action-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const { id, kind, action } = btn.dataset;
      const actionLabels = {
        done: { title: 'Mark as Done', msg: `Mark ${id} as done?`, label: 'Done', danger: false },
        reopen: { title: 'Reopen', msg: `Reopen ${id}? This will move it back to pending/open.`, label: 'Reopen', danger: true },
        close: { title: 'Close Issue', msg: `Close ${id}?`, label: 'Close', danger: false },
      };
      const cfg = actionLabels[action];
      if (!cfg) return;

      showConfirmModal({
        title: cfg.title,
        message: cfg.msg,
        confirmLabel: cfg.label,
        danger: cfg.danger,
        onConfirm: async () => {
          const url = `/api/${kind}/${encodeURIComponent(id)}/${action}`;
          await performAction(url, { successMsg: `${id} ${action} successful` });
        },
      });
    });
  });
}

// ─── Home View ───
let homeInitialized = false;
let selectedItemId = null;

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
        <div class="graph-container" id="graph-panel"></div>
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

  const panel = document.getElementById('items-panel');

  // Always render the list
  const itemsHtml = items.map(item => {
    const color = item.kind === 'task' ? (colorMap[item.priority] || '#D4C4BE') : (colorMap[item.severity] || '#D4C4BE');
    return `<li data-item-id="${item.id}"><span class="dot" style="background:${color}"></span><span class="id">${item.id}</span>${item.title}</li>`;
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
        <h4>${item.title}</h4>
        <div class="meta-row">
          <span class="badge badge-${(item.priority||'P1').toLowerCase()}">${item.priority}</span>
          <span class="badge" style="background:var(--color-surface-alt)">${item.complexity || 'S'}</span>
          <span class="badge" style="background:var(--color-surface-alt)">${item.status}</span>
        </div>
        ${item.refs ? `<div class="refs">refs: ${item.refs}</div>` : ''}
        ${(item.depends_on||[]).length ? `<div style="font-size:12px;color:var(--color-text-muted);margin:8px 0;">Depends: ${item.depends_on.join(', ')}</div>` : ''}
        ${renderFilesSection(item)}
        ${(item.done_when||[]).length ? '<ul class="done-list">' + item.done_when.map(d => `<li>${d}</li>`).join('') + '</ul>' : ''}
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

  // Graph
  renderGraph(document.getElementById('graph-panel'), data.tasks, data.issues);
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
function renderTasks(data) {
  const el = document.getElementById('view-tasks');
  const tasks = data.tasks;

  const groups = { pending: [], in_progress: [], done: [] };
  tasks.forEach(t => {
    const g = groups[t.status] || groups.pending;
    g.push(t);
  });

  const kanbanHtml = `
    <div class="kanban">
      ${renderKanbanCol('In Progress', groups.in_progress)}
      ${renderKanbanCol('Pending', groups.pending)}
      ${renderKanbanCol('Done', groups.done)}
    </div>
  `;

  const detailHtml = tasks.map(t => `
    <div class="detail-item ${t.status === 'done' ? 'status-done' : ''}" id="detail-${t.id}">
      <h4><span class="task-id">${t.id}</span>${t.title}</h4>
      <div class="meta">
        <span class="badge badge-${(t.priority||'P1').toLowerCase()}">${t.priority}</span>
        <span class="badge" style="background:var(--color-surface-alt)">${t.complexity || 'S'}</span>
        <span class="badge" style="background:var(--color-surface-alt)">${t.type || 'feat'}</span>
      </div>
      ${t.refs ? `<div class="refs">refs: ${t.refs}</div>` : ''}
      ${(t.depends_on||[]).length ? `<div class="deps">← ${t.depends_on.join(', ')}</div>` : ''}
      ${renderFilesSection(t)}
      ${(t.done_when||[]).length ? '<ul class="done-when">' + t.done_when.map(d => `<li>${d}</li>`).join('') + '</ul>' : ''}
    </div>
  `).join('');

  el.innerHTML = `<div class="tasks-layout">
    ${kanbanHtml}
    <div class="detail-section">${detailHtml}</div>
  </div>`;

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
}

// ─── Issues View ───
function renderIssues(data) {
  const el = document.getElementById('view-issues');
  const issues = data.issues;

  const groups = { open: [], closed: [] };
  issues.forEach(i => { (groups[i.status] || groups.open).push(i); });

  const kanbanHtml = `
    <div class="kanban">
      ${renderIssueKanbanCol('Open', groups.open)}
      ${renderIssueKanbanCol('Closed', groups.closed)}
    </div>
  `;

  const detailHtml = issues.map(i => `
    <div class="detail-item ${i.status === 'closed' ? 'status-done' : ''}" id="detail-${i.id}">
      <h4><span class="task-id">${i.id}</span>${i.title}</h4>
      <div class="meta">
        <span class="badge badge-${(i.severity||'P1').toLowerCase()}">${i.severity}</span>
      </div>
      ${i.description ? `<p style="font-size:13px;margin-top:8px;color:var(--color-text);">${i.description}</p>` : ''}
    </div>
  `).join('');

  el.innerHTML = `<div class="tasks-layout">
    ${kanbanHtml}
    <div class="detail-section">${detailHtml || '<div class="docs-empty">No issues</div>'}</div>
  </div>`;

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
}

// ─── Helpers ───
function renderKanbanCol(title, tasks) {
  const bgMap = { P0: '#FFF5F6', P1: '#FFF8F2', P2: '#F2FBF5' };
  const cards = tasks.map(t => {
    const color = colorMap[t.priority] || '#D4C4BE';
    const bg = bgMap[t.priority] || '#FFF8F2';
    return `<div class="kanban-card" style="border-color:${color};background:${bg}" data-id="${t.id}">
      <div class="card-id">${t.id} · ${t.complexity || 'S'}</div>
      <div class="card-title">${t.title}</div>
    </div>`;
  }).join('');
  const empty = tasks.length === 0 ? '<div class="kanban-empty">No items</div>' : '';
  return `<div class="kanban-col"><h4>${title} <span class="count">${tasks.length}</span></h4>${cards}${empty}</div>`;
}

function renderIssueKanbanCol(title, issues) {
  const bgMap = { P0: '#FFF5F6', P1: '#FFF8F2', P2: '#F2FBF5' };
  const cards = issues.map(i => {
    const color = colorMap[i.severity] || '#D4C4BE';
    const bg = bgMap[i.severity] || '#FFF8F2';
    return `<div class="kanban-card" style="border-color:${color};background:${bg}" data-id="${i.id}">
      <div class="card-id">${i.id}</div>
      <div class="card-title">${i.title}</div>
    </div>`;
  }).join('');
  const empty = issues.length === 0 ? '<div class="kanban-empty">No items</div>' : '';
  return `<div class="kanban-col"><h4>${title} <span class="count">${issues.length}</span></h4>${cards}${empty}</div>`;
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

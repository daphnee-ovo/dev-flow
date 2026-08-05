let appData = null;
let lastDataJson = '';

// 比较数据是否变化，跳过不必要的重渲染
function dataChanged(newData) {
  const json = JSON.stringify(newData);
  if (json === lastDataJson) return false;
  lastDataJson = json;
  return true;
}

// ─── SSE Connection ───
function connectSSE() {
  const evtSource = new EventSource('/api/v1/events');
  const banner = document.getElementById('reconnecting');

  evtSource.onopen = () => { banner.classList.remove('visible'); };

  evtSource.addEventListener('update', async (e) => {
    try {
      // v1 events are lightweight notifications; re-fetch data
      await refreshData();
    } catch (err) { console.error('SSE refresh error:', err); }
  });

  evtSource.onerror = () => {
    banner.classList.add('visible');
  };
}

// ─── Data Fetching ───
async function refreshData() {
  const [statusResp, tasksResp, issuesResp, docsResp] = await Promise.all([
    fetch('/api/v1/status'),
    fetch('/api/v1/tasks'),
    fetch('/api/v1/issues'),
    fetch('/api/v1/docs'),
  ]);
  const status = await statusResp.json();
  const tasksData = await tasksResp.json();
  const issuesData = await issuesResp.json();
  const docsData = await docsResp.json();

  // Fetch doc content for existing docs
  const docs = { brainstorm: { exists: false }, prd: { exists: false }, spec: { exists: false } };
  for (const item of docsData.items) {
    if (item.exists) {
      const docResp = await fetch(`/api/v1/docs/${item.name}`);
      const docContent = await docResp.json();
      docs[item.name] = { exists: true, content: docContent.content };
    } else {
      docs[item.name] = { exists: false, content: null };
    }
  }

  const newData = {
    status,
    tasks: tasksData.items,
    issues: issuesData.items,
    docs,
  };
  if (!dataChanged(newData)) return;
  appData = newData;
  renderCurrentView();
}

// ─── Tab Navigation ───
const tabs = document.querySelectorAll('.nav-tabs button');
const views = document.querySelectorAll('.view');

function switchTab(tabName) {
  tabs.forEach(t => {
    t.classList.toggle('active', t.dataset.tab === tabName);
    t.setAttribute('aria-selected', t.dataset.tab === tabName);
  });
  views.forEach(v => v.classList.toggle('active', v.id === 'view-' + tabName));
  window.location.hash = tabName;
  renderCurrentView();
}

tabs.forEach(btn => {
  btn.addEventListener('click', () => switchTab(btn.dataset.tab));
});

// Keyboard: arrow keys between tabs
document.querySelector('.nav-tabs').addEventListener('keydown', (e) => {
  const focused = document.activeElement;
  const tabList = [...tabs];
  const idx = tabList.indexOf(focused);
  if (idx < 0) return;

  if (e.key === 'ArrowRight' && idx < tabList.length - 1) {
    tabList[idx + 1].focus();
    e.preventDefault();
  } else if (e.key === 'ArrowLeft' && idx > 0) {
    tabList[idx - 1].focus();
    e.preventDefault();
  } else if (e.key === 'Enter') {
    focused.click();
  }
});

function updateNavInfo(data) {
  const nameEl = document.getElementById('nav-project-name');
  const phaseEl = document.getElementById('nav-phase');
  if (nameEl && data.status) {
    nameEl.textContent = data.status.name || '—';
    phaseEl.textContent = `${data.status.phase || '—'} · ${data.status.mode || ''}`;
  }
}

// Hash routing
function initFromHash() {
  const hash = window.location.hash.replace('#', '') || 'home';
  switchTab(hash);
}

// ─── Render Router ───
function renderCurrentView() {
  if (!appData) return;
  updateNavInfo(appData);
  const active = document.querySelector('.view.active');
  if (!active) return;

  switch (active.id) {
    case 'view-home': renderHome(appData); break;
    case 'view-docs': renderDocs(appData); break;
    case 'view-tasks': renderTasks(appData); break;
    case 'view-issues': renderIssues(appData); break;
  }
}

// ─── Initial Load ───
async function init() {
  try {
    await refreshData();
  } catch (e) {
    console.error('Initial data load failed:', e);
  }
  initFromHash();
  connectSSE();
}

window.addEventListener('hashchange', initFromHash);
init();

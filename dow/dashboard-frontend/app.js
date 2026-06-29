let appData = null;
let lastDataJson = '';

// ─── SSE Connection ───
function connectSSE() {
  const evtSource = new EventSource('/api/events');
  const banner = document.getElementById('reconnecting');

  evtSource.onopen = () => { banner.classList.remove('visible'); };

  evtSource.addEventListener('update', (e) => {
    try {
      if (e.data === lastDataJson) return;
      lastDataJson = e.data;
      appData = JSON.parse(e.data);
      renderCurrentView();
    } catch (err) { console.error('SSE parse error:', err); }
  });

  evtSource.onerror = () => {
    banner.classList.add('visible');
  };
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
    const resp = await fetch('/api/data');
    appData = await resp.json();
  } catch (e) {
    console.error('Initial data load failed:', e);
  }
  initFromHash();
  connectSSE();
}

window.addEventListener('hashchange', initFromHash);
init();

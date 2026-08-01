const sizeMap = { S: 22, M: 30, L: 38 };
const severitySizeMap = { P0: 38, P1: 30, P2: 22 };
const colorMap = { P0: '#E85D6F', P1: '#E8A44C', P2: '#6DC08A' };
const PULSE_CLASS = 'node-pulse';

let simulation = null;
let graphNodes = [];
let graphLinks = [];
let svgEl = null;
let userHasInteracted = false;

function renderGraph(container, tasks, issues) {
  const allItems = [
    ...(tasks || []).map(t => ({ ...t, kind: 'task' })),
    ...(issues || []).map(i => ({ ...i, kind: 'issue', complexity: 'S' })),
  ];

  if (allItems.length === 0) {
    container.innerHTML = '<div class="docs-empty">No tasks or issues</div>';
    return;
  }

  // Build explicit edges
  const edges = [];
  allItems.forEach(t => {
    (t.depends_on || []).forEach(dep => {
      if (allItems.find(x => x.id === dep)) {
        edges.push({ source: t.id, target: dep, implicit: false });
      }
    });
  });

  // Compute implicit edges from file overlap (tasks + issues with files)
  const itemsWithFiles = allItems.filter(t => {
    const files = t.files || {};
    return (files.create && files.create.length) || (files.modify && files.modify.length);
  });
  const implicitEdges = computeImplicitEdges(itemsWithFiles, edges);
  const allEdges = [...edges, ...implicitEdges];

  const rect = container.getBoundingClientRect();
  const svgW = rect.width || 600;
  const svgH = rect.height || 400;

  // Initialize node positions: reuse previous or spread in center
  const prevPositions = {};
  graphNodes.forEach(n => { prevPositions[n.id] = { x: n.x, y: n.y }; });

  graphNodes = allItems.map((t, i) => {
    const prev = prevPositions[t.id];
    const angle = (2 * Math.PI * i) / allItems.length;
    const spread = Math.min(svgW, svgH) * 0.3;
    return {
      ...t,
      x: prev ? prev.x : svgW / 2 + Math.cos(angle) * spread,
      y: prev ? prev.y : svgH / 2 + Math.sin(angle) * spread,
    };
  });

  graphLinks = allEdges.map(e => ({ source: e.source, target: e.target, implicit: e.implicit || false, sharedFiles: e.sharedFiles || [] }));

  // Render SVG
  if (!svgEl || container.querySelector('svg') !== svgEl.node()) {
    container.innerHTML = '';
    userHasInteracted = false;
    svgEl = d3.select(container).append('svg').attr('width', svgW).attr('height', svgH);

    // Defs
    const defs = svgEl.append('defs');
    defs.append('marker').attr('id', 'arrow-default').attr('viewBox', '0 0 10 10')
      .attr('refX', 10).attr('refY', 5).attr('markerWidth', 5).attr('markerHeight', 5)
      .attr('orient', 'auto').append('path').attr('d', 'M0,0 L10,5 L0,10 Z').attr('fill', '#D4C4BE');
    defs.append('marker').attr('id', 'arrow-implicit').attr('viewBox', '0 0 10 10')
      .attr('refX', 10).attr('refY', 5).attr('markerWidth', 5).attr('markerHeight', 5)
      .attr('orient', 'auto').append('path').attr('d', 'M0,0 L10,5 L0,10 Z').attr('fill', '#B8A8A0');
    defs.append('marker').attr('id', 'arrow-hl').attr('viewBox', '0 0 10 10')
      .attr('refX', 10).attr('refY', 5).attr('markerWidth', 5).attr('markerHeight', 5)
      .attr('orient', 'auto').append('path').attr('d', 'M0,0 L10,5 L0,10 Z').attr('fill', '#F2A0B0');

    // Diagonal stripe patterns for closed/done nodes (one per priority color)
    Object.entries(colorMap).forEach(([key, color]) => {
      const pat = defs.append('pattern').attr('id', `stripe-${key}`)
        .attr('width', 6).attr('height', 6).attr('patternUnits', 'userSpaceOnUse')
        .attr('patternTransform', 'rotate(45)');
      pat.append('rect').attr('width', 6).attr('height', 6).attr('fill', '#FBF7F5');
      pat.append('line').attr('x1', 0).attr('y1', 0).attr('x2', 0).attr('y2', 6)
        .attr('stroke', color).attr('stroke-width', 2.5).attr('stroke-opacity', 0.8);
    });

    // Zoom container — all content goes inside this <g>
    const zoomG = svgEl.append('g').attr('class', 'zoom-container');
    zoomG.append('g').attr('class', 'edges-group');
    zoomG.append('g').attr('class', 'nodes-group');

    // Zoom: scroll wheel to zoom, left-click drag on background to pan
    const zoom = d3.zoom()
      .scaleExtent([0.3, 3])
      .filter(event => event.button !== 2)
      .on('zoom', (event) => {
        if (event.sourceEvent) userHasInteracted = true;
        zoomG.attr('transform', event.transform);
      });

    svgEl.call(zoom);

    // Resize SVG when container resizes
    const ro = new ResizeObserver(() => {
      const r = container.getBoundingClientRect();
      if (r.width > 0 && r.height > 0) {
        svgEl.attr('width', r.width).attr('height', r.height);
      }
    });
    ro.observe(container);
  }

  svgEl.attr('width', svgW).attr('height', svgH);

  // Edges
  const edgesGroup = svgEl.select('.zoom-container .edges-group');
  const edgeEls = edgesGroup.selectAll('line').data(graphLinks, d => d.source + '-' + d.target + (d.implicit ? '-impl' : ''));
  edgeEls.exit().remove();
  edgeEls.enter().append('line')
    .attr('stroke', d => d.implicit ? '#B8A8A0' : '#D4C4BE')
    .attr('stroke-opacity', d => d.implicit ? 0.35 : 0.6)
    .attr('stroke-width', 1.5)
    .attr('stroke-dasharray', d => d.implicit ? '4,3' : 'none')
    .attr('marker-end', d => d.implicit ? 'url(#arrow-implicit)' : 'url(#arrow-default)');

  // Nodes
  const nodesGroup = svgEl.select('.zoom-container .nodes-group');
  const nodeEls = nodesGroup.selectAll('g.node').data(graphNodes, d => d.id);
  nodeEls.exit().remove();

  const enter = nodeEls.enter().append('g').attr('class', 'node').style('cursor', 'grab');
  enter.each(function(d) {
    const g = d3.select(this);
    if (d.kind === 'issue') {
      g.append('rect').attr('class', 'shape');
    } else {
      g.append('circle').attr('class', 'shape');
    }
  });
  enter.append('text').attr('text-anchor', 'middle').attr('dominant-baseline', 'central')
    .attr('font-size', '11px').attr('fill', 'var(--color-text)').attr('pointer-events', 'none');

  const allNodes = nodesGroup.selectAll('g.node');
  allNodes.select('.shape').each(function(d) {
    const el = d3.select(this);
    const size = d.kind === 'issue' ? (severitySizeMap[d.severity] || 30) : (sizeMap[d.complexity] || 30);
    const priorityKey = d.kind === 'issue' ? d.severity : d.priority;
    const isClosed = d.status === 'done' || d.status === 'closed';
    const fill = isClosed
      ? `url(#stripe-${priorityKey || 'P1'})`
      : (colorMap[priorityKey] || '#D4C4BE');
    el.attr('fill', fill)
      .attr('fill-opacity', isClosed ? 0.6 : 0.85)
      .attr('stroke', 'none');
    if (d.kind === 'issue') {
      const s = size * 1.6;
      el.attr('x', -s/2).attr('y', -s/2).attr('width', s).attr('height', s).attr('rx', 4);
    } else {
      el.attr('r', size);
    }
  });
  allNodes.select('text').text(d => d.id.replace('TASK-', '').replace('ISSUE-', ''));
  allNodes.classed(PULSE_CLASS, d => d.status === 'in_progress');

  // Tooltip
  let tooltip = container.querySelector('.graph-tooltip');
  if (!tooltip) {
    tooltip = document.createElement('div');
    tooltip.className = 'graph-tooltip';
    container.appendChild(tooltip);
  }

  let resetBtn = container.querySelector('.graph-reset-btn');
  if (!resetBtn) {
    resetBtn = document.createElement('button');
    resetBtn.className = 'graph-reset-btn';
    resetBtn.textContent = 'Reset';
    resetBtn.addEventListener('click', () => { userHasInteracted = false; fitGraphToView(); });
    container.appendChild(resetBtn);
  }

  allNodes
    .on('mouseenter', function(event, d) {
      const badge = d.kind === 'issue' ? d.severity : d.priority;
      const _esc = s => s ? s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;') : '';
      tooltip.innerHTML = `<b>${d.id}</b><br>${_esc(d.title)}<br><span class="badge badge-${(badge||'P1').toLowerCase()}">${badge}</span> · ${d.status}`;
      tooltip.classList.add('visible');
    })
    .on('mousemove', function(event) {
      const rect = container.getBoundingClientRect();
      tooltip.style.left = (event.clientX - rect.left + 16) + 'px';
      tooltip.style.top = (event.clientY - rect.top - 40) + 'px';
    })
    .on('mouseleave', function() { tooltip.classList.remove('visible'); })
    .on('click', function(event, d) {
      event.stopPropagation();
      highlightChain(d.id, allNodes, edgesGroup.selectAll('line'));
      if (typeof selectItemFromGraph === 'function') selectItemFromGraph(d.id);
    });

  svgEl.on('click', () => {
    allNodes.style('opacity', 1);
    edgesGroup.selectAll('line').each(function(d) {
      d3.select(this)
        .attr('stroke', d.implicit ? '#B8A8A0' : '#D4C4BE')
        .attr('stroke-opacity', d.implicit ? 0.35 : 0.6)
        .attr('stroke-dasharray', d.implicit ? '4,3' : 'none')
        .attr('marker-end', d.implicit ? 'url(#arrow-implicit)' : 'url(#arrow-default)');
    });
  });

  // Force simulation
  if (simulation) simulation.stop();

  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  simulation = d3.forceSimulation(graphNodes)
    .force('link', d3.forceLink(graphLinks).id(d => d.id).distance(100).strength(0.4))
    .force('charge', d3.forceManyBody().strength(-200))
    .force('collide', d3.forceCollide(d => {
      const sz = d.kind === 'issue' ? (severitySizeMap[d.severity] || 30) : (sizeMap[d.complexity] || 30);
      return (d.kind === 'issue' ? sz * 0.8 : sz) + 8;
    }))
    .force('centerX', d3.forceX(svgW / 2).strength(0.05))
    .force('centerY', d3.forceY(svgH / 2).strength(0.05))
    .alphaDecay(reducedMotion ? 1 : 0.03)
    .alphaMin(0.005)
    .velocityDecay(0.4);

  simulation.on('tick', () => {
    edgesGroup.selectAll('line').each(function(e) {
      const s = graphNodes.find(n => n.id === (typeof e.source === 'string' ? e.source : e.source.id));
      const t = graphNodes.find(n => n.id === (typeof e.target === 'string' ? e.target : e.target.id));
      if (!s || !t) return;
      const tsz = t.kind === 'issue' ? (severitySizeMap[t.severity] || 30) : (sizeMap[t.complexity] || 30);
      const r = (t.kind === 'issue' ? tsz * 0.8 : tsz) + 4;
      const dx = t.x - s.x, dy = t.y - s.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      d3.select(this).attr('x1', s.x).attr('y1', s.y).attr('x2', t.x - (dx / dist) * r).attr('y2', t.y - (dy / dist) * r);
    });
    allNodes.attr('transform', d => `translate(${d.x},${d.y})`);
  });

  // Auto-fit: only on first render (before user interacts)
  simulation.on('end', () => {
    if (userHasInteracted || graphNodes.length === 0) return;
    fitGraphToView();
  });

  // Drag
  allNodes.call(d3.drag()
    .on('start', (event, d) => {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      d.fx = d.x; d.fy = d.y;
    })
    .on('drag', (event, d) => { d.fx = event.x; d.fy = event.y; })
    .on('end', (event, d) => {
      if (!event.active) simulation.alphaTarget(0);
      d.fx = null; d.fy = null;
    })
  );
}

function highlightChain(nodeId, allNodes, allEdges) {
  const related = new Set([nodeId]);
  function upstream(id) {
    graphLinks.forEach(e => {
      const sid = typeof e.source === 'string' ? e.source : e.source.id;
      const tid = typeof e.target === 'string' ? e.target : e.target.id;
      if (sid === id && !related.has(tid)) { related.add(tid); upstream(tid); }
    });
  }
  function downstream(id) {
    graphLinks.forEach(e => {
      const sid = typeof e.source === 'string' ? e.source : e.source.id;
      const tid = typeof e.target === 'string' ? e.target : e.target.id;
      if (tid === id && !related.has(sid)) { related.add(sid); downstream(sid); }
    });
  }
  upstream(nodeId);
  downstream(nodeId);

  allNodes.style('opacity', d => related.has(d.id) ? 1 : 0.15);
  allEdges.each(function(e) {
    const sid = typeof e.source === 'string' ? e.source : e.source.id;
    const tid = typeof e.target === 'string' ? e.target : e.target.id;
    const isRel = related.has(sid) && related.has(tid);
    d3.select(this)
      .attr('stroke', isRel ? '#F2A0B0' : (e.implicit ? '#B8A8A0' : '#D4C4BE'))
      .attr('stroke-opacity', isRel ? 1 : 0.08)
      .attr('stroke-dasharray', e.implicit ? '4,3' : 'none')
      .attr('marker-end', isRel ? 'url(#arrow-hl)' : (e.implicit ? 'url(#arrow-implicit)' : 'url(#arrow-default)'));
  });
}

function computeImplicitEdges(tasks, explicitEdges) {
  const explicitSet = new Set();
  explicitEdges.forEach(e => {
    explicitSet.add(e.source + '->' + e.target);
    explicitSet.add(e.target + '->' + e.source);
  });

  const implicit = [];
  for (let i = 0; i < tasks.length; i++) {
    for (let j = i + 1; j < tasks.length; j++) {
      const a = tasks[i], b = tasks[j];
      const aFiles = a.files || {};
      const bFiles = b.files || {};
      const filesA = new Set([...(aFiles.create || []), ...(aFiles.modify || [])]);
      const filesB = new Set([...(bFiles.create || []), ...(bFiles.modify || [])]);
      if (filesA.size === 0 || filesB.size === 0) continue;

      const shared = [...filesA].filter(f => f && filesB.has(f));
      if (shared.length === 0) continue;

      if (explicitSet.has(a.id + '->' + b.id) || explicitSet.has(b.id + '->' + a.id)) continue;

      // Determine direction: source depends on target (arrow: source → target)
      const [dependent, dependency] = resolveImplicitDirection(a, b, shared);
      implicit.push({ source: dependent.id, target: dependency.id, implicit: true, sharedFiles: shared });
    }
  }
  return implicit;
}

function fitGraphToView() {
  if (!svgEl || graphNodes.length === 0) return;
  let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity;
  graphNodes.forEach(n => { x0 = Math.min(x0, n.x); x1 = Math.max(x1, n.x); y0 = Math.min(y0, n.y); y1 = Math.max(y1, n.y); });
  const pad = 60;
  const svgW = +svgEl.attr('width') || 600;
  const svgH = +svgEl.attr('height') || 400;
  const bw = (x1 - x0) + pad * 2 || 1;
  const bh = (y1 - y0) + pad * 2 || 1;
  const scale = Math.min(svgW / bw, svgH / bh, 1.5);
  const tx = svgW / 2 - (x0 + x1) / 2 * scale;
  const ty = svgH / 2 - (y0 + y1) / 2 * scale;
  const zoomG = svgEl.select('.zoom-container');
  const t = d3.zoomIdentity.translate(tx, ty).scale(scale);
  zoomG.transition().duration(600).attr('transform', t);
  svgEl.call(d3.zoom().transform, t);
}

// Resolve direction for implicit dependency edge.
// Returns [dependent, dependency] — dependent depends on dependency.
// Priority: create→modify > status > ID order
function resolveImplicitDirection(a, b, sharedFiles) {
  // Rule 1 (highest): modify depends on create — if one creates a shared file and the other modifies it
  const createsA = new Set((a.files || {}).create || []);
  const createsB = new Set((b.files || {}).create || []);
  const aCreatesShared = sharedFiles.some(f => createsA.has(f));
  const bCreatesShared = sharedFiles.some(f => createsB.has(f));
  if (aCreatesShared && !bCreatesShared) return [b, a]; // b(modify) depends on a(create)
  if (bCreatesShared && !aCreatesShared) return [a, b]; // a(modify) depends on b(create)

  // Rule 2: undone/open depends on done/closed
  const aDone = a.status === 'done' || a.status === 'closed';
  const bDone = b.status === 'done' || b.status === 'closed';
  if (aDone && !bDone) return [b, a]; // b(undone) depends on a(done)
  if (bDone && !aDone) return [a, b]; // a(undone) depends on b(done)

  // Rule 3 (lowest): higher ID depends on lower ID
  return a.id < b.id ? [b, a] : [a, b];
}

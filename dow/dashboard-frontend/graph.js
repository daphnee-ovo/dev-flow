const sizeMap = { S: 22, M: 30, L: 38, XL: 46 };
const colorMap = { P0: '#E85D6F', P1: '#E8A44C', P2: '#6DC08A' };
const borderMap = { done: '#6DC08A', in_progress: '#F2A0B0', pending: '#D4C4BE', open: '#E8A44C', closed: '#6DC08A' };

let simulation = null;
let graphNodes = [];
let graphLinks = [];
let svgEl = null;

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

  // Compute implicit edges from file overlap
  const implicitEdges = computeImplicitEdges(allItems.filter(t => t.kind === 'task'), edges);
  const allEdges = [...edges, ...implicitEdges];

  // Dagre layout
  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: 'TB', nodesep: 60, ranksep: 80 });
  g.setDefaultEdgeLabel(() => ({}));
  allItems.forEach(t => g.setNode(t.id, { width: (sizeMap[t.complexity] || 30) * 2.5, height: (sizeMap[t.complexity] || 30) * 2.5 }));
  allEdges.forEach(e => g.setEdge(e.source, e.target));
  dagre.layout(g);

  const rect = container.getBoundingClientRect();
  const svgW = rect.width || 600;
  const svgH = rect.height || 400;

  // Centering offset
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  g.nodes().forEach(id => { const n = g.node(id); minX = Math.min(minX, n.x); maxX = Math.max(maxX, n.x); minY = Math.min(minY, n.y); maxY = Math.max(maxY, n.y); });
  const ox = (svgW - (maxX - minX)) / 2 - minX;
  const oy = (svgH - (maxY - minY)) / 2 - minY;

  // Create or update nodes with dagre positions as targets
  const prevPositions = {};
  graphNodes.forEach(n => { prevPositions[n.id] = { x: n.x, y: n.y }; });

  graphNodes = allItems.map(t => {
    const dagreNode = g.node(t.id);
    const targetX = dagreNode.x + ox;
    const targetY = dagreNode.y + oy;
    const prev = prevPositions[t.id];
    return {
      ...t,
      x: prev ? prev.x : targetX,
      y: prev ? prev.y : targetY,
      targetX,
      targetY,
    };
  });

  graphLinks = allEdges.map(e => ({ source: e.source, target: e.target, implicit: e.implicit || false, sharedFiles: e.sharedFiles || [] }));

  // Render SVG
  if (!svgEl || container.querySelector('svg') !== svgEl.node()) {
    container.innerHTML = '';
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

    // Zoom container — all content goes inside this <g>
    const zoomG = svgEl.append('g').attr('class', 'zoom-container');
    zoomG.append('g').attr('class', 'edges-group');
    zoomG.append('g').attr('class', 'nodes-group');

    // Zoom: scroll wheel to zoom, right-click drag to pan
    const zoom = d3.zoom()
      .scaleExtent([0.3, 3])
      .filter(event => {
        // Allow: scroll wheel (zoom) or right-click drag (pan)
        if (event.type === 'wheel') return true;
        if (event.type === 'mousedown' && event.button === 2) return true;
        if (event.type === 'mousemove' || event.type === 'mouseup') return true;
        return false;
      })
      .on('zoom', (event) => {
        zoomG.attr('transform', event.transform);
      });

    svgEl.call(zoom);
    svgEl.on('contextmenu', (event) => event.preventDefault());

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
    const size = sizeMap[d.complexity] || 30;
    const fill = d.kind === 'issue' ? (colorMap[d.severity] || '#E8A44C') : (colorMap[d.priority] || '#D4C4BE');
    el.attr('fill', fill)
      .attr('fill-opacity', 0.85)
      .attr('stroke', borderMap[d.status] || '#D4C4BE')
      .attr('stroke-width', 2.5);
    if (d.kind === 'issue') {
      const s = size * 1.6;
      el.attr('x', -s/2).attr('y', -s/2).attr('width', s).attr('height', s).attr('rx', 4);
    } else {
      el.attr('r', size);
    }
  });
  allNodes.select('text').text(d => d.id.replace('TASK-', '').replace('ISSUE-', ''));

  // Tooltip
  let tooltip = container.querySelector('.graph-tooltip');
  if (!tooltip) {
    tooltip = document.createElement('div');
    tooltip.className = 'graph-tooltip';
    container.appendChild(tooltip);
  }

  allNodes
    .on('mouseenter', function(event, d) {
      const badge = d.kind === 'issue' ? d.severity : d.priority;
      tooltip.innerHTML = `<b>${d.id}</b><br>${d.title}<br><span class="badge badge-${(badge||'P1').toLowerCase()}">${badge}</span> · ${d.status}`;
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
    .force('link', d3.forceLink(graphLinks).id(d => d.id).distance(80).strength(0.3))
    .force('charge', d3.forceManyBody().strength(-150))
    .force('collide', d3.forceCollide(d => (d.kind === 'issue' ? (sizeMap[d.complexity] || 30) * 0.8 : (sizeMap[d.complexity] || 30)) + 8))
    .force('posX', d3.forceX(d => d.targetX).strength(reducedMotion ? 1 : 0.4))
    .force('posY', d3.forceY(d => d.targetY).strength(reducedMotion ? 1 : 0.4))
    .alphaDecay(reducedMotion ? 1 : 0.05)
    .alphaMin(0.01)
    .velocityDecay(0.5);

  simulation.on('tick', () => {
    edgesGroup.selectAll('line').each(function(e) {
      const s = graphNodes.find(n => n.id === (typeof e.source === 'string' ? e.source : e.source.id));
      const t = graphNodes.find(n => n.id === (typeof e.target === 'string' ? e.target : e.target.id));
      if (!s || !t) return;
      const r = t.kind === 'issue' ? (sizeMap[t.complexity] || 30) * 0.8 + 4 : (sizeMap[t.complexity] || 30) + 4;
      const dx = t.x - s.x, dy = t.y - s.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      d3.select(this).attr('x1', s.x).attr('y1', s.y).attr('x2', t.x - (dx / dist) * r).attr('y2', t.y - (dy / dist) * r);
    });
    allNodes.attr('transform', d => `translate(${d.x},${d.y})`);
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
      const filesA = new Set([...(a.files_create || []), ...(a.files_modify || [])]);
      const filesB = new Set([...(b.files_create || []), ...(b.files_modify || [])]);
      if (filesA.size === 0 || filesB.size === 0) continue;

      const shared = [...filesA].filter(f => f && filesB.has(f));
      if (shared.length === 0) continue;

      if (!explicitSet.has(a.id + '->' + b.id) && !explicitSet.has(b.id + '->' + a.id)) {
        implicit.push({ source: a.id, target: b.id, implicit: true, sharedFiles: shared });
      }
    }
  }
  return implicit;
}

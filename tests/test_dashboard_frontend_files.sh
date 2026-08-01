#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v node >/dev/null 2>&1; then
  echo "ERROR: node is required to run the dashboard frontend regression test" >&2
  exit 1
fi

node - "$project_root" <<'NODE'
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const projectRoot = process.argv[2];

function loadFunctions(relativePath, names) {
  const filePath = path.join(projectRoot, relativePath);
  const source = fs.readFileSync(filePath, 'utf8');
  const exports = names.map(name => `this.${name} = ${name};`).join('\n');
  const context = {};
  vm.runInNewContext(`${source}\n${exports}`, context, { filename: filePath });
  return context;
}

const views = loadFunctions('dow/dashboard-frontend/views.js', [
  'renderFilesSection',
  'renderIssueFiles',
]);

const taskHtml = views.renderFilesSection({
  files: {
    create: ['src/new.rs'],
    modify: ['src/main.rs'],
    test: ['tests/new.rs'],
  },
});
assert.match(taskHtml, /files \(3\)/);
assert.match(taskHtml, /create:<\/span> src\/new\.rs/);
assert.match(taskHtml, /modify:<\/span> src\/main\.rs/);
assert.match(taskHtml, /test:<\/span> tests\/new\.rs/);

const issueHtml = views.renderIssueFiles({
  files: {
    create: ['docs/issue.md'],
    modify: ['src/issue.rs'],
  },
});
assert.match(issueHtml, /files \(2\)/);
assert.match(issueHtml, /create:<\/span> docs\/issue\.md/);
assert.match(issueHtml, /modify:<\/span> src\/issue\.rs/);

assert.strictEqual(views.renderFilesSection({ files: {} }), '');
assert.strictEqual(views.renderIssueFiles({ files: {} }), '');

const graph = loadFunctions('dow/dashboard-frontend/graph.js', ['computeImplicitEdges']);
const implicitEdges = graph.computeImplicitEdges([
  {
    id: 'TASK-T001',
    status: 'done',
    files: { create: ['src/shared.rs'], modify: [] },
  },
  {
    id: 'TASK-T002',
    status: 'pending',
    files: { create: [], modify: ['src/shared.rs'] },
  },
], []);
assert.deepStrictEqual(JSON.parse(JSON.stringify(implicitEdges)), [{
  source: 'TASK-T002',
  target: 'TASK-T001',
  implicit: true,
  sharedFiles: ['src/shared.rs'],
}]);

console.log('dashboard frontend v1 file-scope rendering: ok');
NODE

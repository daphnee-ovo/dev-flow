import * as vscode from 'vscode';
import { ChildProcess, spawn } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

let dashboardProcess: ChildProcess | null = null;
let currentPanel: vscode.WebviewPanel | null = null;

export function activate(context: vscode.ExtensionContext) {
  context.subscriptions.push(
    vscode.commands.registerCommand('dow-dashboard.open', () => {
      openDashboardPanel(context);
    })
  );

  const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 50);
  statusBar.text = '$(dashboard) Dow';
  statusBar.tooltip = 'Open Dow Dashboard';
  statusBar.command = 'dow-dashboard.open';
  statusBar.show();
  context.subscriptions.push(statusBar);
}

export function deactivate() {
  killDashboard();
}

function openDashboardPanel(context: vscode.ExtensionContext) {
  if (currentPanel) {
    currentPanel.reveal(vscode.ViewColumn.One);
    return;
  }

  currentPanel = vscode.window.createWebviewPanel(
    'dowDashboard',
    'Dow Dashboard',
    vscode.ViewColumn.One,
    { enableScripts: true, retainContextWhenHidden: true }
  );

  currentPanel.webview.html = getLoadingHtml();

  startDashboard(currentPanel);

  currentPanel.onDidDispose(() => {
    currentPanel = null;
    killDashboard();
  }, null, context.subscriptions);
}

function killDashboard() {
  if (dashboardProcess) {
    dashboardProcess.kill();
    dashboardProcess = null;
  }
}

function findDowBinary(): string | null {
  const home = process.env.HOME || process.env.USERPROFILE || '';
  const candidates = [
    path.join(home, '.local', 'bin', 'dow'),
    path.join(home, '.cargo', 'bin', 'dow'),
    '/usr/local/bin/dow',
    '/usr/bin/dow',
  ];
  for (const p of candidates) {
    if (fs.existsSync(p)) {
      return p;
    }
  }
  return null;
}

function getUserShellEnv(): NodeJS.ProcessEnv {
  const env = { ...process.env };
  const home = env.HOME || env.USERPROFILE || '';
  const extraPaths = [
    path.join(home, '.local', 'bin'),
    path.join(home, '.cargo', 'bin'),
  ];
  env.PATH = extraPaths.join(':') + ':' + (env.PATH || '');
  return env;
}

async function startDashboard(panel: vscode.WebviewPanel) {
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (!workspaceFolder) {
    panel.webview.html = getErrorHtml('No workspace folder open');
    return;
  }

  const devDocPath = path.join(workspaceFolder.uri.fsPath, '.dev-doc');
  try {
    await vscode.workspace.fs.stat(vscode.Uri.file(devDocPath));
  } catch {
    panel.webview.html = getErrorHtml(
      'No .dev-doc/ found in workspace. Run <code>dow init</code> first.'
    );
    return;
  }

  killDashboard();

  const dowBin = findDowBinary() || 'dow';
  const proc = spawn(dowBin, ['dashboard', '--no-open'], {
    cwd: workspaceFolder.uri.fsPath,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: getUserShellEnv(),
  });

  dashboardProcess = proc;

  let portResolved = false;

  proc.stderr?.on('data', (data: Buffer) => {
    const text = data.toString();
    if (!portResolved) {
      const match = text.match(/Listening on http:\/\/127\.0\.0\.1:(\d+)/);
      if (match) {
        portResolved = true;
        const port = parseInt(match[1], 10);
        panel.webview.html = getDashboardHtml(port);
      }
    }
  });

  proc.on('error', (err: Error) => {
    panel.webview.html = getErrorHtml(
      `Failed to start <code>${dowBin}</code>: ${err.message}. Is dow installed?`
    );
  });

  proc.on('exit', (code: number | null) => {
    if (!portResolved) {
      panel.webview.html = getErrorHtml(
        `dow dashboard exited with code ${code} before starting.`
      );
    }
    dashboardProcess = null;
  });

  setTimeout(() => {
    if (!portResolved) {
      panel.webview.html = getErrorHtml(
        'Timeout waiting for dow dashboard to start.'
      );
      killDashboard();
    }
  }, 10000);
}

function getDashboardHtml(port: number): string {
  return `<!DOCTYPE html>
<html>
<head>
  <style>
    body, html { margin: 0; padding: 0; width: 100%; height: 100%; overflow: hidden; }
    iframe { border: none; width: 100%; height: 100%; }
  </style>
</head>
<body>
  <iframe src="http://127.0.0.1:${port}"></iframe>
</body>
</html>`;
}

function getLoadingHtml(): string {
  return `<!DOCTYPE html>
<html>
<head>
  <style>
    body { display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0;
           font-family: var(--vscode-font-family); color: var(--vscode-foreground); }
  </style>
</head>
<body>
  <p>Starting dashboard...</p>
</body>
</html>`;
}

function getErrorHtml(message: string): string {
  return `<!DOCTYPE html>
<html>
<head>
  <style>
    body { display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0;
           font-family: var(--vscode-font-family); color: var(--vscode-errorForeground, #f44); padding: 16px; text-align: center; }
  </style>
</head>
<body>
  <p>${message}</p>
</body>
</html>`;
}

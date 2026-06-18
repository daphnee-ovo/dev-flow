#!/usr/bin/env node
"use strict";

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const http = require("http");

const REPO = "daphnee-ovo/dev-flow";
const BIN_DIR = path.join(__dirname, "..", "bin");
const pkg = require("../package.json");
const VERSION = `v${pkg.version}`;

const PLATFORM_MAP = {
  "linux-x64": "linux-x86_64",
  "linux-arm64": "linux-aarch64",
  "darwin-arm64": "darwin-arm64",
  "win32-x64": "windows-x86_64",
};

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;
  return `${platform}-${arch}`;
}

function getDownloadUrl() {
  const key = getPlatformKey();
  const mapped = PLATFORM_MAP[key];
  if (!mapped) {
    throw new Error(
      `Unsupported platform: ${key}. Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`
    );
  }
  return `https://github.com/${REPO}/releases/download/${VERSION}/dow-${VERSION}-${mapped}.tar.gz`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const get = url.startsWith("https") ? https.get : http.get;
    get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        return download(res.headers.location).then(resolve, reject);
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`Download failed: HTTP ${res.statusCode} from ${url}`));
      }
      const chunks = [];
      res.on("data", (chunk) => chunks.push(chunk));
      res.on("end", () => resolve(Buffer.concat(chunks)));
      res.on("error", reject);
    }).on("error", reject);
  });
}

async function install() {
  const url = getDownloadUrl();
  console.log(`[dev-flow] Downloading ${url}`);

  const tarball = await download(url);

  // Extract to a temp directory to avoid overwriting the Node.js wrapper (bin/dow)
  const os = require("os");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "dow-install-"));
  const tmpFile = path.join(tmpDir, "dow.tar.gz");
  fs.writeFileSync(tmpFile, tarball);

  try {
    execSync(`tar -xzf "${tmpFile}" -C "${tmpDir}"`, { stdio: "pipe" });
  } catch (e) {
    throw new Error(`Failed to extract tarball: ${e.message}`);
  }

  // Find the native binary in the extracted contents
  const srcName = process.platform === "win32" ? "dow.exe" : "dow";
  let srcPath = path.join(tmpDir, "bin", srcName);
  if (!fs.existsSync(srcPath)) {
    srcPath = path.join(tmpDir, srcName);
  }
  if (!fs.existsSync(srcPath)) {
    throw new Error(`Binary not found after extraction in ${tmpDir}`);
  }

  // Copy native binary as dow-bin into BIN_DIR
  fs.mkdirSync(BIN_DIR, { recursive: true });
  const dstName = process.platform === "win32" ? "dow-bin.exe" : "dow-bin";
  const dstPath = path.join(BIN_DIR, dstName);
  fs.copyFileSync(srcPath, dstPath);
  if (process.platform !== "win32") {
    fs.chmodSync(dstPath, 0o755);
  }

  // Cleanup temp
  fs.rmSync(tmpDir, { recursive: true, force: true });

  console.log(`[dev-flow] Installed dow ${VERSION} for ${getPlatformKey()}`);

  // Register with coding agents
  try {
    execSync(`"${dstPath}" setup`, { stdio: "inherit" });
  } catch {
    console.log("[dev-flow] dow setup skipped (run manually: dow setup)");
  }
}

install().catch((err) => {
  console.error(`[dev-flow] Installation failed: ${err.message}`);
  console.error("[dev-flow] You can install manually: cargo install dev-flow");
  process.exit(1);
});

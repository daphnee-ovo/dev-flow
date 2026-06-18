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

  const tmpFile = path.join(BIN_DIR, "dow.tar.gz");
  fs.mkdirSync(BIN_DIR, { recursive: true });
  fs.writeFileSync(tmpFile, tarball);

  try {
    execSync(`tar -xzf "${tmpFile}" -C "${BIN_DIR}" --strip-components=1 bin/`, {
      stdio: "pipe",
    });
  } catch {
    // Windows or tar without --strip-components support
    execSync(`tar -xzf "${tmpFile}" -C "${BIN_DIR}"`, { stdio: "pipe" });
    // Move nested bin/dow to bin/
    const nested = path.join(BIN_DIR, "bin", "dow");
    if (fs.existsSync(nested)) {
      fs.renameSync(nested, path.join(BIN_DIR, "dow"));
      fs.rmSync(path.join(BIN_DIR, "bin"), { recursive: true, force: true });
    }
  }

  fs.unlinkSync(tmpFile);

  // Rename to dow-bin so it doesn't conflict with the Node.js wrapper (bin/dow)
  const srcName = process.platform === "win32" ? "dow.exe" : "dow";
  const dstName = process.platform === "win32" ? "dow-bin.exe" : "dow-bin";
  const srcPath = path.join(BIN_DIR, srcName);
  const dstPath = path.join(BIN_DIR, dstName);
  if (fs.existsSync(srcPath)) {
    fs.renameSync(srcPath, dstPath);
  }
  if (fs.existsSync(dstPath) && process.platform !== "win32") {
    fs.chmodSync(dstPath, 0o755);
  }

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

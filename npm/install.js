#!/usr/bin/env node
// Postinstall: download the prebuilt ccwhy binary for this platform
// from GitHub releases and unpack it next to the JS shim.

"use strict";

const { createWriteStream, chmodSync, mkdirSync, existsSync, unlinkSync } = require("node:fs");
const { join } = require("node:path");
const { execFileSync } = require("node:child_process");
const { get } = require("node:https");

const pkg = require("./package.json");
const VERSION = `v${pkg.version}`;
const REPO = "SingggggYee/ccwhy";

const PLATFORMS = {
  "darwin-arm64": "ccwhy-macos-aarch64.tar.gz",
  "darwin-x64": "ccwhy-macos-x86_64.tar.gz",
  "linux-arm64": "ccwhy-linux-aarch64.tar.gz",
  "linux-x64": "ccwhy-linux-x86_64.tar.gz",
};

const key = `${process.platform}-${process.arch}`;
const asset = PLATFORMS[key];
if (!asset) {
  console.error(`ccwhy: unsupported platform ${key}.`);
  console.error("Supported: macOS (arm64/x64) and Linux (arm64/x64).");
  console.error("Alternatives: cargo install ccwhy, or download a binary from");
  console.error(`https://github.com/${REPO}/releases`);
  process.exit(1);
}

const url = `https://github.com/${REPO}/releases/download/${VERSION}/${asset}`;
const vendorDir = join(__dirname, "vendor");
const tarPath = join(vendorDir, asset);
const binPath = join(vendorDir, "ccwhy");

function download(href, dest, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > 5) return reject(new Error("too many redirects"));
    get(href, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        res.resume();
        return resolve(download(res.headers.location, dest, redirects + 1));
      }
      if (res.statusCode !== 200) {
        res.resume();
        return reject(new Error(`HTTP ${res.statusCode} for ${href}`));
      }
      const out = createWriteStream(dest);
      res.pipe(out);
      out.on("finish", () => out.close(resolve));
      out.on("error", reject);
    }).on("error", reject);
  });
}

(async () => {
  try {
    mkdirSync(vendorDir, { recursive: true });
    await download(url, tarPath);
    execFileSync("tar", ["-xzf", tarPath, "-C", vendorDir]);
    unlinkSync(tarPath);
    if (!existsSync(binPath)) {
      throw new Error(`binary not found in archive (expected ${binPath})`);
    }
    chmodSync(binPath, 0o755);
    console.log(`ccwhy ${VERSION} installed for ${key}`);
  } catch (err) {
    console.error(`ccwhy: failed to download prebuilt binary: ${err.message}`);
    console.error("Alternatives: cargo install ccwhy, or download a binary from");
    console.error(`https://github.com/${REPO}/releases`);
    process.exit(1);
  }
})();

// Allowlist static server for the browser contract tests.
//
// It answers only what a consumer could legitimately load: the declared CSS
// exports from exports.tsv, the stylesheets those exports import, the public
// fixtures, and harness-only probes. Every
// other path is 404, so a fixture that reaches for a private or undeclared path
// fails the run instead of loading silently.

import { createServer } from "node:http";
import { readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..", "..");
const port = Number(process.env.DS_BROWSER_PORT ?? 4173);

const servedDirectories = [
  "fixtures/brand-theme/",
  "fixtures/plain-html/",
  "fixtures/layer-ownership/",
  "tests/browser/probes/",
];

const contentTypes = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
};

export function declaredExports() {
  return readFileSync(path.join(root, "exports.tsv"), "utf8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"))
    .map((line) => {
      const [compatibility, name, file] = line.split("\t");
      return { compatibility, name, path: file };
    });
}

// An export is published with the stylesheets it imports. ds-check confines
// every Design System import to a quoted "./" path below the importing file, so
// follow exactly that form and nothing else.
const IMPORT = /@import\s+"\.\/([^"]+)"/g;

export function publishedStylesheets() {
  const published = new Set();
  const pending = declaredExports().map((entry) => entry.path);
  while (pending.length > 0) {
    const relative = pending.pop();
    if (published.has(relative)) {
      continue;
    }
    published.add(relative);
    const source = readFileSync(path.join(root, relative), "utf8");
    for (const [, target] of source.matchAll(IMPORT)) {
      const imported = path.posix.join(path.posix.dirname(relative), target);
      if (!imported.startsWith(path.posix.dirname(relative) + "/")) {
        throw new Error(`import ${target} in ${relative} leaves its directory`);
      }
      pending.push(imported);
    }
  }
  return published;
}

const exportPaths = publishedStylesheets();

function allowed(relative) {
  if (exportPaths.has(relative)) {
    return true;
  }
  return servedDirectories.some((directory) => relative.startsWith(directory));
}

function resolve(urlPath) {
  let decoded;
  try {
    decoded = decodeURIComponent(urlPath);
  } catch {
    return null;
  }
  const relative = path.posix.normalize(decoded).replace(/^\/+/, "");
  if (relative.split("/").includes("..") || !allowed(relative)) {
    return null;
  }
  const type = contentTypes[path.extname(relative)];
  const file = path.join(root, relative);
  try {
    return type && statSync(file).isFile() ? { file, type } : null;
  } catch {
    return null;
  }
}

createServer((request, response) => {
  const { pathname } = new URL(request.url, "http://localhost");
  const target = request.method === "GET" ? resolve(pathname) : null;
  if (!target) {
    response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    response.end("not a declared export or fixture path\n");
    return;
  }
  response.writeHead(200, {
    "content-type": target.type,
    "cache-control": "no-store",
  });
  response.end(readFileSync(target.file));
}).listen(port, "127.0.0.1");

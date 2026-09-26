// Allowlist static server for the browser contract tests.
//
// It answers only what a consumer could legitimately load: the declared CSS
// exports from exports.tsv, the stylesheets those exports import, the public
// fixtures, and harness-only probes. Every
// other path is 404, so a fixture that reaches for a private or undeclared path
// fails the run instead of loading silently.

import { createServer } from "node:http";
import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..", "..");
const port = Number(process.env.DS_BROWSER_PORT ?? 4173);

// Packaged mode (release verification). DS_PACKAGED_ROOT names an UNPACKED release
// archive outside this repository. The Design System stylesheets are then served only
// from that archive's css/ directory, never from packages/styles: both the source-path
// alias the fixtures use (/packages/styles/...) and the real consumer path
// (/design-system/...) resolve into the archive. DS_PACKAGED_TAILWIND_OUTPUT names the
// Tailwind output built from the archive's tailwind.css. Every response served from
// packaged files carries `x-ds-source: packaged-archive` so a test can prove it.
const packagedRoot = process.env.DS_PACKAGED_ROOT ? path.resolve(process.env.DS_PACKAGED_ROOT) : null;
const packagedTailwind = process.env.DS_PACKAGED_TAILWIND_OUTPUT
  ? path.resolve(process.env.DS_PACKAGED_TAILWIND_OUTPUT)
  : null;
if (packagedRoot) {
  const inside = path.relative(root, packagedRoot);
  if (!inside.startsWith("..") && !path.isAbsolute(inside)) {
    throw new Error(`DS_PACKAGED_ROOT ${packagedRoot} is inside the repository; unpack the archive elsewhere`);
  }
  if (!existsSync(path.join(packagedRoot, "MANIFEST.tsv")) || !existsSync(path.join(packagedRoot, "css", "core.css"))) {
    throw new Error(`DS_PACKAGED_ROOT ${packagedRoot} is not an unpacked release archive`);
  }
}

const servedDirectories = [
  "fixtures/brand-theme/",
  "fixtures/layouts/",
  "fixtures/plain-html/",
  "fixtures/primitives/",
  "fixtures/scoping/",
  "fixtures/static-renderer/public/",
  "fixtures/layer-ownership/",
  "adapters/tailwind/fixture/",
  "tests/browser/probes/",
];

const contentTypes = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".svg": "image/svg+xml",
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

// The stylesheets an unpacked archive publishes: css/core.css and what it imports.
function packagedStylesheets() {
  const published = new Set();
  const pending = ["core.css"];
  while (pending.length > 0) {
    const relative = pending.pop();
    if (published.has(relative)) {
      continue;
    }
    published.add(relative);
    const source = readFileSync(path.join(packagedRoot, "css", relative), "utf8");
    for (const [, target] of source.matchAll(IMPORT)) {
      pending.push(path.posix.join(path.posix.dirname(relative), target));
    }
  }
  return published;
}

const exportPaths = packagedRoot ? new Set() : publishedStylesheets();
const packagedPaths = packagedRoot ? packagedStylesheets() : new Set();

/** The archive file behind a request path in packaged mode, or null. */
function packagedFile(relative) {
  let name = null;
  if (relative === "packages/styles/index.css") {
    name = "core.css";
  } else if (relative.startsWith("packages/styles/")) {
    name = relative.slice("packages/styles/".length);
  } else if (relative.startsWith("design-system/")) {
    name = relative.slice("design-system/".length);
  } else if (relative === "adapters/tailwind/fixture/output.css" && packagedTailwind) {
    return packagedTailwind;
  }
  return name !== null && packagedPaths.has(name) ? path.join(packagedRoot, "css", name) : null;
}

function allowed(relative) {
  if (packagedRoot) {
    if (relative.startsWith("packages/") || relative.startsWith("adapters/tailwind/index.css")) {
      return packagedFile(relative) !== null;
    }
    if (packagedFile(relative) !== null) {
      return true;
    }
  } else if (exportPaths.has(relative)) {
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
  const indexed = decoded.endsWith("/") ? `${decoded}index.html` : decoded;
  const relative = path.posix.normalize(indexed).replace(/^\/+/, "");
  if (relative.split("/").includes("..") || !allowed(relative)) {
    return null;
  }
  const type = contentTypes[path.extname(relative)];
  const packaged = packagedRoot ? packagedFile(relative) : null;
  const file = packaged ?? path.join(root, relative);
  try {
    return type && statSync(file).isFile() ? { file, type, packaged: packaged !== null } : null;
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
    ...(target.packaged ? { "x-ds-source": "packaged-archive" } : {}),
  });
  response.end(readFileSync(target.file));
}).listen(port, "127.0.0.1");

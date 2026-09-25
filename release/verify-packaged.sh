#!/bin/sh
# Prove the packaged form, not repository source, satisfies every consumer.
#
#   release/verify-packaged.sh <design-system-<version>.tar.gz>
#
# Unpacks the archive into a temporary directory OUTSIDE this repository, then:
#   1. `ds-check release --source`: manifest, imports, surface, docs, privacy, and byte
#      equality with the inventory sources;
#   2. plain / Tailwind-free fixtures, the primitives, layouts, scoping, and brand-theme
#      fixtures, and the whole browser suite in Chromium, Firefox, and WebKit, with every
#      Design System stylesheet served from the unpacked archive (tests/browser/server.mjs
#      packaged mode), never from packages/styles;
#   3. the Tailwind fixture, built by a consumer-owned locked Tailwind install from the
#      archive's declared tailwind.css export only;
#   4. the S5.T1 static-renderer fixture, staged from the archive's css/ and rendered by
#      the pinned Hugo under a PATH with no Rust, Node, or Tailwind, then checked by
#      `ds-check static-renderer`;
#   5. the final supported load path (/design-system/core.css) and the historical Chromium
#      @import layer-order race, in tests/browser/packaged.
#
# Run from the repository root. This is contributor verification: it uses Node, the
# Playwright browsers, and the pinned Hugo. It needs no credentials and publishes nothing.
# Environment: DS_CHECK (prebuilt ds-check), HUGO_BIN (real hugo past a version-manager
# shim), DS_BROWSER_PORT, DS_RACE_ITERATIONS.
set -eu

die() { printf 'verify-packaged: %s\n' "$*" >&2; exit 1; }
[ $# -eq 1 ] && [ -f "$1" ] || die "usage: verify-packaged.sh <archive.tar.gz>"
[ -f exports.tsv ] && [ -d tests/browser ] || die "run from the repository root"
repo=$(pwd)
archive=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
base=$(basename "$archive")
name=${base%.tar.gz}

if [ -z "${DS_CHECK:-}" ]; then
  cargo build -q --locked -p design-system-check
  DS_CHECK=$repo/target/debug/design-system-check
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
case "$work" in "$repo"|"$repo"/*) die "the working directory is inside the repository" ;; esac
tar -xzf "$archive" -C "$work"
pkg=$work/$name
[ -d "$pkg/css" ] || die "the archive has no css/ directory"

printf '== 1. archive proof (%s)\n' "$name"
"$DS_CHECK" release "$pkg" "$repo"

printf '== 2. Tailwind consumer built from the declared export only\n'
consumer=$work/tailwind-consumer
mkdir -p "$consumer"
cp adapters/tailwind/package.json adapters/tailwind/package-lock.json "$consumer/"
cp adapters/tailwind/fixture/index.html "$consumer/index.html"
cp -R "$pkg/css" "$consumer/design-system"
# The consumer's own input: the same content as the fixture input, importing the archive's
# declared tailwind.css export instead of a repository path.
sed 's|@import "\.\./index\.css" source(none);|@import "./design-system/tailwind.css" source(none);|' adapters/tailwind/fixture/input.css > "$consumer/input.css"
grep -q '@import "./design-system/tailwind.css" source(none);' "$consumer/input.css" || die "the fixture input no longer has the expected core import"
( cd "$consumer" && npm ci --no-audit --no-fund >/dev/null && npx tailwindcss -i ./input.css -o ./output.css --minify )
[ -s "$consumer/output.css" ] || die "the Tailwind build produced no output"
if grep -q 'packages/styles\|adapters/tailwind' "$consumer/output.css"; then die "the Tailwind output names a repository path"; fi

printf '== 3. static-renderer fixture staged from the archive, toolchain-free\n'
DS_PACKAGED_CSS=$pkg/css sh fixtures/static-renderer/verify-toolchain-free.sh
"$DS_CHECK" static-renderer exports.tsv layouts.tsv primitives.tsv theme.tsv base.tsv fixtures/static-renderer

printf '== 4. browser suite against the unpacked archive (all three engines)\n'
export DS_PACKAGED_ROOT=$pkg DS_PACKAGED_TAILWIND_OUTPUT=$consumer/output.css
( cd tests/browser && npx playwright test )

printf '== 5. packaged load path and the @import layer-order race\n'
( cd tests/browser && npx playwright test --config playwright.packaged.config.mjs )

printf 'packaged form verified: %s\n' "$name"

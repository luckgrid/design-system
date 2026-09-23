#!/bin/sh
# Consumer step: render the fixture with the pinned Hugo. Run from the repository
# root after stage.sh. It reads only the fixture and its staged boundary, and
# needs only Hugo plus POSIX shell tools: no Node, Rust, Tailwind, or package
# manager.
set -eu

fixture=fixtures/static-renderer

die() { printf 'build: %s\n' "$*" >&2; exit 1; }

[ -f "$fixture/hugo.toml" ] || die "run from the repository root"
[ -d "$fixture/stage/design-system" ] || die "no staged boundary; run $fixture/stage.sh first"
command -v hugo >/dev/null 2>&1 || die "hugo is not on PATH"

want=$(cat "$fixture/HUGO_VERSION")
have=$(hugo version | sed -n 's/^hugo v\([0-9][0-9.]*\).*/\1/p')
[ "$have" = "$want" ] || die "hugo $have found; the fixture pins exactly $want"

rm -rf "$fixture/public"
cache=$(mktemp -d)
trap 'rm -rf "$cache"' EXIT
HUGO_CACHEDIR=$cache hugo \
  --source "$fixture" \
  --destination "$(pwd)/$fixture/public" \
  --environment production \
  --noBuildLock \
  --quiet
printf 'rendered %s with hugo %s\n' "$fixture/public" "$have"

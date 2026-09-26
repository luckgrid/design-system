#!/bin/sh
# Prove the stage and render steps run with no Rust, Node, or Tailwind toolchain.
# Run from the repository root.
#
# The steps run under an empty environment whose PATH is a directory of symlinks
# to only the POSIX tools they use plus the pinned Hugo. The script first proves
# that no forbidden toolchain command resolves on that PATH, then stages and
# renders. HUGO_BIN overrides how Hugo is found (for example past a version
# manager shim). EXTRA_TOOLS symlinks additional commands into the PATH; it
# exists so a negative probe can show that a forbidden tool fails the check.
set -eu

die() { printf 'toolchain-free: %s\n' "$*" >&2; exit 1; }

fixture=fixtures/static-renderer
[ -f "$fixture/hugo.toml" ] || die "run from the repository root"

bin=$(mktemp -d)
home=$(mktemp -d)
trap 'rm -rf "$bin" "$home"' EXIT

for tool in sh cat cp cut dirname grep head mkdir mktemp mv rm sed shasum sha256sum sort tail tr wc ${EXTRA_TOOLS:-}; do
  path=$(command -v "$tool" 2>/dev/null || true)
  case "$path" in /*) ln -s "$path" "$bin/$tool" ;; esac
done
hugo_path=${HUGO_BIN:-$(command -v hugo || true)}
[ -x "$hugo_path" ] || die "hugo not found"
ln -s "$hugo_path" "$bin/hugo"

for forbidden in cargo rustc rustup rustfmt node nodejs npm npx pnpm yarn bun deno tailwindcss; do
  if PATH=$bin command -v "$forbidden" >/dev/null 2>&1; then
    die "$forbidden resolves on the restricted PATH"
  fi
done

# DS_PACKAGED_CSS, when set, points the stage step at an unpacked release archive.
run() { env -i PATH="$bin" HOME="$home" TMPDIR="$home" ${DS_PACKAGED_CSS:+DS_PACKAGED_CSS="$DS_PACKAGED_CSS"} sh "$@"; }
run "$fixture/stage.sh"
run "$fixture/build.sh"
printf 'toolchain-free: staged and rendered with PATH limited to %s\n' "$(ls "$bin" | tr '\n' ' ')"

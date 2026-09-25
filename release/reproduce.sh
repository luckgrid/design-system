#!/bin/sh
# Assemble the same candidate twice from two independent clean clones and compare.
#
#   release/reproduce.sh --source <git-url-or-path> --commit <full-sha> \
#       (--rehearsal <n> | --release) [--out <dir>]
#
# Each assembly runs in a fresh clone in its own temporary directory, so neither sees
# the other's files or any local edit. The script then compares the tar checksum, the
# unpacked file trees, IDENTITY.tsv, and MANIFEST.tsv byte for byte, and reports every
# field that legitimately differs.
#
# Legitimately nondeterministic fields, and only these:
#   - the gzip wrapper bytes and therefore the .tar.gz checksum, when the two
#     assemblies use different gzip builds (compare the tar checksum instead);
#   - BUILD-ENV.txt tool_* lines, which record the tools that ran;
#   - the temporary directory paths, which appear in no archive file.
# The tar, MANIFEST.tsv, IDENTITY.tsv, and every archived file must be identical.
# When both clones run on one machine the gzip wrapper is expected to match too, and
# the script reports whether it did.
set -eu

die() { printf 'reproduce: %s\n' "$*" >&2; exit 1; }

source_repo=; commit=; mode_args=; out=
while [ $# -gt 0 ]; do
  case "$1" in
    --source) source_repo=${2:?}; shift 2 ;;
    --commit) commit=${2:?}; shift 2 ;;
    --rehearsal) mode_args="--rehearsal ${2:?}"; shift 2 ;;
    --release) mode_args="--release --commit $commit"; shift ;;
    --out) out=${2:?}; shift 2 ;;
    *) die "unknown argument: $1" ;;
  esac
done
[ -n "$source_repo" ] && [ -n "$commit" ] && [ -n "$mode_args" ] || die "need --source, --commit, and --rehearsal <n> or --release"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
for side in a b; do
  git clone -q --no-hardlinks "$source_repo" "$work/clone-$side"
  git -C "$work/clone-$side" checkout -q --detach "$commit"
  [ "$(git -C "$work/clone-$side" rev-parse HEAD)" = "$commit" ] || die "clone $side is not at $commit"
  ( cd "$work/clone-$side" && sh release/assemble.sh $mode_args --out "$work/out-$side" > "$work/assemble-$side.log" )
done

field() { sed -n "s/^$2=//p" "$work/out-$1/BUILD-ENV.txt"; }
tar_a=$(field a tar_sha256); tar_b=$(field b tar_sha256)
gz_a=$(field a archive_sha256); gz_b=$(field b archive_sha256)
[ "$tar_a" = "$tar_b" ] || die "the tar checksums differ: $tar_a vs $tar_b"
cmp -s "$work/out-a/SHA256SUMS" "$work/out-b/SHA256SUMS" && sums=identical || sums=DIFFERENT
name=$(ls "$work/out-a"/design-system-*.tar | head -n 1 | xargs basename | sed 's/\.tar$//')
diff -r "$work/out-a/stage/$name" "$work/out-b/stage/$name" >/dev/null || die "the unpacked trees differ"
cmp -s "$work/out-a/stage/$name/MANIFEST.tsv" "$work/out-b/stage/$name/MANIFEST.tsv" || die "MANIFEST.tsv differs"
cmp -s "$work/out-a/stage/$name/IDENTITY.tsv" "$work/out-b/stage/$name/IDENTITY.tsv" || die "IDENTITY.tsv differs"
if [ "$gz_a" = "$gz_b" ]; then gz=identical; else gz="DIFFERENT (allowed only across gzip builds)"; fi
printf 'reproducible: %s\n  tar sha256      %s (identical in both clean clones)\n  .tar.gz sha256  %s\n  gzip wrapper    %s\n  SHA256SUMS      %s\n' "$name" "$tar_a" "$gz_a" "$gz" "$sums"
printf '  differing BUILD-ENV lines:\n'
diff "$work/out-a/BUILD-ENV.txt" "$work/out-b/BUILD-ENV.txt" | sed 's/^/    /' || true
if [ -n "$out" ]; then mkdir -p "$out"; cp "$work/out-a/$name.tar.gz" "$work/out-a/$name.tar" "$work/out-a/SHA256SUMS" "$work/out-a/BUILD-ENV.txt" "$work/out-a/ds-consumer.sh" "$out/"; fi

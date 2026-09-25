#!/bin/sh
# Verify one release archive exactly as a consumer would receive it.
#
#   release/verify-archive.sh <design-system-<version>.tar.gz> [--source <repo-root>]
#
# Checks the SHA256SUMS beside the archive (when present), that every tar member stays
# inside design-system-<version>/ with no link or device, unpacks into a temporary
# directory OUTSIDE the source repository, and runs `ds-check release` on the unpacked
# tree, which is the manifest, class, import-graph, surface, documentation, and privacy
# proof. With --source it also proves every file equals its inventory source.
#
# Run from the repository root. DS_CHECK names a prebuilt ds-check binary; the default
# builds and uses target/debug/design-system-check. Prints the unpacked directory on
# the last line when DS_KEEP=1, so a caller can reuse it.
set -eu

die() { printf 'verify-archive: %s\n' "$*" >&2; exit 1; }
[ $# -ge 1 ] && [ -f "$1" ] || die "usage: verify-archive.sh <archive.tar.gz> [--source <repo-root>]"
archive=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
shift
source_root=
if [ $# -ge 2 ] && [ "$1" = --source ]; then source_root=$2; fi

if command -v sha256sum >/dev/null 2>&1; then
  digest() { sha256sum "$1" | cut -d ' ' -f 1; }
else
  digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
fi

base=$(basename "$archive")
case "$base" in design-system-*.tar.gz) ;; *) die "unexpected archive name $base" ;; esac
name=${base%.tar.gz}

sums=$(dirname "$archive")/SHA256SUMS
if [ -f "$sums" ]; then
  want=$(sed -n "s/^\([0-9a-f]\{64\}\)  $base\$/\1/p" "$sums")
  [ -n "$want" ] || die "SHA256SUMS has no entry for $base"
  [ "$(digest "$archive")" = "$want" ] || die "archive checksum does not match SHA256SUMS"
  printf 'checksum   %s matches SHA256SUMS\n' "$base"
fi

dots=..
tar -tzf "$archive" | while IFS= read -r entry; do
  case "$entry" in
    /*|"$dots"|"$dots"/*|*/"$dots"/*|*/"$dots") die "unsafe member path: $entry" ;;
    "$name"|"$name"/*) ;;
    *) die "member outside $name/: $entry" ;;
  esac
done
tar -tvzf "$archive" | while IFS= read -r line; do
  case "$line" in
    -*|d*) ;;
    *) die "non-regular member: $line" ;;
  esac
done
printf 'members    all inside %s/, regular files and directories only\n' "$name"

if [ -z "${DS_CHECK:-}" ]; then
  cargo build -q --locked -p design-system-check
  DS_CHECK=$(pwd)/target/debug/design-system-check
fi
work=$(mktemp -d)
if [ "${DS_KEEP:-}" != 1 ]; then trap 'rm -rf "$work"' EXIT; fi
tar -xzf "$archive" -C "$work"
[ -z "$(find "$work" -type l)" ] || die "the archive unpacks a symbolic link"
if [ -n "$source_root" ]; then
  "$DS_CHECK" release "$work/$name" "$source_root"
else
  "$DS_CHECK" release "$work/$name"
fi
if [ "${DS_KEEP:-}" = 1 ]; then printf '%s\n' "$work/$name"; fi

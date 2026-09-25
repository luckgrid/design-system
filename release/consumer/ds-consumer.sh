#!/bin/sh
# Pin, obtain, verify, install, remove, and restore a Design System release archive.
#
# Needs only POSIX shell tools plus tar and gzip; curl only for an https source.
# No Rust, Node, Tailwind, package manager, or Design System source checkout.
#
#   ds-consumer.sh pin <version> <archive-sha256> <archive-source>
#   ds-consumer.sh install <dest-dir>
#   ds-consumer.sh verify <dest-dir>
#   ds-consumer.sh remove <dest-dir>
#   ds-consumer.sh purge <dest-dir>
#   ds-consumer.sh restore <dest-dir>
#
# Run from the consumer project root. State lives beside your files:
#   design-system.pin           the active pin (version, archive name, sha256, source)
#   design-system.pin.previous  the pin that was active before the last `pin`
#   .design-system-cache/       the downloaded archive and its unpacked copy
# `install` puts the browser-ready CSS in <dest-dir>/design-system/ and prints the
# exact stylesheet path to link: <dest-dir>/design-system/core.css.
set -eu

pin_file=design-system.pin
previous_file=design-system.pin.previous
cache=${DS_CACHE:-.design-system-cache}
tab=$(printf '\t')

die() { printf 'ds-consumer: %s\n' "$*" >&2; exit 1; }

if command -v sha256sum >/dev/null 2>&1; then
  digest() { sha256sum "$1" | cut -d ' ' -f 1; }
else
  digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
fi

pin_value() { sed -n "s/^$1=//p" "$2" | head -n 1; }

read_pin() {
  [ -f "$pin_file" ] || die "no $pin_file; run: ds-consumer.sh pin <version> <sha256> <source>"
  version=$(pin_value version "$pin_file")
  archive=$(pin_value archive "$pin_file")
  sha256=$(pin_value sha256 "$pin_file")
  source=$(pin_value source "$pin_file")
  [ -n "$version" ] && [ -n "$archive" ] && [ -n "$sha256" ] && [ -n "$source" ] || die "$pin_file is incomplete"
  [ "$archive" = "design-system-$version.tar.gz" ] || die "$pin_file archive does not match its version"
}

cmd_pin() {
  [ $# -eq 3 ] || die "usage: pin <version> <archive-sha256> <archive-source>"
  case "$1" in
    ""|*[!0-9A-Za-z.+-]*) die "version has characters outside 0-9 A-Z a-z . + -" ;;
  esac
  case "$2" in
    *[!0-9a-f]*|"") die "sha256 must be 64 lowercase hex digits" ;;
  esac
  [ "${#2}" -eq 64 ] || die "sha256 must be 64 lowercase hex digits"
  [ -n "$3" ] || die "archive source is empty"
  if [ -f "$pin_file" ]; then cp "$pin_file" "$previous_file"; fi
  printf 'version=%s\narchive=design-system-%s.tar.gz\nsha256=%s\nsource=%s\n' "$1" "$1" "$2" "$3" > "$pin_file"
  printf 'pinned design-system %s (sha256 %s)\n' "$1" "$2"
}

fetch_archive() {
  read_pin
  mkdir -p "$cache"
  file=$cache/$archive
  if [ ! -f "$file" ] || [ "$(digest "$file")" != "$sha256" ]; then
    rm -f "$file"
    case "$source" in
      https://*) command -v curl >/dev/null 2>&1 || die "curl is needed for an https source"
                 curl -fsSL -o "$file" "$source" || die "download failed: $source" ;;
      file://*)  cp "${source#file://}" "$file" || die "cannot read $source" ;;
      /*|./*)    cp "$source" "$file" || die "cannot read $source" ;;
      *) die "archive source must be https://, file://, or a path: $source" ;;
    esac
  fi
  got=$(digest "$file")
  if [ "$got" != "$sha256" ]; then
    rm -f "$file"
    die "checksum mismatch for $archive: expected $sha256, got $got"
  fi
}

check_listing() {
  dots=..
  tar -tzf "$1" > "$cache/listing.$$" || die "cannot list $1"
  while IFS= read -r entry; do
    case "$entry" in
      /*|"$dots"|"$dots"/*|*/"$dots"/*|*/"$dots") rm -f "$cache/listing.$$"; die "unsafe archive path: $entry" ;;
    esac
    case "$entry" in
      design-system-"$version"|design-system-"$version"/*) ;;
      *) rm -f "$cache/listing.$$"; die "archive path outside design-system-$version/: $entry" ;;
    esac
  done < "$cache/listing.$$"
  rm -f "$cache/listing.$$"
}

cmd_install() {
  [ $# -eq 1 ] || die "usage: install <dest-dir>"
  dest=$1
  fetch_archive
  check_listing "$file"
  work=$cache/unpacked
  rm -rf "$work"
  mkdir -p "$work"
  tar -xzf "$file" -C "$work"
  root=$work/design-system-$version
  [ -f "$root/MANIFEST.tsv" ] || die "archive has no MANIFEST.tsv"
  if [ -n "$(find "$work" -type l 2>/dev/null)" ]; then die "archive contains a symbolic link"; fi
  [ "$(sed -n "s/^version$tab//p" "$root/IDENTITY.tsv")" = "$version" ] || die "archive identity does not match the pin"
  # Every archived file must match its manifest checksum before anything is installed.
  while IFS="$tab" read -r want _size _class _kind path; do
    case "$want" in \#*|"") continue ;; esac
    [ -f "$root/$path" ] || die "archive is missing $path"
    [ "$(digest "$root/$path")" = "$want" ] || die "archive file $path does not match MANIFEST.tsv"
  done < "$root/MANIFEST.tsv"
  target=$dest/design-system
  if [ -e "$target" ] && [ ! -f "$target/.ds-pin" ]; then
    die "$target exists and is not a Design System install; refusing to replace it"
  fi
  rm -rf "$target"
  mkdir -p "$target"
  ( cd "$root/css" && find . -type f | sed 's|^\./||' | sort ) | while IFS= read -r rel; do
    mkdir -p "$target/$(dirname "$rel")"
    cp "$root/css/$rel" "$target/$rel"
  done
  cp "$pin_file" "$target/.ds-pin"
  css_manifest "$root/MANIFEST.tsv" > "$target/.ds-manifest"
  rm -rf "$work"
  printf 'installed design-system %s\nlink this stylesheet first: %s/core.css\n' "$version" "$target"
}

# The css/ rows of MANIFEST.tsv as <sha256><TAB><path relative to css/>.
css_manifest() {
  while IFS="$tab" read -r want _size _class _kind path; do
    case "$path" in css/*) printf '%s\t%s\n' "$want" "${path#css/}" ;; esac
  done < "$1" | sort -k 2
}

cmd_verify() {
  [ $# -eq 1 ] || die "usage: verify <dest-dir>"
  target=$1/design-system
  read_pin
  [ -f "$target/.ds-pin" ] || die "no install at $target"
  cmp -s "$pin_file" "$target/.ds-pin" || die "installed pin differs from $pin_file"
  listed=$cache.listed.$$
  ( cd "$target" && find . -type f ! -name '.ds-*' | sed 's|^\./||' | sort ) > "$listed"
  count=0
  while IFS="$tab" read -r want path; do
    [ -f "$target/$path" ] || { rm -f "$listed"; die "installed file $path is missing"; }
    [ "$(digest "$target/$path")" = "$want" ] || { rm -f "$listed"; die "installed file $path was modified"; }
    count=$((count + 1))
  done < "$target/.ds-manifest"
  [ "$(wc -l < "$listed" | tr -d ' ')" = "$count" ] || { rm -f "$listed"; die "installed tree holds files outside the manifest"; }
  rm -f "$listed"
  printf 'verified design-system %s: %s file(s) match sha256 %s\n' "$version" "$count" "$sha256"
}

cmd_remove() {
  [ $# -eq 1 ] || die "usage: remove <dest-dir>"
  target=$1/design-system
  if [ -e "$target" ]; then
    [ -f "$target/.ds-pin" ] || die "$target is not a Design System install; refusing to remove it"
    rm -rf "$target"
  fi
  rm -rf "$cache"
  printf 'removed design-system install and cache\n'
}

cmd_purge() {
  cmd_remove "$@"
  rm -f "$pin_file" "$previous_file"
  printf 'purged pins: the project is back to its pre-design-system state\n'
}

cmd_restore() {
  [ $# -eq 1 ] || die "usage: restore <dest-dir>"
  if [ -f "$previous_file" ]; then
    cmd_remove "$1" >/dev/null
    mv "$previous_file" "$pin_file"
    cmd_install "$1"
    printf 'restored the previous pin\n'
  else
    cmd_purge "$1" >/dev/null
    printf 'no previous pin: restored the pre-design-system state\n'
  fi
}

[ $# -ge 1 ] || die "usage: ds-consumer.sh pin|install|verify|remove|purge|restore ..."
command=$1
shift
case "$command" in
  pin) cmd_pin "$@" ;;
  install) cmd_install "$@" ;;
  verify) cmd_verify "$@" ;;
  remove) cmd_remove "$@" ;;
  purge) cmd_purge "$@" ;;
  restore) cmd_restore "$@" ;;
  *) die "unknown command: $command" ;;
esac

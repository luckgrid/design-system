#!/bin/sh
# Producer step: stage the declared public CSS exports into a release-shaped
# boundary. Run from the repository root; needs only POSIX shell tools.
#
# The staged tree holds each export under its declared export name plus exactly
# the stylesheets that export imports, laid out relative to the export file. No
# repository path survives into it, so a consumer sees the same shape a release
# archive will have. The consumer build reads only this tree.
set -eu

fixture=fixtures/static-renderer
stage=$fixture/stage
out=$stage/design-system
tab=$(printf '\t')

die() { printf 'stage: %s\n' "$*" >&2; exit 1; }

[ -f exports.tsv ] || die "run from the repository root (no exports.tsv)"

if command -v sha256sum >/dev/null 2>&1; then
  digest() { sha256sum "$1" | cut -d ' ' -f 1; }
else
  digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
fi

rm -rf "$stage"
mkdir -p "$out"
: > "$stage/MANIFEST.tsv"

exports=$(grep -v '^[[:space:]]*#' exports.tsv | grep -v '^[[:space:]]*$' || true)
[ -n "$exports" ] || die "exports.tsv declares no export"

printf '%s\n' "$exports" | while IFS="$tab" read -r class name source; do
  [ "$class" = public-preview ] || die "export $name has unexpected class $class"
  case "$name" in
    */*|..*|"") die "export name $name is not a plain file name" ;;
  esac
  root_dir=$(dirname "$source")
  todo=$stage/todo.$$
  seen=$stage/seen.$$
  printf '%s\n' "$name" > "$todo"
  : > "$seen"
  while [ -s "$todo" ]; do
    rel=$(head -n 1 "$todo")
    tail -n +2 "$todo" > "$todo.next" && mv "$todo.next" "$todo"
    grep -qxF "$rel" "$seen" && continue
    printf '%s\n' "$rel" >> "$seen"
    if [ "$rel" = "$name" ]; then src=$source; else src=$root_dir/$rel; fi
    [ -f "$src" ] || die "missing stylesheet for $rel"
    mkdir -p "$out/$(dirname "$rel")"
    cp "$src" "$out/$rel"
    printf '%s\t%s\n' "$(digest "$out/$rel")" "$rel" >> "$stage/MANIFEST.tsv"
    # Imports are confined to the quoted ./ form below the importing file.
    if grep '^@import' "$src" | grep -qv '^@import[[:space:]]*"\./[^"]*"'; then
      die "$rel has an import outside the quoted ./ form"
    fi
    here=$(dirname "$rel")
    grep '^@import' "$src" | sed 's/^@import[[:space:]]*"\.\/\([^"]*\)".*/\1/' | while read -r target; do
      if [ "$here" = . ]; then next=$target; else next=$here/$target; fi
      printf '%s\n' "$next"
    done >> "$todo"
  done
  rm -f "$todo" "$seen"
done

sort -k 2 "$stage/MANIFEST.tsv" -o "$stage/MANIFEST.tsv"
printf 'staged %s file(s) under %s\n' "$(wc -l < "$stage/MANIFEST.tsv" | tr -d ' ')" "$out"

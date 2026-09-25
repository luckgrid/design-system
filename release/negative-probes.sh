#!/bin/sh
# Prove that `ds-check release` rejects what it must. Each probe mutates a fresh copy
# of an unpacked archive, re-writes MANIFEST.tsv where the mutation should not be caught
# by the checksum alone, and requires a specific failure.
#
#   release/negative-probes.sh <unpacked-archive-dir>
#
# Run from the repository root. DS_CHECK names a prebuilt ds-check binary; the default
# builds and uses target/debug/design-system-check.
set -eu

die() { printf 'probes: %s\n' "$*" >&2; exit 1; }
[ $# -eq 1 ] && [ -d "$1" ] || die "usage: negative-probes.sh <unpacked-archive-dir>"
src=$(cd "$1" && pwd)
name=$(basename "$src")
tab=$(printf '\t')
if [ -z "${DS_CHECK:-}" ]; then
  cargo build -q --locked -p design-system-check
  DS_CHECK=$(pwd)/target/debug/design-system-check
fi
if command -v sha256sum >/dev/null 2>&1; then
  digest() { sha256sum "$1" | cut -d ' ' -f 1; }
else
  digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
work=$tmp/$name
pass=0

fresh() { rm -rf "$tmp/$name"; cp -R "$src" "$tmp/$name"; }
# Re-record one file's checksum and size so only the content rule can reject it.
remanifest() {
  sum=$(digest "$work/$1")
  size=$(wc -c < "$work/$1" | tr -d ' ')
  sed "s|^[0-9a-f]\{64\}${tab}[0-9]*${tab}\(.*\)${tab}$1\$|$sum${tab}$size${tab}\1${tab}$1|" "$work/MANIFEST.tsv" > "$work/MANIFEST.new"
  mv "$work/MANIFEST.new" "$work/MANIFEST.tsv"
}
expect() { # <label> <substring> [extra ds-check args]
  label=$1; want=$2; shift 2
  if out=$("$DS_CHECK" release "$work" "$@" 2>&1); then
    die "probe '$label' was ACCEPTED: $out"
  fi
  case "$out" in
    *"$want"*) pass=$((pass + 1)); printf 'rejected  %-44s %s\n' "$label" "$want" ;;
    *) die "probe '$label' failed for the wrong reason (wanted '$want'): $out" ;;
  esac
}
append() { printf '%s\n' "$2" >> "$work/$1"; remanifest "$1"; }

fresh; "$DS_CHECK" release "$work" >/dev/null || die "the unmutated archive must pass"

fresh; printf ' ' >> "$work/css/base.css";                            expect "edited css without re-manifest" "does not match its MANIFEST.tsv"
fresh; printf 'x\n' > "$work/css/extra.css";                            expect "extra file" "does not list"
fresh; rm "$work/css/layouts/grid.css";                                  expect "missing file" "is missing"
fresh; ln -s core.css "$work/css/link.css";                              expect "symbolic link" "symbolic link"
fresh; chmod +x "$work/css/core.css";                                    expect "executable stylesheet" "executable bit"
fresh; chmod -x "$work/consumer/ds-consumer.sh";                         expect "non-executable tool" "executable bit"

fresh; append css/base.css '@import "../escape.css";';                  expect "import leaves the archive" "must be quoted ./ paths"
fresh; append css/base.css '@import "./missing.css";';                  expect "import of a missing file" "not in the archive"
fresh; append css/base.css '@\69mport "https://example.test/x.css";'; expect "escaped remote import" "must be quoted ./ paths"
fresh; append css/base.css 'a { color: red ! important; }';             expect "spaced important" "contains !important"
fresh; append css/base.css 'a { color: red !\69mportant; }';            expect "escaped important" "contains !important"
fresh; append css/base.css 'a { background: url(x.png); }';             expect "url() in the packaged CSS" "url("
fresh; append css/base.css '@theme { --x: 1; }';                        expect "Tailwind directive in the core" "Tailwind-free"
fresh; append css/tailwind.css ':root { --x: var(--ds-\72 ef-gray-100); }'; expect "escaped reference token in the adapter" "--ds-ref-"
fresh; append css/tokens/semantic.css ':root { --ds-color-extra: red; }'; expect "undeclared custom property" "does not classify"
fresh; append css/layouts/stack.css '.ds-extra { color: red; }';         expect "unpromoted hook" "do not promote"
fresh; append css/core.css '@layer other;'; sed 's/^@layer ds\.tokens.*$/@layer ds.base, ds.tokens;/' "$work/css/core.css" > "$work/core.new"; mv "$work/core.new" "$work/css/core.css"; remanifest css/core.css; expect "changed layer order" "must publish"

fresh; append RELEASE.md 'see /Users/someone/private/notes';           expect "local user path" "local user path"
fresh; append RELEASE.md 'token ghp_abcdefghijklmnopqrstuvwxyz0123456789'; expect "credential" "GitHub credential"
fresh; append docs/architecture/hooks.md 'moved to lg-workstreams';     expect "private repository name" "private repository name"
fresh; append docs/architecture/hooks.md 'see [gone](missing.md)';      expect "dangling documentation link" "not in the archive"
fresh; append RELEASE.md '@VERSION@';                                   expect "unreplaced placeholder" "unreplaced placeholder"
fresh; append docs/browser-support.md 'Firefox follows the system keyboard-navigation setting'; expect "R1 unobserved generalization" "R1"
fresh; sed 's/Chrome for Testing 123.0.6312.122/Chrome 123/g' "$work/docs/browser-support.md" > "$work/b.new"; mv "$work/b.new" "$work/docs/browser-support.md"; remanifest docs/browser-support.md; expect "R2 Chrome for Testing unnamed" "R2"
fresh; sed 's/Windows Edge/Edge/g; s/Linux desktop/Linux/g' "$work/docs/browser-support.md" > "$work/b.new"; mv "$work/b.new" "$work/docs/browser-support.md"; remanifest docs/browser-support.md; expect "R3 unverified platforms unnamed" "R3"
fresh; append consumer/ds-consumer.sh 'cargo build';                    expect "consumer script runs cargo" "toolchain-free"

fresh; sed 's/^MIT License/Apache License/' "$work/LICENSE" > "$work/l.new"; mv "$work/l.new" "$work/LICENSE"; remanifest LICENSE; expect "wrong license text" "MIT License"
fresh; sed "s/^license${tab}MIT/license${tab}Apache-2.0/" "$work/IDENTITY.tsv" > "$work/i.new"; mv "$work/i.new" "$work/IDENTITY.tsv"; remanifest IDENTITY.tsv; expect "identity license" "license"
fresh; sed "s/^maturity${tab}preview/maturity${tab}stable/" "$work/IDENTITY.tsv" > "$work/i.new"; mv "$work/i.new" "$work/IDENTITY.tsv"; remanifest IDENTITY.tsv; expect "stable maturity" "maturity"
fresh; sed "s/^identity${tab}rehearsal/identity${tab}release/" "$work/IDENTITY.tsv" > "$work/i.new"; mv "$work/i.new" "$work/IDENTITY.tsv"; remanifest IDENTITY.tsv; expect "rehearsal claiming to be a release" "release identity"

fresh; append css/base.css '/* comment only */'; expect "edited stylesheet against source" "differs from" .

printf 'all %s negative probes rejected as required\n' "$pass"

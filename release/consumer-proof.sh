#!/bin/sh
# Independent static-consumer proof: pin -> obtain -> checksum-verify -> install ->
# verify the active pin -> upgrade -> restore the previous pin -> remove -> restore the
# pre-Design-System state, with a PATH that holds only POSIX tools, tar, and gzip.
#
#   release/consumer-proof.sh <archive-A.tar.gz> <archive-B.tar.gz>
#
# A is the previous known-good pin; B is the candidate. The consumer is a plain static
# site directory outside this repository. It needs no Rust, Node, Tailwind, private
# path, or repository-relative import: the script under test is the one the archive ships
# (consumer/ds-consumer.sh), extracted from the archive, not read from this repository.
#
# Run from anywhere; it changes only temporary directories. By default the archives are
# obtained through a file:// source, the same code path as a local mirror. To prove the
# published assets, set DS_PROOF_SOURCE_A and DS_PROOF_SOURCE_B to their https:// URLs
# (the archives named on the command line must be the same files, so their checksums
# match); curl is then added to the PATH and nothing else changes.
set -eu

die() { printf 'consumer-proof: %s\n' "$*" >&2; exit 1; }
[ $# -eq 2 ] && [ -f "$1" ] && [ -f "$2" ] || die "usage: consumer-proof.sh <archive-A.tar.gz> <archive-B.tar.gz>"
a=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
b=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")

if command -v sha256sum >/dev/null 2>&1; then
  digest() { sha256sum "$1" | cut -d ' ' -f 1; }
else
  digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
fi
version_of() { basename "$1" .tar.gz | sed 's/^design-system-//'; }
va=$(version_of "$a"); vb=$(version_of "$b")
[ "$va" != "$vb" ] || die "the two archives must be different versions"

work=$(mktemp -d)
bin=$work/bin
site=$work/site
mkdir -p "$bin" "$site/public"
trap 'rm -rf "$work"' EXIT

# A PATH of only the tools the consumer flow needs.
for tool in sh cat cp cmp cut dirname basename find grep gzip head ls mkdir mktemp mv rm sed sha256sum shasum sort tail tar tr uname wc diff; do
  path=$(command -v "$tool" 2>/dev/null || true)
  case "$path" in /*) ln -s "$path" "$bin/$tool" ;; esac
done
source_a=${DS_PROOF_SOURCE_A:-file://$a}
source_b=${DS_PROOF_SOURCE_B:-file://$b}
case "$source_a$source_b" in
  *https://*)
    path=$(command -v curl 2>/dev/null || true)
    [ -n "$path" ] || die "curl is needed for an https:// source"
    ln -s "$path" "$bin/curl" ;;
esac
for forbidden in cargo rustc rustup node nodejs npm npx pnpm yarn bun deno tailwindcss hugo; do
  if PATH=$bin command -v "$forbidden" >/dev/null 2>&1; then die "$forbidden resolves on the restricted PATH"; fi
done

# The script is taken from the archive, exactly as a consumer would obtain it.
tar -xzf "$a" -C "$work" "design-system-$va/consumer/ds-consumer.sh"
script=$work/design-system-$va/consumer/ds-consumer.sh
ds() { ( cd "$site" && env -i PATH="$bin" HOME="$work" TMPDIR="$work" sh "$script" "$@" ); }

# A consumer that does not use the Design System yet.
printf '<!doctype html><title>site</title><p>hello</p>\n' > "$site/public/index.html"
printf 'p { margin: 0; }\n' > "$site/public/app.css"
( cd "$site" && find . -type f | sort ) > "$work/before.txt"
step() { printf '\n-- %s\n' "$*"; }

step "pin A ($va) with its archive checksum"
ds pin "$va" "$(digest "$a")" "$source_a"
step "install A: obtain, checksum-verify, install"
ds install public
[ -f "$site/public/design-system/core.css" ] || die "core.css was not installed"
step "verify the active pin"
ds verify public

step "a tampered archive is refused before anything is copied"
cp "$a" "$work/tampered.tar.gz"; printf 'x' >> "$work/tampered.tar.gz"
( cd "$site" && rm -rf .design-system-cache public/design-system )
cp "$site/design-system.pin" "$work/pin.saved"
# The pin names archive design-system-<va>.tar.gz; give the tampered copy that name.
mkdir -p "$work/tamper"; cp "$work/tampered.tar.gz" "$work/tamper/design-system-$va.tar.gz"
printf 'version=%s\narchive=design-system-%s.tar.gz\nsha256=%s\nsource=file://%s\n' "$va" "$va" "$(digest "$a")" "$work/tamper/design-system-$va.tar.gz" > "$site/design-system.pin"
if ds install public > "$work/tamper.out" 2>&1; then die "a tampered archive was installed"; fi
grep -q "checksum mismatch" "$work/tamper.out" || die "wrong failure for a tampered archive: $(cat "$work/tamper.out")"
[ ! -e "$site/public/design-system" ] || die "a tampered archive left an install behind"
printf 'refused: %s\n' "$(cat "$work/tamper.out")"
cp "$work/pin.saved" "$site/design-system.pin"
ds install public >/dev/null
ds verify public >/dev/null

step "an edited installed file is caught by verify"
printf '/* edit */\n' >> "$site/public/design-system/base.css"
if ds verify public > "$work/edit.out" 2>&1; then die "verify accepted an edited file"; fi
printf 'caught: %s\n' "$(cat "$work/edit.out")"
ds install public >/dev/null
ds verify public

step "upgrade: pin B ($vb), install, verify"
ds pin "$vb" "$(digest "$b")" "$source_b"
ds install public
ds verify public
[ "$(sed -n 's/^version=//p' "$site/design-system.pin")" = "$vb" ] || die "the active pin is not B"
[ "$(sed -n 's/^version=//p' "$site/design-system.pin.previous")" = "$va" ] || die "the previous pin is not A"
[ "$(sed -n 's/^version=//p' "$site/public/design-system/.ds-pin")" = "$vb" ] || die "the installed pin is not B"

step "restore: back to the previous pin (A)"
ds restore public
ds verify public
[ "$(sed -n 's/^version=//p' "$site/design-system.pin")" = "$va" ] || die "restore did not return to A"
[ "$(sed -n 's/^version=//p' "$site/public/design-system/.ds-pin")" = "$va" ] || die "the installed pin is not A after restore"
[ ! -e "$site/design-system.pin.previous" ] || die "the previous pin file was not consumed"

step "remove: the install and cache go, the consumer's own files stay"
ds remove public
[ ! -e "$site/public/design-system" ] && [ ! -e "$site/.design-system-cache" ] || die "remove left files behind"
[ -f "$site/public/index.html" ] && [ -f "$site/public/app.css" ] || die "remove touched the consumer's own files"

step "restore with no previous pin: the pre-Design-System state"
ds install public >/dev/null
ds restore public
( cd "$site" && find . -type f | sort ) > "$work/after.txt"
diff "$work/before.txt" "$work/after.txt" || die "the consumer is not back to its pre-Design-System state"
printf 'pre-Design-System state restored: %s file(s), identical to the start\n' "$(wc -l < "$work/after.txt" | tr -d ' ')"

step "refuses to touch a directory it did not install"
mkdir -p "$site/public/design-system"; printf 'mine\n' > "$site/public/design-system/mine.css"
ds pin "$va" "$(digest "$a")" "$source_a"
if ds install public > "$work/own.out" 2>&1; then die "install replaced a directory it did not create"; fi
[ -f "$site/public/design-system/mine.css" ] || die "the consumer's own directory was damaged"
printf 'refused: %s\n' "$(cat "$work/own.out")"

printf '\nconsumer proof passed: pin, obtain, checksum, install, verify, upgrade, restore, remove, and pre-Design-System restore, on a PATH of %s\n' "$(ls "$bin" | tr '\n' ' ')"

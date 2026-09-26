#!/bin/sh
# Assemble the Design System release archive from an exact clean checkout.
#
#   release/assemble.sh --rehearsal <n> [--out <dir>]
#   release/assemble.sh --release --commit <full-sha> [--out <dir>]
#
# Run from the repository root. Needs only git, gzip, and POSIX shell tools: no Rust,
# Node, Tailwind, credentials, or network.
#
# A rehearsal builds version <core>-rehearsal.<n> and marks every identity file and
# document as a rehearsal; it has no tag and must never be published. A release
# builds the version in release/identity.toml and requires the caller to name the
# exact commit, which must be the checked-out HEAD of a clean tree.
#
# The tar is byte-reproducible: files come from a sorted Git tree, owner and group are
# zero, and every mtime is the commit time. The gzip wrapper is not required to be
# byte-identical across gzip builds; compare the tar checksum, which is recorded
# beside the archive checksum in BUILD-ENV.txt.
set -eu

die() { printf 'assemble: %s\n' "$*" >&2; exit 1; }

[ -f release/identity.toml ] && [ -f release/inventory.tsv ] || die "run from the repository root"
tab=$(printf '\t')

if command -v sha256sum >/dev/null 2>&1; then
  digest() { sha256sum "$1" | cut -d ' ' -f 1; }
else
  digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
fi
toml() { sed -n "s/^$1 = \"\(.*\)\"\$/\1/p" release/identity.toml | head -n 1; }

mode=
number=
commit_arg=
out=dist
while [ $# -gt 0 ]; do
  case "$1" in
    --rehearsal) mode=rehearsal; number=${2:?--rehearsal needs a number}; shift 2 ;;
    --release) mode=release; shift ;;
    --commit) commit_arg=${2:?--commit needs a sha}; shift 2 ;;
    --out) out=${2:?--out needs a directory}; shift 2 ;;
    *) die "unknown argument: $1" ;;
  esac
done
[ -n "$mode" ] || die "choose --rehearsal <n> or --release --commit <sha>"

product=$(toml product); base_version=$(toml version); tag_value=$(toml tag)
maturity=$(toml maturity); license=$(toml license); repository=$(toml repository)
channel=$(toml channel); format=$(toml archive_format)
[ "$maturity" = preview ] || die "identity maturity must be preview for this candidate"
[ "$license" = MIT ] || die "identity license must be MIT"

commit=$(git rev-parse HEAD)
[ -z "$(git status --porcelain)" ] || die "the working tree is not clean; the archive identity would not match its commit"

case "$mode" in
  release)
    [ "$commit_arg" = "$commit" ] || die "--commit must equal the checked-out HEAD $commit"
    version=$base_version
    tag=$tag_value
    ;;
  rehearsal)
    case "$number" in ""|*[!0-9]*) die "rehearsal number must be digits" ;; esac
    version=${base_version%%-*}-rehearsal.$number
    tag=none
    ;;
esac
name=design-system-$version

# Substitute release identity placeholders. A rehearsal banner marks a rehearsal.
if [ "$mode" = rehearsal ]; then
  notice='> **REHEARSAL BUILD. This is not a release. Do not tag, publish, or pin it.**'
else
  notice=''
fi
substitute() {
  sed -e "s|@VERSION@|$version|g" -e "s|@TAG@|$tag|g" -e "s|@COMMIT@|$commit|g" \
      -e "s|@IDENTITY_KIND@|$mode|g" -e "s|@REHEARSAL_NOTICE@|$notice|g" "$1"
}

case "$out" in /*) ;; *) out=$(pwd)/$out ;; esac
mtime=$(git log -1 --format=%cI HEAD)
stage=$out/stage
root=$stage/$name
rm -rf "$stage" "$out/scratch.git" "$out/$name.tar" "$out/$name.tar.gz"
mkdir -p "$root"

: > "$stage/.classes"
grep -v '^[[:space:]]*#' release/inventory.tsv | grep -v '^[[:space:]]*$' | while IFS="$tab" read -r class kind archive source transform; do
  [ -f "$source" ] || die "inventory source is missing: $source"
  # Only committed content may enter the archive: with a clean tree, a tracked file is HEAD's.
  git ls-files --error-unmatch -- "$source" >/dev/null 2>&1 || die "inventory source is not tracked by Git: $source"
  mkdir -p "$root/$(dirname "$archive")"
  case "$transform" in
    none) cp "$source" "$root/$archive" ;;
    identity) substitute "$source" > "$root/$archive" ;;
    tailwind-core-import)
      [ "$(grep -c '^@import "\.\./\.\./packages/styles/index\.css";$' "$source")" = 1 ] || die "$source must hold exactly one repository-relative core import"
      sed 's|^@import "\.\./\.\./packages/styles/index\.css";$|@import "./core.css";|' "$source" > "$root/$archive"
      ;;
    *) die "unknown transform $transform for $archive" ;;
  esac
  if [ "$kind" = tool ]; then chmod 755 "$root/$archive"; else chmod 644 "$root/$archive"; fi
  printf '%s\t%s\t%s\n' "$archive" "$class" "$kind" >> "$stage/.classes"
done

{
  printf 'product\t%s\nversion\t%s\ntag\t%s\nidentity\t%s\nmaturity\t%s\nlicense\t%s\n' \
    "$product" "$version" "$tag" "$mode" "$maturity" "$license"
  printf 'source_repository\t%s\nsource_commit\t%s\nchannel\t%s\narchive_format\t%s\n' \
    "$repository" "$commit" "$channel" "$format"
} > "$root/IDENTITY.tsv"
chmod 644 "$root/IDENTITY.tsv"
printf 'IDENTITY.tsv\tdocumentation\tidentity\n' >> "$stage/.classes"

# MANIFEST.tsv: <sha256> <size> <class> <kind> <path>, sorted by path, without itself.
LC_ALL=C sort "$stage/.classes" | while IFS="$tab" read -r archive class kind; do
  printf '%s\t%s\t%s\t%s\t%s\n' "$(digest "$root/$archive")" "$(wc -c < "$root/$archive" | tr -d ' ')" "$class" "$kind" "$archive"
done > "$root/MANIFEST.tsv"
chmod 644 "$root/MANIFEST.tsv"
rm -f "$stage/.classes"

# Deterministic tar through a scratch Git tree: sorted names, zero owner, commit-time mtimes.
# The scratch repository ignores the caller's Git configuration (line-ending conversion,
# tar umask, attributes) and the gzip wrapper ignores the GZIP environment variable, so
# no local setting can change the bytes.
scratch=$out/scratch.git
git init -q --bare "$scratch"
isolated() { env GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null GIT_CONFIG_NOSYSTEM=1 GIT_DIR="$scratch" "$@"; }
tree=$(cd "$stage" && isolated GIT_WORK_TREE="$stage" git -c core.autocrlf=false -c core.filemode=true add -A -f . >/dev/null && isolated GIT_WORK_TREE="$stage" git write-tree)
isolated git -c tar.umask=0002 archive --format=tar --mtime="$mtime" "$tree" > "$out/$name.tar"
rm -rf "$scratch"
GZIP= gzip -n -9 -c "$out/$name.tar" > "$out/$name.tar.gz"
cp "$root/consumer/ds-consumer.sh" "$out/ds-consumer.sh"

tar_sha=$(digest "$out/$name.tar")
archive_sha=$(digest "$out/$name.tar.gz")
script_sha=$(digest "$out/ds-consumer.sh")
printf '%s  %s\n%s  %s\n' "$archive_sha" "$name.tar.gz" "$script_sha" "ds-consumer.sh" > "$out/SHA256SUMS"
{
  printf 'identity=%s\nversion=%s\ntag=%s\nsource_commit=%s\nsource_tree=%s\n' "$mode" "$version" "$tag" "$commit" "$(git rev-parse HEAD^{tree})"
  printf 'mtime=%s\ntar_sha256=%s\narchive_sha256=%s\nds_consumer_sha256=%s\n' "$mtime" "$tar_sha" "$archive_sha" "$script_sha"
  printf 'tool_git=%s\ntool_gzip=%s\ntool_sh=%s\ntool_os=%s\n' \
    "$(git --version)" "$(GZIP= gzip --version 2>&1 | head -n 1)" "$(readlink /bin/sh 2>/dev/null || echo sh)" "$(uname -sm)"
} > "$out/BUILD-ENV.txt"

printf 'assembled %s (%s)\n  tar     %s  %s\n  archive %s  %s\n' "$name" "$mode" "$tar_sha" "$name.tar" "$archive_sha" "$name.tar.gz"

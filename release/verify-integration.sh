#!/bin/sh
# Phase B check: the merged main commit carries exactly the reviewed source tree.
#
#   release/verify-integration.sh <reviewed-head-sha> <merged-main-sha>
#
# A squash merge gives main a new commit, so the commit ids differ; what must be
# identical is the whole-repository tree. This proves the release will be assembled from
# the reviewed content. Run from a clone that contains both commits.
set -eu

die() { printf 'verify-integration: %s\n' "$*" >&2; exit 1; }
[ $# -eq 2 ] || die "usage: verify-integration.sh <reviewed-head-sha> <merged-main-sha>"
for sha in "$1" "$2"; do
  case "$sha" in *[!0-9a-f]*|"") die "$sha is not a full lowercase sha" ;; esac
  [ "${#sha}" -eq 40 ] || die "$sha is not a full sha"
  git cat-file -e "$sha^{commit}" 2>/dev/null || die "$sha is not a commit in this clone"
done
reviewed=$(git rev-parse "$1^{tree}")
merged=$(git rev-parse "$2^{tree}")
[ "$reviewed" = "$merged" ] || die "the trees differ: reviewed $reviewed, merged $merged"
printf 'integration verified: reviewed %s and merged %s have the identical tree %s\n' "$1" "$2" "$reviewed"

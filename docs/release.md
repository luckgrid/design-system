# Release

How the Design System release archive is defined, assembled, verified, and (by a
maintainer, separately) published. Consumers do not need this page; they need
[`release/consumer/README.md`](/release/consumer/README.md).

## Identity

One exact merged `main` commit, one version, one immutable tag.

| Authority | File |
|---|---|
| Version, tag, maturity, license, channel | [`release/identity.toml`](/release/identity.toml) |
| What the archive contains | [`release/inventory.tsv`](/release/inventory.tsv) |
| Release notes shipped in the archive | [`release/RELEASE.md`](/release/RELEASE.md) |
| Consumer instructions shipped in the archive | [`release/consumer/README.md`](/release/consumer/README.md) |

The product maturity is `preview` and the version carries a `-preview.<n>`
pre-release. `ds-check inventory` rejects `stable`, a `1.0.0` or later version, a
registry channel, a version that disagrees with `Cargo.toml`, and a license that
disagrees with `LICENSE`, the descriptor, or the docs. Maturity and per-surface
compatibility stay separate: every consumer-facing surface is `public-preview`.

## The archive

`design-system-<version>.tar.gz` unpacks to one directory, `design-system-<version>/`:

| Path | What |
|---|---|
| `css/core.css` and the stylesheets it imports | The browser-ready CSS. Link `core.css`. |
| `css/tailwind.css` | Optional Tailwind v4 adapter; its one repository-relative import becomes `./core.css`. |
| `surface/*.tsv` | The public surface: exports, semantic tokens, theme, base, layouts, primitives, Tailwind projection. |
| `docs/` | Contract documentation and the browser support statement. |
| `consumer/ds-consumer.sh` | The pin, install, verify, remove, and restore script. |
| `LICENSE`, `README.md`, `RELEASE.md` | License, consumer instructions, release notes. |
| `IDENTITY.tsv`, `MANIFEST.tsv` | Version, commit, maturity, license; sha256, size, class, and kind of every other file. |

## Commands

All are run from the repository root of a clean checkout. Ordinary build, test, and
verification are credential-free; nothing here publishes.

```sh
# Source-side proof of the inventory, identity, and license metadata.
cargo run --locked -p design-system-check -- inventory \
  release/inventory.tsv release/identity.toml exports.tsv adapter-exports.tsv bootstrap-surfaces.tsv

# Assemble. A rehearsal is version <core>-rehearsal.<n>, is marked as one throughout, and
# has no tag. A release names the exact commit and must be the checked-out HEAD.
sh release/assemble.sh --rehearsal 1 --out dist
sh release/assemble.sh --release --commit "$(git rev-parse HEAD)" --out dist

# Verify the archive as a consumer receives it (unpacked outside the repository).
sh release/verify-archive.sh dist/design-system-<version>.tar.gz --source .

# Assemble the same commit from two independent clean clones and compare.
sh release/reproduce.sh --source "$(pwd)" --commit "$(git rev-parse HEAD)" --rehearsal 1

# After the reviewed pull request is squash-merged: the merged main commit must carry
# exactly the reviewed tree, so the release is assembled from reviewed content.
sh release/verify-integration.sh <reviewed-head-sha> <merged-main-sha>

# Prove the verifier rejects what it must (mutation probes).
sh release/negative-probes.sh <unpacked archive directory>

# Prove the packaged form: the whole browser suite, the Tailwind fixture, the static
# renderer fixture, the final load path, and the @import layer-order race, all against
# the unpacked archive with every stylesheet served from it.
sh release/verify-packaged.sh dist/design-system-<version>.tar.gz

# Prove pin, install, verify, upgrade, restore, remove, and pre-Design-System restore
# with a PATH that holds no Rust, Node, or Tailwind.
sh release/consumer-proof.sh dist/design-system-<previous>.tar.gz dist/design-system-<version>.tar.gz
```

## Reproducibility

The tar is byte-reproducible: files come from a sorted Git tree, owner and group
are zero, and every modification time is the commit time. Two clean clones
assembled from one commit produce the same tar checksum, the same unpacked tree,
and the same `MANIFEST.tsv` and `IDENTITY.tsv`.

Legitimately nondeterministic, and only these:

- the gzip wrapper bytes, and so the `.tar.gz` checksum, if two assemblies use
  different gzip builds (compare the tar checksum instead; on one machine the
  wrapper matches too);
- the `tool_*` lines of `BUILD-ENV.txt`, which record the tools that ran;
- temporary directory paths, which appear in no archive file.

The published archive checksum is the one in `SHA256SUMS` on the release; consumers
pin that value.

## Privilege boundary

Assembly, verification, and every check above need no credentials. Publication is
a separate, least-privilege maintainer step performed only after the merged commit
has been assembled and verified from a clean checkout of that exact commit:

1. create the immutable tag `v<version>` on that commit and push it;
2. create the GitHub Release for the tag and attach `design-system-<version>.tar.gz`,
   `SHA256SUMS`, and `ds-consumer.sh`;
3. re-run `release/verify-archive.sh` and `release/consumer-proof.sh` against the
   published assets (`DS_PROOF_SOURCE_A` and `DS_PROOF_SOURCE_B` take their https URLs).

CI never tags or publishes. A rehearsal is never tagged or published.

## Rollback

Consumers roll back with `ds-consumer.sh restore`: the previous pin returns and is
reinstalled and verified; with no previous pin the project returns to its
pre-Design-System state. Maintainers never move or delete a published tag. A bad
release is superseded by a new version, and the release notes say which version to
pin instead.

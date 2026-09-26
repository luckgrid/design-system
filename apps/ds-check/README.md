# `ds-check`

`ds-check` is the repository's Rust validation CLI. It checks tracked-file
classification, CSS layer ownership, tokens, theme behavior, classless base
ownership, layouts, primitives, public-hook reach, and the static-renderer
fixture's staged public-export boundary and coupling guards. `ds-check inventory`
proves the release inventory, identity, and license metadata against the declared
exports; `ds-check release <unpacked archive> [source]` proves an unpacked release
archive: checksums, packaged imports, the public surface tables against the shipped
CSS, documentation links, and a privacy scan. See [docs/release.md](/docs/release.md).

Run the complete command set from the repository root as documented in
[CONTRIBUTING.md](/CONTRIBUTING.md). The tool is contributor infrastructure;
it is not part of the CSS runtime or a requirement for consumers.

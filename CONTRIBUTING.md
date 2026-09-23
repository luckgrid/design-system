# Contributing

The Design System is in experimental bootstrap and is currently unlicensed.
Public visibility is not permission to reuse or redistribute the source.

## Toolchain

Install Rust 1.98.1 with `rustfmt` and `clippy`, or let `rustup` honor
`rust-toolchain.toml`.

Run the bootstrap quality checks from the repository root of a Git checkout.
The bootstrap inventory completeness check uses `git ls-files` as the tracked
source authority:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo +1.98.1 check --workspace --all-targets --locked
cargo run --locked -p design-system-check -- check \
  bootstrap-surfaces.tsv fixtures/plain-html
cargo run --locked -p design-system-check -- layers \
  exports.tsv bootstrap-surfaces.tsv fixtures/plain-html
cargo run --locked -p design-system-check -- tokens \
  tokens.tsv exports.tsv docs/architecture/tokens.md \
  fixtures/plain-html fixtures/brand-theme fixtures/layouts fixtures/primitives \
  fixtures/scoping fixtures/static-renderer/consumer tests/browser/probes
cargo run --locked -p design-system-check -- theme \
  theme.tsv exports.tsv docs/architecture/theme.md \
  fixtures tests/browser/probes
cargo run --locked -p design-system-check -- base \
  base.tsv exports.tsv docs/architecture/base.md fixtures/plain-html
cargo run --locked -p design-system-check -- layout \
  layouts.tsv exports.tsv docs/architecture/layouts.md fixtures/layouts
cargo run --locked -p design-system-check -- primitive \
  primitives.tsv layouts.tsv exports.tsv docs/architecture/primitives.md \
  fixtures/primitives
cargo run --locked -p design-system-check -- hooks \
  layouts.tsv primitives.tsv theme.tsv exports.tsv docs/architecture/hooks.md \
  fixtures/scoping
sh fixtures/static-renderer/stage.sh
sh fixtures/static-renderer/build.sh
sh fixtures/static-renderer/verify-toolchain-free.sh
cargo run --locked -p design-system-check -- static-renderer \
  exports.tsv layouts.tsv primitives.tsv theme.tsv base.tsv fixtures/static-renderer
```

The static-renderer fixture needs the exact Hugo version in
`fixtures/static-renderer/HUGO_VERSION` and POSIX shell tools; it needs no Rust,
Node, or Tailwind to stage or render.

Browser contract tests need Node 20 or later and live only in `tests/browser/`.
Node is not a root workspace tool:

```sh
cd tests/browser
npm ci
npx playwright install chromium firefox webkit
npx playwright test
```

A browser check that is skipped or cannot run is not a pass.

## Pull requests

Keep changes bounded and explain compatibility impact for any surface that is
classified beyond `internal`. Do not promote consumer-specific behavior into
shared source without evidence from materially unlike consumers.

The supported CSS surfaces are the `public-preview` `core.css` entrypoint
declared in `exports.tsv`, the `public-preview` semantic token roles in
`tokens.tsv`, the theme default and hook in `theme.tsv`, the classless base
defaults in `base.tsv`, the layout hooks in `layouts.tsv`, and the primitive hooks and states in `primitives.tsv`. Changes to the entrypoint, its layer order, what `exports.tsv`
declares, or a public role's name, meaning, value type, or alias relationship
are compatibility changes and must say so. So are changes to a base default or
to a layout hook or its documented contract, and to the public hook vocabulary
and scoping boundary in `docs/architecture/hooks.md`. Reference tokens (`--ds-ref-*`) are
internal. Other files in `packages/styles/` stay `internal` until a reviewed
source change defines and tests a public contract for them.

For vulnerabilities, follow [SECURITY.md](SECURITY.md) rather than publishing
sensitive details in an issue.

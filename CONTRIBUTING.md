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
```

Browser contract tests need Node 20 or later and live only in `tests/browser/`.
Node is not a root workspace tool:

```sh
cd tests/browser
npm ci
npx playwright install chromium firefox webkit
npx playwright test
```

A browser check that is skipped or cannot run is not a pass.

## Program references

Use the complete program identifier when durable documentation refers to a story
or task, for example `DS-E01.S1.T3`. Prefer the durable task/story name when it
reads more clearly. Do not use bare references such as `T3`, `S2.T1`, or `S2`
outside a table or list whose parent identifier is explicit in the same entry.

## Pull requests

Keep changes bounded and explain compatibility impact for any surface that is
classified beyond `internal`. Do not promote consumer-specific behavior into
shared source without evidence from materially unlike consumers.

The only supported CSS surface is the `public-preview` `core.css` entrypoint
declared in `exports.tsv`. Changes to it, to its layer order, or to what
`exports.tsv` declares are compatibility changes and must say so. Other files in
`packages/styles/` stay `internal` until an owning DS-E01 task accepts a public
contract for them.

For vulnerabilities, follow [SECURITY.md](SECURITY.md) rather than publishing
sensitive details in an issue.

# Design System @VERSION@

@REHEARSAL_NOTICE@

This archive is release identity **@IDENTITY_KIND@**: version `@VERSION@`, source commit
`@COMMIT@`, tag `@TAG@`. Product maturity is `preview`; every surface below is
`public-preview`, and none is `public-stable`. Licensed under the MIT License (see
`LICENSE`).

## The browser-ready CSS

The artifact you link is one plain stylesheet:

```text
css/core.css
```

It imports the other files in `css/` with relative `./` paths, so keep the whole
`css/` directory together. No Rust, Node, Tailwind, package manager, build step, or
Design System source checkout is needed to use it.

```html
<link rel="stylesheet" href="/design-system/core.css">
<link rel="stylesheet" href="/app.css">
```

Load `core.css` first. Declare your own layers after it (`@layer app;`) so your
overrides win without specificity escalation. See `docs/architecture/css-entrypoint.md`.

`css/tailwind.css` is an optional Tailwind v4 adapter. Only consumers who already
build with Tailwind v4 need it; it is not part of the plain path.

## Install without any tool beyond shell, tar, and a checksum command

1. Download `design-system-@VERSION@.tar.gz` and `SHA256SUMS` from the release for
   tag `@TAG@`.
2. Verify the archive, checking only its own line of `SHA256SUMS`:
   `grep 'design-system-@VERSION@.tar.gz' SHA256SUMS | shasum -a 256 -c -` (or `sha256sum -c -`).
3. Unpack it: `tar -xzf design-system-@VERSION@.tar.gz`.
4. Copy the CSS into your project: `cp -R design-system-@VERSION@/css/. your-project/design-system/`.
5. Link `design-system/core.css` as shown above.

## Install with the pin script (repeatable, verifiable, removable)

`consumer/ds-consumer.sh` is POSIX shell and is also published beside the archive.
Run it from your project root.

```sh
# pin the exact release: version, archive sha256 (from SHA256SUMS), and where to fetch it
sh ds-consumer.sh pin @VERSION@ <archive-sha256> https://github.com/luckgrid/design-system/releases/download/@TAG@/design-system-@VERSION@.tar.gz

sh ds-consumer.sh install public      # verifies the checksum and every file, installs public/design-system/
sh ds-consumer.sh verify public       # confirms the installed files match the active pin
sh ds-consumer.sh remove public       # deletes the install and the download cache
sh ds-consumer.sh restore public      # back to the previous pin; with none, the pre-design-system state
sh ds-consumer.sh purge public        # remove, and delete both pin files
```

A checksum mismatch stops the install before anything is copied. A source may also
be a `file://` URL or a local path. The script refuses to remove or replace a
directory it did not install.

To upgrade, `pin` the new version (the old pin is kept as `design-system.pin.previous`),
`install`, and `verify`. To roll back, `restore`.

## What is in the archive

`MANIFEST.tsv` lists every file with its sha256, size, compatibility class, and kind.
`IDENTITY.tsv` records the version, source commit, maturity, and license. `surface/`
holds the machine-readable public surface: exports, semantic tokens, theme, base,
layout and primitive hooks. `docs/` holds the contract documentation and the browser
support statement. `RELEASE.md` holds the release notes, known limitations, migration
notes, and the feedback route.

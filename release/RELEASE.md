# Release notes — Design System @VERSION@

@REHEARSAL_NOTICE@

| | |
|---|---|
| Version | `@VERSION@` |
| Tag | `@TAG@` |
| Source commit | `@COMMIT@` |
| Identity | @IDENTITY_KIND@ |
| Product maturity | `preview` |
| License | MIT (see `LICENSE`) |
| Channel | immutable source tag and GitHub Release archive; no package registry |

## Maturity and compatibility are separate

`preview` describes the whole product: it can change between preview releases. Each
surface has its own class. Every consumer-facing surface in this release is
`public-preview` (deliberately exposed, still moving). **No surface is
`public-stable`**, and this release does not make one so. Repository paths and
anything not listed in `MANIFEST.tsv` are not API.

| Surface | Class | Where |
|---|---|---|
| `core.css` and the stylesheets it imports | `public-preview` | `css/` |
| `tailwind.css` optional adapter | `public-preview` | `css/tailwind.css` |
| Semantic `--ds-*` roles | `public-preview` | `surface/tokens.tsv` |
| Theme default and `data-ds-scheme` hook | `public-preview` | `surface/theme.tsv` |
| Classless base subjects | `public-preview` | `surface/base.tsv` |
| Layout and primitive class hooks and states | `public-preview` | `surface/layouts.tsv`, `surface/primitives.tsv` |
| Cascade layer order under `ds` | `public-preview` | `docs/architecture/css-entrypoint.md` |
| `--ds-ref-*` reference values and nested sub-layers | `internal` | not a compatibility surface |

## Browser support

Read `docs/browser-support.md`. In short: desktop Chromium 123+ and Edge 123+
(verified on macOS at the floor version; the executed floor browser was Chrome for
Testing 123.0.6312.122, the exact Chromium engine but not branded Google Chrome, and
branded Google Chrome 123 was not executed; Edge 123.0.2420.97 was executed), and
Firefox 121+ with a stated qualification. Windows desktop,
Linux desktop, Windows Edge, Chrome Android, Firefox Android, and Safari on macOS,
iOS, and iPadOS are unverified, not excluded.

## Known limitations

- Preview surfaces can change; see the policy below.
- Components and utilities layers are empty; no component API exists.
- Firefox 121 keyboard evidence depends on the `accessibility.tabfocus` setting; see
  the browser support statement for exactly what was observed.
- Forced-colors emulation evidence is engine-specific and partial; see the browser
  support statement.
- The Tailwind adapter needs Tailwind v4 in the consumer's build; the plain path does not.

## Migration notes

This is the first release. There is nothing to upgrade from. A project that linked
repository source paths such as `packages/styles/index.css` must instead pin this
release and link `design-system/core.css`; the CSS content and layer contract are the
same files. Each later preview release will list any breaking change here with a
migration note.

## Provisional breaking-change and deprecation rules (preview horizon)

These rules cover only the preview horizon and do not pre-empt the project's later
governance policy.

- A `public-preview` surface may change or be removed in any preview release.
- Every such change is listed in that release's notes under "Breaking changes", with
  a migration note. Where practical a surface is announced as deprecated in the
  release before it is removed; this is a courtesy, not a guarantee.
- `internal` material carries no promise and is never listed.
- Consumers pin an exact version. Nothing floats.

## Support, contribution, and security

- Support is best effort through GitHub issues; there is no service-level promise.
- Only the latest preview release is considered current. There are no backports.
- Contributions follow `CONTRIBUTING.md` in the source repository.
- Report suspected vulnerabilities privately through the source repository's GitHub
  private vulnerability reporting flow, not in a public issue.

## Feedback

Open an issue in the source repository and choose the matching form:

| Kind | Use it for |
|---|---|
| Defect or regression | The CSS behaves differently from its documented contract, or a release changed behavior. |
| Missing semantic role or hook | You need a role or hook the public surface does not offer. |
| Portability or install failure | The pin, checksum, install, or link path did not work in your environment. |
| Accessibility issue | An accessibility problem in the shipped CSS. |
| Feature request | Anything else you would like the Design System to do. |

Feedback is evidence to triage. Reporting a need does not by itself add, promote, or
guarantee any public API.

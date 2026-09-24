# Generated HTML conformance

This contributor-only check validates the generated static-renderer pages after
`fixtures/static-renderer/build.sh` has run. It pins `html-validate` 9.4.0 and
keeps the command deterministic over every generated HTML page.

The profile disables formatting/preference rules that are not the E01 claim
(doctype casing, trailing whitespace, button preference, implicit button type,
and the validator's `aria-label` preference). Structural and remaining ARIA
validation stays enabled. This is representative generated-HTML evidence, not
a universal claim for every consumer page and not a substitute for the manual
AT pass.

```sh
npm ci
npm run validate
```

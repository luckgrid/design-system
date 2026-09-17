# Design System

A portable, web-platform-first design system: CSS-first, semantic-HTML-first,
framework-agnostic, and independently versioned.

> **Status: pre-implementation.** This repository has just been bootstrapped.
> There is no released CSS, no public API, and no supported artifact yet. Nothing
> here is stable. See [Current status](#current-status).

## What this is

A standalone, independently versioned open-source design system, intended to be
consumed as published artifacts rather than by forking or reaching into source
paths. Its first implementation horizon is frontend-focused:

- portable CSS core with an explicit cascade/layer contract;
- semantic and classless base styling for ordinary HTML before specialized hooks;
- reference and semantic token layers, with a consumer-themeable contract;
- base primitives and semantic UI primitives, kept distinct;
- bounded fluid spacing and type where interpolation is genuinely intended;
- container-first responsiveness for reusable modules;
- native HTML/CSS interaction before shared JavaScript, where the accepted
  browser and accessibility policy permits it.

Rust and Cargo are the workspace and contributor-tooling substrate. **Consuming
the released CSS will not require Rust**, Cargo, or this repository's toolchain.

## What this is not

- Not a component framework for any specific UI runtime. No React, Solid, or Vue
  runtime is required to consume the core styles.
- Not a Tailwind plugin. Tailwind is an optional adapter, neither required nor
  forbidden.
- Not a backend or service framework. Runtime, storage, auth, and domain APIs
  belong to the systems that own them.
- Not a general-purpose UI kit lifted from one product. Shared abstractions are
  promoted only after evidence from genuinely unlike consumers.

## Current status

The repository exists; the implementation does not. In order:

1. inventory and behavioral verification of the existing implementation evidence;
2. consumer, browser, and compatibility inventory;
3. source decisions and workspace bootstrap — exact Cargo members, crate and
   package names, MSRV, license, and release topology;
4. the portable token, theme, cascade, and classless-base core;
5. layouts, primitives, scoping, migration comparison, and refinement.

Until step 3 lands, this repository deliberately declares no license file, no
Cargo manifest, no toolchain pin, and no package names. Those are decisions with
long compatibility consequences, and they are made once, with evidence.

## Stability

Public visibility does not make every path public API. When surfaces begin to
exist they are classified explicitly:

| Class | Meaning |
|---|---|
| `public-stable` | Supported. Compatibility impact is recorded for changes. |
| `public-preview` | Published, still moving. May change without a major bump. |
| `internal` | Not API. May change or disappear at any time. |

Nothing is `public-stable` today. Repository creation implies no stability
promise.

## License

**This repository is currently unlicensed.** It is public and readable, but under
default copyright that means all rights are reserved — it is not yet open source,
and it is not safe to copy, modify, or redistribute.

Open-sourcing it is the intent. Licensing is deliberately part of the source
decisions step above rather than a choice made in passing, because it is
effectively irreversible once consumers exist. A `LICENSE` file will land with
that step, and this section will say so.

Until then, please treat the repository as read-only.

## Contributing

Contribution and security-reporting terms are not yet published. They are part of
the same bootstrap step.

## Provenance

Planning and specification authority for this project lives outside this
repository. See [`WORKSTREAMS.md`](WORKSTREAMS.md).

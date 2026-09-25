# Security

**Do not publish suspected vulnerability details in a public issue.**

## Reporting a vulnerability

Report privately through this repository's GitHub private vulnerability
reporting flow:

1. open the repository's **Security** tab;
2. choose **Report a vulnerability**;
3. describe the issue, affected paths, and reproduction steps.

Private vulnerability reporting is enabled on this repository, so any GitHub
account can open a private advisory draft visible only to maintainers. No
private Luckgrid access and no prior contact are required.

If you cannot use that flow, open a public issue that reports **only** that you
have a security concern and requests a private channel — without exploit
details, affected inputs, or reproduction steps — and a maintainer will respond
with a private route.

## Scope

Reports are welcome for the published release archive, repository tooling,
workflow configuration, and dependency and supply-chain posture.

## Supported versions

Only the latest preview release is considered current. There is no backport
policy for earlier preview releases; a fix ships in a new preview release. A
vulnerability in a preview CSS artifact is fixed by publishing a new version that
consumers pin, not by changing an existing tag.

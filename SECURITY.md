# Security Policy

## Reporting a vulnerability

Please do not report security vulnerabilities through public GitHub issues, discussions, or pull requests.

The repository is currently private. A direct check of GitHub's Private Vulnerability Reporting endpoint returned 404 in that state, so this document does not claim that a private reporting form is available. Authorized collaborators should contact a repository administrator through an existing private collaboration channel. Do not place sensitive details in an ordinary issue.

When the repository becomes public, a maintainer should enable and verify GitHub Private Vulnerability Reporting under **Settings → Code security** before announcing the public release. Once the repository's **Security** tab shows **Report a vulnerability**, use that form. GitHub documents the process in [Privately reporting a security vulnerability](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability).

If the expected private form is unavailable after publicization, use GitHub's repository-owner contact or abuse-reporting channels to request a private route without posting vulnerability details publicly. The project does not invent or publish a personal security address for this purpose.

Include, as far as you can:

- the affected version or commit;
- the affected command or component;
- reproduction steps or a proof of concept;
- the impact you observed; and
- any suggested mitigation.

The maintainers will acknowledge the report as soon as practical, assess it, and keep you informed of material progress. Please allow a reasonable period for a fix before public disclosure. Credit will be given unless you prefer to remain anonymous.

## Scope

The main security surfaces are:

- HTTPS requests to Digitransit and parsing untrusted provider responses;
- local handling of a user-supplied Digitransit subscription key;
- parsing command-line and configuration input; and
- downloadable executables and installers published through GitHub Releases.

The CLI sends credentials only in the `digitransit-subscription-key` request header, redacts them from ordinary configuration output and diagnostics, rejects redirects, and bounds provider response processing. Security reports should still cover any observed failure of those controls.

## Local secret scanning

The repository intentionally has no automatic push or pull-request secret-scan
job. Before merging or releasing, maintainers scan the complete tracked Git
history locally with a current Gitleaks installation:

```sh
gitleaks git --redact --no-banner .
```

Investigate every finding. If a real credential entered the repository, rotate
or revoke it first, then remove it from the repository and history as needed.
Never copy an unredacted finding into an issue, pull request, transcript, or log.

## Supported versions

No GitHub Release has been published yet. After v1.0.0 is released, the latest release will receive security fixes. This section will be updated if support expands to additional release lines.

## Safe harbor

We consider good-faith security research conducted under this policy to be authorized. We will not pursue or support legal action against researchers who act in good faith, avoid privacy violations and service disruption, and give us a reasonable time to respond before disclosure.

# Security Policy

## Reporting a vulnerability

Please do not report security vulnerabilities through public GitHub issues, discussions, or pull requests.

Report sensitive vulnerabilities through [GitHub Private Vulnerability Reporting](https://github.com/jarimustonen/reitti-cli/security/advisories/new). Do not include sensitive details in an ordinary issue. If the private reporting form is unavailable, contact the repository owner through an existing private channel to arrange a report.

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

The latest 1.x release receives security fixes. Update to the latest release before reporting an issue that may already be fixed.

## Safe harbor

We consider good-faith security research conducted under this policy to be authorized. We will not pursue or support legal action against researchers who act in good faith, avoid privacy violations and service disruption, and give us a reasonable time to respond before disclosure.

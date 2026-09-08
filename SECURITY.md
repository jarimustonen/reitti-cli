# Security Policy

## Reporting a vulnerability

Please do not report security vulnerabilities through public GitHub issues, discussions, or pull requests.

### While the repository is private

Access is restricted, and GitHub Private Vulnerability Reporting is not currently available for this repository. Authorized collaborators should contact a repository administrator through an existing private collaboration channel. The project does not publish a fallback security email, so do not place sensitive details in a public or ordinary issue.

### Before and after publicization

Before changing repository visibility, a maintainer must enable and verify GitHub Private Vulnerability Reporting. Once the repository's **Security** tab shows **Report a vulnerability**, use that private form. GitHub documents the process in [Privately reporting a security vulnerability](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability).

The publicization check must update this section if the reporting channel differs. The presence of this document alone does not prove that the GitHub setting is enabled.

Include, as far as you can:

- the affected version or commit;
- the affected command or component;
- reproduction steps or a proof of concept;
- the impact you observed; and
- any suggested mitigation.

The maintainers will acknowledge the report as soon as practical, assess it, and keep you informed of material progress. Please allow a reasonable period for a fix before public disclosure. Credit will be given unless you prefer to remain anonymous.

## Scope

The most relevant security surfaces are:

- HTTPS requests to Digitransit services and parsing their responses;
- handling a user-supplied Digitransit subscription key;
- parsing command-line and configuration input; and
- downloadable executables and installers published with future GitHub Releases.

The current implementation is an early placeholder. This policy describes the intended v1.0 surface without claiming that release binaries or a production service exist today.

## Supported versions

No supported version has been released yet. Once v1.0 is available, the latest release will receive security fixes. This section will be updated if support expands to additional release lines.

## Safe harbor

We consider good-faith security research conducted under this policy to be authorized. We will not pursue or support legal action against researchers who act in good faith, avoid privacy violations and service disruption, and give us a reasonable time to respond before disclosure.

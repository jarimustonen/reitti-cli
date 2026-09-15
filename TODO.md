# Work-session handoff

## 🔄 Continue here

The public **v1.0.0** release remains installed and available through crates.io, GitHub Releases, the project Homebrew tap, and the generated shell installer. Its tag points to `2aeb5e1be33a29de08d805f40fd266c6f73f731c`.

The post-v1.0 work is complete on clean, pushed `main`:

- a missing Digitransit credential now directs agents to the bundled setup guidance;
- a bare `reitti` invocation shows readable root help on stdout and exits successfully instead of printing escaped newlines as an error; and
- `reitti stop show <STOP_ID>` returns bounded stop details, HSL zone, nullable parent station, and a verified `reittiopas_url`. Stop-search and departure stop objects carry the same human-facing link.

Issue `add-stop-details` is closed as done. Its `validation.md` records deployed Digitransit schema evidence, the official HSL web-route evidence and stability caveat, and sanitized bounded live acceptance for an ordinary stop and a station platform. The implementation run `01m2g181743zj5xxjt7376qjx6` landed successfully with no preserved work. Independent review completed with no unresolved product decision. The conductor reran all repository gates successfully: formatting, strict Clippy, all workspace tests, 23 credential-free provider fixtures, and issue metadata validation. The reservation-aware issue DAG was empty at terminal verification.

The maintainer established a standing release policy in `AGENTS.md`, `OSS-RELEASE.md`, and `docs/DISTRIBUTION.md`: after an explicit stint and completed `/stint-handoff`, an agent may autonomously decide and execute a release without another confirmation when the reservation-aware issue DAG is completely empty. This authorizes the decision only; all release safety, secret scanning, SemVer/changelog, distribution, live-smoke, and publication verification gates remain mandatory. Deployment is a separate action after handoff, never part of `/stint-handoff` itself.

The maintainer explicitly requested deployment after this handoff. The next action is to prepare and publish **v1.1.0** through the existing gated release workflow because the new `stop show` command is a backward-compatible feature. Preserve the crates.io publication order (`reitti-core` before `reitti-cli`), cargo-dist targets, Homebrew delegation, provenance/attestation checks, and the required self-hosted macOS ARM64 runner. Do not reinstall from the local checkout as machine setup.

No test-account reset is configured for this CLI project.

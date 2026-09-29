# Work-session handoff

## 🔄 Continue here

The public **v1.1.0** release is complete and verified across crates.io, GitHub Releases, the project Homebrew tap, and the generated shell installer. Tag `v1.1.0` resolves to release source `7858af1ef2340477f276a4a9dbe89396463d54b3`. Later documentation commits on clean, pushed `main` do not change that binary identity.

The release contains the completed post-v1.0 work:

- a missing Digitransit credential directs agents to the bundled setup guidance;
- a bare `reitti` invocation prints readable root help on stdout and exits successfully instead of emitting escaped newlines as an error; and
- `reitti stop show <STOP_ID>` returns bounded stop details, HSL zone, nullable parent station, and a verified `reittiopas_url`; stop-search and departure stop objects carry the same human-facing link.

Release preparation run `01m36fqhxrecstwcsxmxkmrkt3` landed with no preserved work. The conductor then passed formatting, strict Clippy, all workspace tests, 23 credential-free provider fixtures, issue metadata validation, contract/readiness checks, a release-time Gitleaks history scan, and bounded live online acceptance for an ordinary stop and a station platform. Shipshape plan `84656978f18158154280a7e614cb03d18cb1eebf1d025cfe742cb582948eae05` completed as release run `01M36GCT90H2372XS32GC01KFY` after resuming from a pre-publication missing-token stop with the canonical SOPS-managed crates.io credential.

Both crates were published in dependency order (`reitti-core` before `reitti-cli`). Cargo-dist workflow run `35828847225` succeeded for macOS ARM64 on the required self-hosted runner and Linux musl ARM64/x86_64. GitHub Release assets, aggregate checksums, executable architecture and source identity, the three GitHub artifact attestations, the Homebrew formula, and a disposable public shell installation were verified. The disposable installation proved the original bare-invocation regression fixed in the actual published artifact.

Homebase run `01m36h0qae5jybmp8xpg2kya5z` advanced the managed fleet pin and all three release checksums to v1.1.0 with its full 336-test dotfiles suite green (one intentional skip). That change is clean and pushed. Haapa was converged through `homebase fleet apply --component package/reitti-cli`; `/home/jari/.local/bin/reitti` now reports v1.1.0 and release commit `7858af1`, its bundled skill reports v1.1.0, and a final bare invocation exits 0 with readable stdout and empty stderr. Temporary release-tool downloads were removed.

There is no prepared execution agenda: the reservation-aware issue DAG is empty, there are no unresolved product decisions or accepted spin-offs, and no test-account reset is configured. A future stint should begin from new user direction or intake rather than manufacturing work from this completed release.

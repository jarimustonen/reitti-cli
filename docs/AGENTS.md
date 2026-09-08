# Documentation guidance

Documentation in this directory is public, user-facing material unless it explicitly identifies itself as maintainer release operations.

- Verify every CLI example against the current `reitti` binary and its structured help.
- Never include real subscription keys, private machine names, local home paths, ignored build caches, or unpublished artifact claims.
- Keep API registration and rotation in `digitransit-api-access.md`, runtime precedence and safe setup in `configuration.md`, and release mechanics in `DISTRIBUTION.md`.
- Describe Digitransit and HSL attribution accurately and state that reitti-cli is independent.
- Treat fixed future timestamps as illustrative and tell users to supply their actual travel date and UTC offset.
- Preserve the distinction between direct preflight builds, final artifacts, GitHub Actions evidence, and published releases.

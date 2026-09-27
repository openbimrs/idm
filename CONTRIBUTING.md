# Contributing

Thank you for improving `openbimrs/idm`.

## Setup

Install Rust 1.85.0 (the pinned MSRV), Python 3.9+, `uv`, Node 22+, and npm. Build caches should live outside the repository when practical:

```bash
export CARGO_TARGET_DIR=/mnt/backup/build-cache/openbim-idm-target
uv sync --extra test
npm ci
```

## Boundaries

- Put all implementation, public Rust types, CLI behavior, and PyO3 code in `openbim-idm`.
- Keep `idmxml/src/lib.rs` as the single re-export statement. The alias dependency must remain `version = "=<workspace version>"` (updated in lockstep).
- Do not add Annex B XSD files, DIN/ISO PDFs, standards text, or redistributed examples. Local references belong in ignored `references/`.
- Do not publish anything outside `.github/workflows/release.yml`, and do not add PyPI/npm publishing without a separately reviewed change. Every artifact must pass `scripts/check-leakage.py`.
- Preserve unknown XML content and namespace spellings; this is a lossless model.

## Gates

```bash
./scripts/gate.sh
```

At minimum, changes must pass formatting, all-feature build/tests, Clippy with warnings denied, rustdoc warnings denied, alias purity and mutation checks, package leakage checks, Python tests, and the docs build. Add a changelog entry for user-visible behavior.

## Pull requests

Keep commits focused. Explain standards assumptions, include synthetic non-normative fixtures where formal schemas are needed, and report checks actually run. Never attach standards files to an issue or pull request.

## Releasing

Releases publish `openbim-idm` and then `idmxml` from CI through crates.io
trusted publishing (`.github/workflows/release.yml`); no one needs a crates.io
token. Only crates.io is published; the Python package is not on PyPI.

1. Bump `version` in the root `Cargo.toml` `[workspace.package]`, in both
   `openbim-idm/Cargo.toml` and `idmxml/Cargo.toml` (explicit, not inherited),
   the alias pin `openbim-idm = { ..., version = "=x.y.z" }`, and
   `pyproject.toml`; run `cargo update -w`. Date the `## [Unreleased]`
   changelog section as `## [x.y.z] - date` and update the link references.
2. Run `./scripts/gate.sh` (it packages both crates and runs
   `scripts/check-leakage.py` on each `.crate`; this check is mandatory), and
   `cargo semver-checks -p openbim-idm` against the last release to confirm the
   bump matches the change (pre-1.0: breaking change bumps the minor version).
3. Merge to `main`, then push an annotated tag `vx.y.z` on that commit.

The workflow refuses a tag that is not on `main`, does not match both crates'
version, or has no changelog section; it runs the full gate on the tagged
commit, publishes `openbim-idm`, then `idmxml` (the alias pins the exact
version, so order matters), and creates the GitHub release from the changelog.
Re-running a partly failed release skips crates already live. To rehearse, run
the workflow by hand with an existing tag: it gates and packages, and publishes
nothing. Publishing requires approval in the protected `crates.io` environment.

crates.io trusts this repository, the file name `release.yml` and the
`crates.io` environment (each crate's Settings -> Trusted Publishing); renaming
either needs the same change there.

## Licensing contributions

Unless an explicitly signed agreement says otherwise, every contribution
submitted to this repository is licensed under `AGPL-3.0-or-later`. Submit only
work that you have the right to license. Identify third-party material and
preserve its license, attribution, and provenance.

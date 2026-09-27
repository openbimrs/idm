# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases are intended to follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-09-27

First functional release of the standalone `openbimrs/idm` repository. It
replaces the name-reservation placeholder `0.1.0` that was published from the
former `openbimrs/openbim` monorepo.

### Changed

- **Breaking:** the crates now carry the complete implementation below; the placeholder `DocumentKind` enum (and `DocumentKind::ALL`) from `0.1.0` is removed, and the `openbim-core` dependency is gone.
- The alias `idmxml` pins `openbim-idm = "=0.2.0"` with `default-features = false`; the canonical crate's default `cli` feature is not enabled through the alias.
- Publication is enabled for the two Cargo packages through a tag-driven crates.io trusted-publishing workflow (`.github/workflows/release.yml`). The Python package is not yet published to PyPI.
- Relicensed repository-authored work from MIT to `AGPL-3.0-or-later`; historical releases remain under their published MIT terms, and third-party material retains its own terms.

### Added

- Standalone `openbim-idm` canonical crate with lossless XML, catalog-aware editing, validation, Rust CLI, and PyO3 implementation.
- Exact-version, implementation-free `idmxml` Rust alias crate.
- Python `idmxml` facade and `idmpy` CLI.
- Explicit-path, offline formal XSD validation without redistributed schemas.
- Safety limits for XML byte size and nesting depth; DTD-defined entity and `DOCTYPE` rejection.
- Generated declaration catalog with source hashes and no XSD bytes.
- CI, publication/leakage checks, alias mutation probe, and VitePress documentation.

### Security

- Excluded Annex B XSDs, PDFs, and standards material from source, package, wheel, and Pages artifacts.

### Fixed

- Preserve predefined and numeric XML character references during lossless round trips, and reject undefined or invalid references instead of silently dropping them.
- Scan every public source payload—not only filenames and the generated catalog—for renamed or embedded XSD/PDF standards content, with a mutation-verified leakage probe.
- Keep the complete repository gate operational in exported source trees that intentionally omit `.git` metadata.

## [0.1.0] - 2026-08-24

Name reservation published from the former `openbimrs/openbim` monorepo under
MIT: a `DocumentKind` placeholder enum and no parser. Superseded by `0.2.0`.

[Unreleased]: https://github.com/openbimrs/idm/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/idm/releases/tag/v0.2.0
[0.1.0]: https://crates.io/crates/openbim-idm/0.1.0

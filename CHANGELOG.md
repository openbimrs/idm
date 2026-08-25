# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases are intended to follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/openbimrs/idm/commits/main

# Changelog

The project follows Keep a Changelog. The first release from this repository is `0.2.0`.

## 0.2.0 - 2026-09-27

- Published `openbim-idm` and the `idmxml` alias to crates.io, replacing the `0.1.0` name-reservation placeholder (breaking: its `DocumentKind` enum is removed).

- Introduced the standalone canonical `openbim-idm` implementation and exact-version `idmxml` alias.
- Migrated the lossless Rust engine, Rust and Python CLIs, PyO3 facade, safety limits, generated catalog, tests, and non-normative fixture.
- Replaced embedded-schema behavior with explicit-path, offline XSD validation.
- Added architecture, security, provenance, publication, CI, artifact leakage, alias-purity, mutation, and documentation gates.

The canonical changelog is [CHANGELOG.md](https://github.com/openbimrs/idm/blob/main/CHANGELOG.md).

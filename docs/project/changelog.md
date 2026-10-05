# Changelog

The project follows Keep a Changelog. The first release from this repository is `0.2.0`.

## 0.3.0 - 2026-10-05

- Made the library usable as a foundation for downstream CLIs and GUIs: deeper catalog validation (datatypes, enumerations, element-only text, extension attributes, duplicate ids, references), per-node reads and schema rules, `guid:`/`id:` locators, insert/duplicate/paste/reparent, versioned reversible edits and diff, encoding-aware byte and file I/O, schema-hash verification, and stable error codes.
- Extended the `idmxml` and `idmpy` CLIs (in-place edits, find/list/node/rule, move/duplicate/insert, diff/apply, encodings, JSON errors).
- Namespaced, typed Python API with coded exceptions (old flat names deprecated) and a `wasm32` binding mirroring it.
- Fixed lossless round-trip defects (comment escaping, attribute whitespace, references in API-set text) and stack exhaustion on deep input; **breaking:** the depth limit is now 256.

## 0.2.0 - 2026-09-27

- Published `openbim-idm` and the `idmxml` alias to crates.io, replacing the `0.1.0` name-reservation placeholder (breaking: its `DocumentKind` enum is removed).

- Introduced the standalone canonical `openbim-idm` implementation and exact-version `idmxml` alias.
- Migrated the lossless Rust engine, Rust and Python CLIs, PyO3 facade, safety limits, generated catalog, tests, and non-normative fixture.
- Replaced embedded-schema behavior with explicit-path, offline XSD validation.
- Added architecture, security, provenance, publication, CI, artifact leakage, alias-purity, mutation, and documentation gates.

The canonical changelog is [CHANGELOG.md](https://github.com/openbimrs/idm/blob/main/CHANGELOG.md).

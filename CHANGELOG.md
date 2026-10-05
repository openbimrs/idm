# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases are intended to follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Validation of XSD built-in lexical types, element enumerations, element-only content, unknown attributes (warning), duplicate `id` values and `changedBy` references; `ValidationIssue` gains optional `attribute` and `expected` fields.
- `Document::node_info`, `schema_rule`, `attribute_slots`, `add_schema_attribute`, `remove_attribute`, `find_by_id`, and `guid:`/`id:` locators accepted by every path-taking method.
- `insert_schema_child`, `duplicate_schema_node`, `paste_schema_node`, `reparent_schema_node` with `guid`/`id` regeneration.
- Versioned, reversible `Edit`/`EditBatch` operations (`apply`, `apply_batch`) and `Document::diff`.
- Encoding-aware `parse_bytes`/`to_bytes_with_encoding` (UTF-8/16, ISO-8859-1, windows-1252), `from_path`/`from_reader`/`write_to`, and atomic `write_path`.
- `verify_schema_dir` to compare a local schema directory with the catalog's source hashes.
- Stable `Error::code()`; `Error` is now `Clone + PartialEq + Serialize` and `#[non_exhaustive]`, with new `Content`, `Io` and `Encoding` variants.
- CLI: `--in-place`, `--encoding`, `--json-errors`, and commands `find`, `list`, `node`, `rule`, `insert`, `duplicate`, `move`, `add-attribute`, `remove-attribute`, `diff`, `apply`, `schema --verify`.
- Python: namespaced modules (`idmxml.fileio`, `.schema`, `.validation`, `.errors`, `.models`), coded exception hierarchy, typed shapes, `py.typed`, `Document.copy/==`, node/edit/diff/bytes APIs, and an `idmpy` CLI with matching commands.
- `wasm` feature: a `wasm32-unknown-unknown` binding mirroring the Python facade.

### Changed

- **Breaking (Rust):** `Error` is `#[non_exhaustive]` and has new variants; `ValidationIssue` has new optional fields; `Document::set_text` rejects element-only elements (use `set_text_unchecked`).
- Generated `new_idm` skeletons point `changeLog/@changedBy` at the generated author's `id`.
- The embedded schema catalog is parsed once per process.
- **Deprecated (Python):** the flat names `load`, `loads`, `dump`, `dumps`, `schema_catalog`, `schema_text`, `xsd_validate`, `SCHEMA_FILES` and `DEFAULT_MAX_XML_BYTES` now warn; use the namespaced equivalents.

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

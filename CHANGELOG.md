# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases are intended to follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.0] - 2026-10-05

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

### Fixed

- Comments containing `"`, `'`, `<`, `>` or `&` were written with entity escapes and corrupted on every round trip.
- Tabs, newlines and carriage returns in attribute values, and carriage returns in text, were normalized away when read back; they are now written as character references.
- Text containing `&`, `<` or `]]>` set through the API (or supplied as tree JSON) did not equal its reloaded form, so `from_value`/`from_dict` rejected such documents. Text is now stored in the reader's canonical form.
- `to_xml` could emit unreadable XML (control characters, `]]>` in CDATA, `--` in comments, `?>` in processing instructions, invalid names); it now returns `Error::Write`, and `set_text`/`set_attribute` reject such characters up front.
- Tree JSON deeper than about 40 elements could not be read back (serde_json's recursion limit); `Document::from_json_str`, `Edit::from_json_str` and `EditBatch::from_json_str` accept the full depth range, and the CLI, Python and WASM bindings use them.
- Documents within the old depth limit could overflow small thread stacks (a process abort); see the depth limit change below.
- Element paths for documents whose root is not `idm` used `/idm` as their first segment.
- Undoing a `Move` out of a same-name sibling's subtree targeted the wrong parent.
- `xmlns:*` and `xml:*` attributes set through the API now carry their built-in namespaces, matching what the reader produces.
- Python `fileio.dump` writes atomically.

### Changed

- **Breaking:** `DEFAULT_MAX_XML_DEPTH` is now 256 (was 1024) and applies to XML, JSON and edits. Every operation at this depth fits a 512 KiB stack in release builds (WASM) and the default 2 MiB thread stack in debug builds.
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

[Unreleased]: https://github.com/openbimrs/idm/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/idm/releases/tag/v0.3.0
[0.2.0]: https://github.com/openbimrs/idm/releases/tag/v0.2.0
[0.1.0]: https://crates.io/crates/openbim-idm/0.1.0

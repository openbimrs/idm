# Rust API

The canonical crate is `openbim-idm`; its Rust import name is `openbim_idm`.

## Primary types

- `Document`: bounded parse, serialize, lossless JSON conversion, path access, catalog-aware edits, and validation.
- `Element`, `Attribute`, and `Node`: complete XML tree primitives.
- `SchemaCatalog` and declaration types: generated content-model metadata.
- `ValidationIssue` and `ValidationSeverity`: structured diagnostics.
- `Edit` and `EditBatch`: versioned, serializable, reversible edit operations.
- `NodeInfo`, `AttributeInfo`, `AttributeSlot`: per-element read models for UIs.
- `ParsedBytes` and `Encoding`: encoding-aware reading and writing.
- `Error`: parse, size, depth, path, schema, JSON, cardinality, content-model, I/O, and encoding failures. `Error::code()` returns a stable machine-readable code and `Error` is `Clone + PartialEq + Serialize`.

## Core operations

```rust
use openbim_idm::Document;

let mut document = Document::new_idm("Coordination", "IDM-001")?;
let actions = document.allowed_children("/idm/uc[0]")?;
let sub_uc = document.append_schema_child("/idm/uc[0]", "subUc")?;
document.set_text(&format!("{sub_uc}/uc[0]/description[0]"), "Nested use case")?;
let diagnostics = document.validate();
let xml = document.to_xml(true)?;
# Ok::<(), openbim_idm::Error>(())
```

Paths use indexed local names such as `/idm/uc[0]/subUc[1]`. Every path-taking method also accepts a `guid:<value>` or `id:<value>` locator; use `Document::path_of` to resolve one.

## Reading and I/O

- `Document::from_path`, `from_reader`, `parse_bytes` detect UTF-8/16 (BOM or declaration), ISO-8859-1 and windows-1252, enforce the size limit before and during reading, and report `ParsedBytes::warnings` about information that serialization changes.
- `write_path` writes atomically (temporary file in the same directory, then rename), so a failed write leaves the original file intact. `write_to`, `to_bytes` and `to_bytes_with_encoding` cover streams and other encodings; unrepresentable characters are an `Error::Encoding`.
- `node_info`, `attributes`, `attribute_slots` and `schema_rule` give a property panel everything for one element. `schema_rule` returns `Error::Schema` for undeclared (extension) content.

## Editing

- `set_text` rejects element-only elements with `Error::Content` (`set_text_unchecked` bypasses); `remove_attribute` rejects declared required attributes; `add_schema_attribute` adds a declared attribute with its default.
- `insert_schema_child`, `duplicate_schema_node`, `paste_schema_node` and `reparent_schema_node` are cardinality- and order-checked; copies get fresh `guid`/`id` values (and `changedBy` references inside the copy are remapped).
- `apply(&Edit)` returns the inverse edit, `apply_batch(&EditBatch)` is atomic and returns the inverse batch, and `diff(&other)` derives a best-effort batch. The JSON form is tagged by `op` and the batch carries `version` (currently `1`).

## Validation

`Document::validate` checks required attributes, XSD built-in lexical types (`xs:boolean`, `xs:integer`, `xs:dateTime`, `xs:NCName`, `xs:normalizedString`), the declared GUID pattern, enumerations, cardinality, order, element-only content, duplicate `guid`/`id` values, and `changeLog/@changedBy` references. Undeclared elements and attributes are warnings (`extension_element`, `extension_attribute`), not errors, because they are preserved. `ValidationIssue::attribute` and `::expected` locate the offending field. This is catalog-driven validation, not formal XSD validation.

`verify_schema_dir(dir)` hashes the six recognized files in an explicit directory and reports `match`, `mismatch` or `missing` against the catalog, so you can tell whether the catalog validator applies to your schema set.

### Validation issue codes

| Code | Severity | Meaning |
| --- | --- | --- |
| `invalid_root` | error | root is not the unqualified `idm` element |
| `schema_catalog` / `schema_definition` | error | the embedded catalog or a declaration could not be loaded |
| `required_attribute` | error | a required attribute is missing |
| `attribute_pattern` | error | the `guid` value does not match the declared pattern |
| `attribute_datatype` / `element_datatype` | error | value is not valid for its `xs:` built-in type (`expected.type`) |
| `attribute_enumeration` / `element_enumeration` | error | value is not in the declared enumeration (`expected.enum`) |
| `minimum_cardinality` / `maximum_cardinality` / `choice_cardinality` | error | child counts outside the declared bounds (`expected.min`/`max`) |
| `schema_order` | error | element is not in declaration order |
| `unexpected_text` | error | character data in element-only content |
| `duplicate_guid` / `duplicate_id` | error | `guid` / `id` values are not unique |
| `dangling_reference` | warning | `changeLog/@changedBy` matches no `author/@id` (`expected.oneOf`) |
| `extension_element` / `extension_attribute` | warning | undeclared content, preserved as an extension |
| overlay codes such as `idm.er.required_by_standard` | error | documented standard-over-XSD semantic overlays |

### Error codes

`Error::code()` returns one of `input_too_large`, `max_depth_exceeded`, `invalid_xml`, `write_failed`, `invalid_utf8`, `invalid_path`, `path_not_found`, `schema`, `invalid_json`, `cardinality`, `content_model`, `io`, `encoding`. The CLI (`--json-errors`), Python (`IdmError.code`) and JS (`Error.code`, `Errors.codes()`) expose the same strings.

## WebAssembly

`--features wasm` on `wasm32-unknown-unknown` exports a `Document` class that mirrors the Python facade (camelCase, JSON strings for structured values, errors thrown as `Error` objects with a `code`). The only top-level exports are `Document`, `LoadResult` and the namespace classes `Schema` (`Schema.catalogJson()`), `Engine` (`Engine.version()`) and `Errors` (`Errors.codes()`); there are no flat helper functions. Generate glue with `wasm-bindgen`; see `scripts/wasm-smoke.sh`. Nothing is published to npm.

## External schema access

`schema_text(schema_dir, name)` and `local_schema_inventory(schema_dir)` only read recognized filenames from the explicit directory. They never download or use embedded fallback data. `schema_catalog()` reads the non-normative generated metadata embedded in the crate.

For exact signatures and trait implementations, build the crate documentation:

```bash
cargo doc -p openbim-idm --all-features --no-deps
```

# Rust API

The canonical crate is `openbim-idm`; its Rust import name is `openbim_idm`.

## Primary types

- `Document`: bounded parse, serialize, lossless JSON conversion, path access, catalog-aware edits, and validation.
- `Element`, `Attribute`, and `Node`: complete XML tree primitives.
- `SchemaCatalog` and declaration types: generated content-model metadata.
- `ValidationIssue` and `ValidationSeverity`: structured diagnostics.
- `Error`: parse, size, depth, path, schema, JSON, and cardinality failures.

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

Paths use indexed local names such as `/idm/uc[0]/subUc[1]`.

## External schema access

`schema_text(schema_dir, name)` and `local_schema_inventory(schema_dir)` only read recognized filenames from the explicit directory. They never download or use embedded fallback data. `schema_catalog()` reads the non-normative generated metadata embedded in the crate.

For exact signatures and trait implementations, build the crate documentation:

```bash
cargo doc -p openbim-idm --all-features --no-deps
```

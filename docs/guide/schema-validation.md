# Schema validation and the standards boundary

## What the repository contains

`openbim-idm/catalog/catalog.json` is generated metadata: declaration names, content models, cardinalities, source coordinates, source filenames, semantic overlays, and SHA-256 hashes. A leakage gate verifies that it contains no XML declaration or XSD schema markup.

The repository does **not** contain the six Annex B XSD files, DIN/ISO PDFs, or copied standards examples. The names expected by the catalog are:

- `specId.xsd`
- `authoring.xsd`
- `uc.xsd`
- `businessContextMap.xsd`
- `er.xsd`
- `idm.xsd`

The filenames are identifiers, not redistributed content.

## Python formal validation

Acquire the schemas through a lawful channel and keep them outside tracked files. `references/` is ignored for this purpose.

```python
issues = idmxml.xsd_validate(
    document,
    schema_dir="references/schema/iso29481-3",
)
```

You may pass `schema_path=".../idm.xsd"` instead, or set `IDMXML_SCHEMA_DIR`. Explicit arguments take precedence; passing both is an error.

```bash
idmpy validate model.idmxml --xsd --schema-dir references/schema/iso29481-3
```

Formal XSD validation requires the optional `lxml` dependency. The parser disables network access, DTD loading, and entity resolution. Includes are allowlisted to the six known filenames and resolved from one directory.

## Structural validation

Rust and Python always expose catalog-driven structural and semantic validation without reading external XSDs. It covers ordering, required attributes, choice and cardinality rules, recursive declarations, UUID constraints, duplicate GUIDs, and documented semantic overlays. This is not represented as formal XSD validation.

## Regenerating the catalog

```bash
python scripts/generate_schema_catalog.py \
  --schema-dir references/schema/iso29481-3 \
  --output openbim-idm/catalog/catalog.json
```

Review every hash and declaration change. Never commit the input directory.

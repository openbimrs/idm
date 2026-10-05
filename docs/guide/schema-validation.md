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
issues = idmxml.validation.xsd_validate(
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

Rust and Python always expose catalog-driven structural and semantic validation without reading external XSDs. It covers ordering, required attributes, choice and cardinality rules, recursive declarations, the GUID pattern, XSD built-in lexical types (`xs:boolean`, `xs:integer`, `xs:dateTime`, `xs:NCName`, `xs:normalizedString`), enumerations, element-only content, duplicate `guid`/`id` values, `changedBy` references, and documented semantic overlays. Undeclared elements and attributes are reported as warnings because they are preserved as extension content. This is not represented as formal XSD validation.

## Does my schema set match the catalog?

The catalog records the SHA-256 of each source schema. Check a local directory without exposing or copying its content:

```bash
idmxml schema --verify references/schema/iso29481-3   # exit code 2 on mismatch or missing files
```

```python
report = idmxml.schema.verify("references/schema/iso29481-3")
assert report["ok"]
```

## Regenerating the catalog

```bash
python scripts/generate_schema_catalog.py \
  --schema-dir references/schema/iso29481-3 \
  --output openbim-idm/catalog/catalog.json
```

Review every hash and declaration change. Never commit the input directory.

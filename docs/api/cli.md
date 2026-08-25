# Command-line API

## Rust: `idmxml`

The canonical crate builds the `idmxml` binary with its default `cli` feature.

```text
inspect       summarize a recursive document and diagnostics
validate      run catalog-driven structural and semantic validation
format        serialize the complete XML tree
new           create a semantically complete IDM skeleton
to-json       export the lossless tree
from-json     rebuild XML from lossless tree JSON
get           read indexed-path element text
set           update indexed-path element text
set-attribute update an attribute
allowed       list catalog-permitted child actions
add           create a cardinality-checked child skeleton
remove        remove a content-model-permitted node
schema        print the generated declaration catalog
```

Use `idmxml <command> --help` for arguments. The Rust `validate` command does not claim formal XSD validation.

## Python: `idmpy`

```bash
idmpy inspect model.idmxml --json
idmpy validate model.idmxml
idmpy validate model.idmxml --xsd --schema-dir references/schema/iso29481-3
idmpy new "Coordination" IDM-001 --output model.idmxml
idmpy schema --json
```

`--xsd` requires either `--schema-dir`, `--schema`, or `IDMXML_SCHEMA_DIR`; no embedded or network fallback exists. Validation returns exit code `2` when an error diagnostic is present and `1` for operational failures.

# Command-line API

## Rust: `idmxml`

The canonical crate builds the `idmxml` binary with its default `cli` feature.

```text
inspect          summarize a recursive document and diagnostics
validate         run catalog-driven structural and semantic validation
format           serialize the complete XML tree
new              create a semantically complete IDM skeleton
to-json          export the lossless tree
from-json        rebuild XML from lossless tree JSON
get              read element text
set              update element text
set-attribute    update an attribute
add-attribute    add a schema-declared attribute with its default
remove-attribute remove an attribute (required ones are rejected)
allowed          list catalog-permitted child actions
add              create a cardinality-checked child skeleton
insert           add a child at a sibling position
remove           remove a content-model-permitted node
duplicate        duplicate a subtree with fresh guid/id values
move             move a subtree below another allowed parent / position
find             find paths by --name, --guid or --id
list             list all element paths (or those with a given name)
node             show attributes, text, children and schema handle
rule             show the catalog rule that applies at a path
diff             print the JSON edit batch between two documents
apply            apply a JSON edit batch atomically (--undo writes the inverse)
schema           print the catalog; --verify DIR checks local XSD hashes
```

Editing commands write to stdout by default, to `-o FILE`, or atomically over the input with `-i/--in-place`. `--encoding` selects the output encoding (`utf-8`, `utf-16le`, `utf-16be`, `iso-8859-1`, `windows-1252`); non-UTF-8 input is read transparently and a `note:` is printed to stderr about what serialization changes. Paths accept indexed paths or `guid:`/`id:` locators. `--json-errors` prints failures as `{"code", "message"}` on stderr.

Exit codes: `0` success; `1` operational error (or `find` without matches); `2` validation errors or a schema hash mismatch.

Use `idmxml <command> --help` for arguments. The Rust `validate` command does not claim formal XSD validation.

## Python: `idmpy`

```bash
idmpy inspect model.idmxml --json
idmpy validate model.idmxml
idmpy validate model.idmxml --xsd --schema-dir references/schema/iso29481-3
idmpy new "Coordination" IDM-001 --output model.idmxml
idmpy schema --json
idmpy schema --verify references/schema/iso29481-3
```

`idmpy` offers the same edit, query, `diff`/`apply` and `--in-place`/`--encoding`/`--json-errors` options as `idmxml`.

`--xsd` requires either `--schema-dir`, `--schema`, or `IDMXML_SCHEMA_DIR`; no embedded or network fallback exists. Validation returns exit code `2` when an error diagnostic is present and `1` for operational failures.

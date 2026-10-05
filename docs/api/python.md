# Python API

The Python package is `idmxml`; the native implementation is loaded from `idmxml._native`. The public API is organised in namespaces:

| Namespace | Contents |
| --- | --- |
| `idmxml.Document` | the lossless, schema-aware document |
| `idmxml.fileio` | `load`, `loads`, `dump`, `dumps`, `load_bytes`, `load_with_info` |
| `idmxml.schema` | `FILES`, `catalog`, `read_text`, `verify`, `resolve_dir` |
| `idmxml.validation` | `xsd_validate` (optional, needs `idmxml[xsd]`) |
| `idmxml.errors` | the exception hierarchy |
| `idmxml.models` | `TypedDict` shapes of returned values |

```python
import idmxml
from idmxml import fileio

model = idmxml.Document.new("Coordination", "IDM-001")
model.set_attribute("/idm/specId[0]", "fullTitle", "Updated")
fileio.dump(model, "model.idmxml")
round_tripped = idmxml.Document.from_dict(model.to_dict())
```

The package ships `py.typed` and `_native.pyi`; `mypy --strict` is part of the repository gate.

::: warning Deprecated flat names
`idmxml.load`, `loads`, `dump`, `dumps`, `schema_catalog`, `schema_text`, `xsd_validate`, `SCHEMA_FILES` and `DEFAULT_MAX_XML_BYTES` still resolve, with a `DeprecationWarning` naming the replacement, and will be removed in a later release.
:::

## Reading

- `node(path)` returns attributes, text, child paths and the catalog handle of one element; `attributes`, `attribute_slots` (declared attributes with presence/value) and `schema_rule(path)` expose the schema context a property panel needs. `schema_rule` raises `SchemaError` for undeclared (extension) content.
- `element(path)` returns the complete lossless subtree; `to_dict()` the whole document.
- `find_by_guid`, `find_by_id`, `element_paths`, `count` and `path_of(locator)` locate elements.
- `validate()` returns `Issue` dictionaries: `severity`, `code`, `path`, `message`, plus `attribute` and `expected` when relevant.

## Locators

Wherever a `path` is accepted you may pass `guid:<value>` or `id:<value>` instead. Locators keep working across inserts and removals, whereas indexed paths such as `/idm/uc[0]/subUc[1]` shift. A locator must match exactly one element.

## Editing

- Content: `set_text` (rejects element-only elements; `set_text_unchecked` bypasses), `set_attribute`, `remove_attribute` (rejects declared required attributes), `add_schema_attribute`.
- Structure: `append_schema_child`, `insert_schema_child(parent, name, position)`, `remove_schema_node`, `duplicate_schema_node` (fresh `guid`/`id`), `move_schema_node` (same-name siblings), `reparent_schema_node(path, new_parent, position=)` and `paste_schema_node(parent, element, position=)`. All are cardinality- and order-checked; a failed `reparent_schema_node` leaves the document untouched.
- `copy()`, `==` and `copy.deepcopy` are supported; documents are unhashable.

## Reversible edits and diff

`apply(edit)` applies one edit and returns its inverse; `apply_batch(batch)` is atomic and returns the inverse batch. Edits are tagged dictionaries (`{"op": "set_attribute", "path": ..., "name": ..., "value": ...}`); batches carry `"version": 1`. The operations are `set_text`, `set_children`, `set_attribute`, `remove_attribute`, `insert_child`, `remove_child` and `move`. `diff(other)` returns a best-effort batch that turns one document into another. Real-time collaboration and concurrent-edit merging are out of scope.

## Bytes, encodings and files

`Document.from_bytes`, `to_bytes(encoding=...)`, `fileio.load_bytes` and `fileio.load_with_info` understand UTF-8, UTF-16 (with or without BOM), ISO-8859-1 and windows-1252. `LoadResult.warnings` states what serialization changes (for example the dropped BOM, or non-UTF-8 input written as UTF-8). Unrepresentable characters raise `EncodingError`; nothing is silently replaced.

## Errors

Every error derives from `idmxml.errors.IdmError` (itself a `ValueError`) and carries a stable `.code` such as `path_not_found`, `cardinality`, `content_model`, `invalid_xml`, `invalid_json`, `schema`, `encoding` or `io`. Subclasses: `XmlError`, `PathError`, `SchemaError`, `JsonError`, `CardinalityError`, `ContentModelError`, `EncodingError`, `IdmIoError`.

Python 3.9+ is supported through PyO3 `abi3-py39`. The package is not yet published to PyPI; build it locally with `maturin`.

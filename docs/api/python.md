# Python API

The Python package is `idmxml`; native implementation is loaded from `idmxml._native`.

```python
import idmxml

model = idmxml.Document.new("Coordination", "IDM-001")
model.set_attribute("/idm/specId[0]", "fullTitle", "Updated")
xml = model.to_xml(pretty=True)
data = model.to_dict()
round_tripped = idmxml.Document.from_dict(data)
```

## Module functions

- `load`, `loads`, `dump`, `dumps`: file/string convenience functions.
- `schema_catalog`: return generated declaration metadata.
- `schema_text`: read one recognized schema from an explicit local directory.
- `xsd_validate`: optional formal validation with explicit `schema_dir` or `schema_path`.

## Document methods

The facade delegates parsing, serialization, catalog-aware edit operations, and structural validation to Rust. `Document.validate()` returns dictionaries with severity, code, path, and message. `allowed_children`, `append_schema_child`, `remove_schema_node`, and `move_schema_node` preserve content-model constraints.

Python 3.9+ is supported through PyO3 `abi3-py39`. The package is pre-release and publication-blocked.

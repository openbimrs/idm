# Getting started

## Choose the public surface

| Surface | Stable name | Owner |
|---|---|---|
| Canonical Cargo package | `openbim-idm` | All Rust implementation, types, CLI, and PyO3 behavior |
| Rust compatibility alias | `idmxml` | A single exact-version re-export statement |
| Rust binary | `idmxml` | Built by `openbim-idm` with the `cli` feature |
| Python package | `idmxml` | Thin Python ergonomics over `idmxml._native` |
| Python CLI | `idmpy` | Python command facade |

Packages are not published. For development, clone the repository and use workspace paths.

## Rust

```bash
cargo test --workspace --all-features
cargo run -p openbim-idm --bin idmxml -- \
  inspect openbim-idm/tests/fixtures/recursive-extension.xml --json
```

```rust
use openbim_idm::Document;

let document = Document::parse("<idm/>")?;
assert_eq!(document.root().local_name(), "idm");
# Ok::<(), openbim_idm::Error>(())
```

The parser's default maximum input is 64 MiB and maximum nesting is 1,024 levels. Applications can lower the byte limit with `Document::with_max_xml_bytes`.

## Python

```bash
uv sync --extra test
uv run maturin develop --features python
uv run pytest
```

```python
from pathlib import Path
import idmxml

document = idmxml.load(Path("model.idmxml"))
print(document.count("er"))
print(document.validate())
```

## What “lossless” means

The data model is the complete XML tree, not a narrowed business DTO. A parse/serialize cycle retains qualified names, namespace URIs and prefixes, unknown/vendor content, child and attribute ordering, comments, CDATA, and processing instructions. Pretty formatting may change whitespace; compact serialization and the tree JSON form make structural equality testable.

Next: [formal schema validation without bundled schemas](./schema-validation).

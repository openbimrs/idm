# openbimrs/idm

[![CI](https://github.com/openbimrs/idm/actions/workflows/ci.yml/badge.svg)](https://github.com/openbimrs/idm/actions/workflows/ci.yml)
[![Docs](https://github.com/openbimrs/idm/actions/workflows/pages.yml/badge.svg)](https://openbimrs.github.io/idm/)
[![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-blue)](https://github.com/openbimrs/idm/blob/main/rust-toolchain.toml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

Lossless, recursive ISO 29481-3 idmXML tooling in Rust, with Rust and Python CLIs and a thin PyO3 Python facade.

**Documentation:** [start here](https://openbimrs.github.io/idm/) · [guide](https://openbimrs.github.io/idm/guide/getting-started) · [API](https://openbimrs.github.io/idm/api/rust) · [architecture](https://openbimrs.github.io/idm/architecture/) · [security](https://openbimrs.github.io/idm/security) · [provenance](https://openbimrs.github.io/idm/provenance) · [changelog](https://openbimrs.github.io/idm/project/changelog)

> **Pre-release and non-publishable:** all package versions are `0.1.0`, but both Cargo packages set `publish = false` and Python publication is blocked by repository policy/gates while rights for redistributing the Annex B XSDs remain unresolved.

## What is included

- `openbim-idm`: the canonical crate; it owns the complete Rust tree model, catalog-driven editing/validation, `idmxml` binary, and PyO3 module.
- `idmxml`: an exact-version Rust re-export alias with no implementation or independent types.
- Python import package `idmxml`, native module `idmxml._native`, and `idmpy` CLI.
- A generated declaration catalog at `openbim-idm/catalog/catalog.json` containing names, content models, source coordinates, and SHA-256 provenance hashes—but **no normative XSD bytes**.

The six Annex B XSD files and all DIN/ISO PDFs are deliberately absent. Put lawfully obtained local material under ignored `references/` or elsewhere; formal XSD validation requires an explicit path.

## Capability matrix

| Capability | Rust library / `idmxml` CLI | Python / `idmpy` | Notes |
|---|---:|---:|---|
| Lossless XML tree round trips | Yes | Yes | Preserves namespaces, unknown elements/attributes, ordering, comments, CDATA, and processing instructions |
| Recursive IDM structures | Yes | Yes | Catalog-aware create, move, remove, and cardinality checks |
| Structural/semantic validation | Yes | Yes | Generated content model plus documented ISO-over-XSD overlays |
| Formal XSD validation | No | Yes, optional | Requires `lxml` and an explicit six-file schema directory/root path; offline and entity-safe |
| XML ↔ lossless JSON | Yes | Yes | Complete tree representation, not a reduced domain DTO |
| Embedded schemas or standards text | **No** | **No** | Intentionally excluded pending rights determination |
| ISO 29481-2/BPMN process maps | No | No | Out of scope |
| Digital signatures / schema repair | No | No | Out of scope |

## Rust

```toml
[dependencies]
openbim-idm = { path = "openbim-idm" } # registry publication is blocked today
```

```rust
use openbim_idm::Document;

let mut document = Document::new_idm("Coordination IDM", "IDM-001")?;
let child = document.append_schema_child("/idm/uc[0]", "subUc")?;
document.set_attribute(
    &format!("{child}/uc[0]/specId[0]"),
    "fullTitle",
    "Interface coordination",
)?;
let xml = document.to_xml(true)?;
# Ok::<(), openbim_idm::Error>(())
```

The canonical package intentionally exposes the familiar binary name:

```bash
cargo run -p openbim-idm --bin idmxml -- new "Coordination IDM" IDM-001 -o coordination.xml
cargo run -p openbim-idm --bin idmxml -- inspect coordination.xml --json
cargo run -p openbim-idm --bin idmxml -- validate coordination.xml
```

## Python

```bash
uv sync --extra test
uv run maturin develop --features python
uv run idmpy inspect openbim-idm/tests/fixtures/recursive-extension.xml --json
```

```python
import idmxml

idm = idmxml.Document.new("Coordination IDM", "IDM-001")
idm.append_schema_child("/idm/uc[0]", "subUc")
assert not [issue for issue in idm.validate() if issue["severity"] == "error"]
```

Formal validation never searches the network or silently uses bundled material:

```python
issues = idmxml.xsd_validate(idm, schema_dir="references/schema/iso29481-3")
# Or set IDMXML_SCHEMA_DIR, or pass schema_path=".../idm.xsd".
```

```bash
uv run idmpy validate coordination.xml --xsd --schema-dir references/schema/iso29481-3
```

## Development

```bash
export CARGO_TARGET_DIR=/mnt/backup/build-cache/openbim-idm-target
./scripts/gate.sh
```

See [CONTRIBUTING.md](CONTRIBUTING.md), [PUBLISHING.md](PUBLISHING.md), [SECURITY.md](SECURITY.md), and the [provenance policy](https://openbimrs.github.io/idm/provenance).

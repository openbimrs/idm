# Publishing and provenance

## Status

| Channel | Package | State |
|---|---|---|
| crates.io | [`openbim-idm`](https://crates.io/crates/openbim-idm) (canonical) | Published from this repository, starting with `0.2.0` |
| crates.io | [`idmxml`](https://crates.io/crates/idmxml) (alias) | Published from this repository, starting with `0.2.0` |
| PyPI | `idmxml` (Python facade, `idmpy` CLI) | **Not yet published.** Build locally with `maturin` |
| npm | — | Nothing is published; `package.json` only builds the documentation site |

`0.1.0` of both crates was a name-reservation placeholder published under MIT
from the former `openbimrs/openbim` monorepo. `0.2.0` is the first release from
this repository.

The repository owner lifted the former publication guards (`publish = false` in
both Cargo packages and `tool.openbim-idm.publish = false` in `pyproject.toml`)
in a dedicated change, treating the redistribution question for the published
artifacts as settled: the artifacts contain only repository-authored code and
generated metadata, never the ISO schemas themselves.

## What the artifacts contain

- `openbim-idm` crate: Rust sources (library, `idmxml` CLI, optional PyO3
  module), integration tests with one synthetic non-normative fixture, the
  generated `catalog/catalog.json`, `README.md`, `LICENSE`, and
  `LICENSES/MIT.txt`. The file list is fixed by `include` in
  `openbim-idm/Cargo.toml`.
- `idmxml` crate: exactly `src/lib.rs` (`pub use openbim_idm::*;`),
  `README.md`, `LICENSE`, and `LICENSES/MIT.txt`, depending on
  `openbim-idm = "=<same version>"`.

`catalog.json` holds declaration names, content models, source coordinates, and
SHA-256 hashes of the six source files. It contains no normative XSD bytes.

## What they never contain

The six ISO 29481-3 Annex B XSD files, DIN/ISO PDFs, copied standards text, and
normative examples are not in the repository and never enter any artifact:
crates, Python sdists/wheels, or the Pages site. Formal XSD validation takes an
explicit path to a lawfully obtained local schema set.

`scripts/check-leakage.py` is **mandatory** for every artifact. It rejects
`.xsd`/`.pdf` names, `references/`/`schemas/` path components, and embedded XML
Schema or PDF payloads regardless of filename. `scripts/gate.sh` runs it on the
tracked source tree, both packaged `.crate` files, the built Python artifacts,
and the built docs; the release workflow runs the same gate on the tagged
commit before anything is published. Never publish an artifact that has not
passed it.

## Releasing to crates.io

Releases publish from CI through crates.io trusted publishing (OIDC) in
`.github/workflows/release.yml`; see [CONTRIBUTING.md](CONTRIBUTING.md#releasing)
for the procedure. The publish job runs in the protected `crates.io` GitHub
environment and publishes `openbim-idm` first, then `idmxml`, because the alias
pins the canonical crate's exact version.

## Provenance

The implementation was extracted from a Poing-independent source package and reconciled with the `openbim-idm`/`idmxml` contracts prepared in the OpenBIM workspace. Neither source repository is a runtime or build dependency. See [the detailed provenance record](docs/provenance.md).

The committed catalog identifies the six source filenames and hashes. Regenerate it only from a lawfully obtained local set:

```bash
python scripts/generate_schema_catalog.py \
  --schema-dir references/schema/iso29481-3 \
  --output openbim-idm/catalog/catalog.json
```

`references/` is ignored. Never stage its contents.

## Before publishing to PyPI

PyPI publication is out of scope for now. Adding it requires a separately
reviewed workflow with its own protected environment and trusted publisher, a
leakage check of the exact sdist and wheels it uploads, and clean-install tests
on Python 3.9 and a current Python.

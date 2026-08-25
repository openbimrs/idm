# openbim-idm

Canonical Rust implementation for lossless ISO 29481-3 idmXML parsing, writing, catalog-aware editing, validation, the `idmxml` CLI, and the `idmxml._native` PyO3 module.

This crate is version `0.1.0` and intentionally sets `publish = false`. The six Annex B XSD files are not redistributed. The embedded generated catalog contains declaration metadata and source hashes, not XSD bytes. APIs that read source schemas require an explicit local directory.

See the [repository README](https://github.com/openbimrs/idm#readme) and [documentation](https://openbimrs.github.io/idm/).

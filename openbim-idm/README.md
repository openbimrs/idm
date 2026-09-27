# openbim-idm

Canonical Rust implementation for lossless ISO 29481-3 idmXML parsing, writing, catalog-aware editing, validation, the `idmxml` CLI, and the `idmxml._native` PyO3 module.

The six ISO 29481-3 Annex B XSD files are not redistributed in this crate. The embedded generated catalog contains declaration metadata and source hashes, not XSD bytes. APIs that read source schemas require an explicit local directory.

```toml
[dependencies]
openbim-idm = "0.2"
```

The `idmxml` binary is behind the default `cli` feature; use `default-features = false` for the library alone.

Licensed under `AGPL-3.0-or-later`. Version `0.1.0` was an unrelated MIT-licensed name-reservation placeholder.

See the [repository README](https://github.com/openbimrs/idm#readme) and [documentation](https://openbimrs.github.io/idm/).

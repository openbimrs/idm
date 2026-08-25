# Canonical and alias crates

`openbim-idm` is canonical even though practitioners commonly say “idmXML.” This leaves room for a clear project namespace while preserving a convenient compatibility dependency.

The alias crate's only Rust statement is:

```rust
pub use openbim_idm::*;
```

Its dependency is pinned with an exact `=0.1.0` version and a workspace path. Feature names only forward to the canonical crate. Therefore a graph containing both package names resolves one `openbim-idm` instance and one set of Rust types.

## Enforcement

`scripts/check-alias-purity.sh` inspects the alias manifest and source. `scripts/mutation-probe.sh` first proves the purity gate passes, injects an independent type into a temporary copy, proves the gate fails, then proves the clean copy still passes and the working source hash did not change.

Adding implementation, a build script, a binary, generated source, or an inexact canonical dependency to the alias is an architecture defect.

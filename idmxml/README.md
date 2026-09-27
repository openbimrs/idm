# `idmxml` alias crate

This package is a compatibility name for [`openbim-idm`](https://crates.io/crates/openbim-idm).
It contains no implementation or types: `src/lib.rs` is exactly `pub use openbim_idm::*;`,
and its dependency is pinned to the exact same version. New Rust consumers should depend on
`openbim-idm`.

Licensed under `AGPL-3.0-or-later`. No ISO schema files are included.

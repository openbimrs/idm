# Alias crate instructions

Scope: `idmxml`.

This is a compatibility alias only. `src/lib.rs` must contain exactly:

```rust
pub use openbim_idm::*;
```

Do not add types, traits, modules, functions, constants, macros, tests with implementation logic, binaries, build scripts, or PyO3 code. Keep the dependency path and exact `=VERSION` pin synchronized with the canonical crate. Run `scripts/check-alias-purity.sh` and its mutation probe after any manifest change.

# Architecture

## Layers

1. **Lossless tree core** — `Document`, `Element`, `Attribute`, and `Node` preserve XML information rather than projecting a reduced DTO.
2. **Generated declaration model** — committed catalog metadata drives skeleton creation, ordering, cardinality, choices, labels, and structural diagnostics.
3. **Canonical interfaces** — the `openbim-idm` crate owns the Rust API, `idmxml` Rust binary, and PyO3 `_native` module.
4. **Thin Python ergonomics** — `python/idmxml` adds path I/O, dictionaries, optional `lxml` validation, and `idmpy` argument parsing.
5. **Compatibility alias** — the `idmxml` Cargo package re-exports canonical symbols at an exact version and has no behavior.

## Trust boundaries

XML bytes and externally supplied schemas are untrusted input. Rust parsing applies input/depth limits and rejects `DOCTYPE`; Python formal validation disables entities, DTDs, and network access. The schema directory is an explicit consumer-controlled trust input, constrained to a six-filename include graph.

## Standards boundary

The source schemas are generation/validation inputs, not distributable runtime assets. Runtime structural behavior is based on the generated metadata snapshot and documented overlays. Formal XSD conformance is a separate optional operation requiring user-supplied files.

## Invariants

- One canonical Rust type universe.
- Unknown XML data survives edits.
- No standards payload in source or generated web/package artifacts.
- No silent network fallback.
- Publication stays blocked until provenance and rights gates change together.

See [Canonical and alias crates](./canonical-alias) for the enforced dependency boundary.

# Canonical crate instructions

Scope: `openbim-idm`.

- This crate owns every Rust implementation and public type, the `idmxml` binary, and PyO3 behavior.
- Preserve complete XML tree round trips, unknown content, namespaces, and safety limits.
- Do not embed or fixture normative XSD bytes. Schema-reading APIs require explicit local paths.
- Keep catalog-driven behavior deterministic and test recursive/cardinality cases.
- Feature combinations must compile: default/CLI, no-default core, and all-features/Python.

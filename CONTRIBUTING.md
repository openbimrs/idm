# Contributing

Thank you for improving `openbimrs/idm`.

## Setup

Install Rust 1.85.0 (the pinned MSRV), Python 3.9+, `uv`, Node 22+, and npm. Build caches should live outside the repository when practical:

```bash
export CARGO_TARGET_DIR=/mnt/backup/build-cache/openbim-idm-target
uv sync --extra test
npm ci
```

## Boundaries

- Put all implementation, public Rust types, CLI behavior, and PyO3 code in `openbim-idm`.
- Keep `idmxml/src/lib.rs` as the single re-export statement. The alias dependency must remain `version = "=0.1.0"` (updated in lockstep for later versions).
- Do not add Annex B XSD files, DIN/ISO PDFs, standards text, or redistributed examples. Local references belong in ignored `references/`.
- Do not weaken `publish = false` or the Python publication guard without a recorded rights determination.
- Preserve unknown XML content and namespace spellings; this is a lossless model.

## Gates

```bash
./scripts/gate.sh
```

At minimum, changes must pass formatting, all-feature build/tests, Clippy with warnings denied, rustdoc warnings denied, alias purity and mutation checks, package leakage checks, Python tests, and the docs build. Add a changelog entry for user-visible behavior.

## Pull requests

Keep commits focused. Explain standards assumptions, include synthetic non-normative fixtures where formal schemas are needed, and report checks actually run. Never attach standards files to an issue or pull request.

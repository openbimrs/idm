# HERMES.md

Repository-wide context for coding agents:

- Canonical implementation: `openbim-idm`.
- Compatibility alias: `idmxml`; it must remain an exact-version pure re-export with no types or logic.
- Python package/import remains `idmxml`; PyO3 module remains `idmxml._native`; Python CLI remains `idmpy`; Rust binary remains `idmxml`.
- Publication stays blocked while Annex B redistribution rights are unresolved.
- Never copy XSDs, DIN/ISO PDFs, standards text, or normative examples into tracked paths. Local inputs go under ignored `references/`.
- `openbim-idm/catalog/catalog.json` is generated metadata only. Ensure it never contains XML/XSD bytes.
- Formal validation takes explicit local schema paths and must stay offline/no-network/entity-safe.
- Run `./scripts/gate.sh` before completion and report actual results.

Read the nearest `AGENTS.md` before changing a subtree.

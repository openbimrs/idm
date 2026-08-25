# Publishing and provenance

Publication is intentionally blocked. Both Cargo packages use `publish = false`; `pyproject.toml` records `tool.openbim-idm.publish = false`, and repository release/publish scripts refuse while that value is false. There is no publication workflow.

## Why

The implementation is MIT-licensed, but redistribution rights for the six ISO 29481-3 Annex B XSD files have not been established. They are not part of this repository or any intended artifact. The generated `openbim-idm/catalog/catalog.json` contains declarations, source coordinates, and SHA-256 values only—no normative XSD bytes.

## Provenance

The implementation was extracted from a Poing-independent source package and reconciled with the `openbim-idm`/`idmxml` contracts prepared in the OpenBIM workspace. Neither source repository is a runtime or build dependency. See [the detailed provenance record](docs/provenance.md).

The committed catalog identifies the six source filenames and hashes. Regenerate it only from a lawfully obtained local set:

```bash
python scripts/generate_schema_catalog.py \
  --schema-dir references/schema/iso29481-3 \
  --output openbim-idm/catalog/catalog.json
```

`references/` is ignored. Never stage its contents.

## Publication prerequisites

1. Record a written rights determination for every source schema and any example proposed for redistribution.
2. Confirm names, trademarks, authorship, and source provenance.
3. Review catalog generation and apparent schema inconsistencies.
4. Deliberately remove all three publication guards in a dedicated, reviewed change.
5. Inspect Cargo packages, Python sdist/wheels, and Pages output using `scripts/check-leakage.py`.
6. Test clean installs on Python 3.9 and a current Python, and verify the MSRV.
7. Add a separately reviewed release workflow with protected environments and trusted publishing.

Until those steps are complete, local builds are development artifacts—not releases.

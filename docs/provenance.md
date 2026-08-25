# Provenance and redistribution

## Code

The initial implementation was migrated from the Poing-independent `packages/idmxml` source package and reconciled with standalone crate-name and alias contracts from the OpenBIM workspace. The resulting repository has no source, build, or runtime dependency on either workspace. Implementation and repository-authored documentation are licensed under MIT.

## Generated catalog

`openbim-idm/catalog/catalog.json` records the expected six source filenames, SHA-256 hashes, declaration names, source coordinates, content models, recursive edges, and semantic overlays. It was generated from a local schema set. Verification confirms it contains no `<?xml`, `<xs:schema`, `<xsd:schema`, or normative XSD byte payload.

Hashes establish byte identity; they do not grant redistribution rights.

## Excluded material

No Annex B XSD, DIN/ISO PDF, copied standard text, or normative example is committed. Local materials belong under ignored `references/` and cannot enter package or Pages artifacts. Formal validation consumers supply their own lawfully obtained local files.

## Publication status

Rights remain unresolved, so both Cargo packages set `publish = false` and Python publication is policy/gate-blocked. See [PUBLISHING.md](https://github.com/openbimrs/idm/blob/main/PUBLISHING.md) for prerequisites.

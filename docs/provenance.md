# Provenance and redistribution

## Code

The initial implementation was migrated from Poing commit `96fa4d37090af50588032eb666492a02d6fd6727`, path `extensions/apps/aia/packages/idmxml`. Git history for that source path identifies Friedrich Schrödter as its sole author. The extracted implementation and repository-authored documentation are published here under the repository's AGPL-3.0-or-later license; that grant does not cover ISO/DIN schemas, standards, or normative examples. The resulting repository has no source, build, or runtime dependency on Poing or the OpenBIM superproject.

## Generated catalog

`openbim-idm/catalog/catalog.json` records the expected six source filenames, SHA-256 hashes, declaration names, source coordinates, content models, recursive edges, and semantic overlays. It was generated from a local schema set. Verification confirms it contains no XML declaration, XML Schema root element, or normative XSD byte payload.

Hashes establish byte identity; they do not grant redistribution rights.

## Excluded material

No Annex B XSD, DIN/ISO PDF, copied standard text, or normative example is committed. Local materials belong under ignored `references/` and cannot enter package or Pages artifacts. Formal validation consumers supply their own lawfully obtained local files.

## Publication status

Rights remain unresolved, so both Cargo packages set `publish = false` and Python publication is policy/gate-blocked. See [PUBLISHING.md](https://github.com/openbimrs/idm/blob/main/PUBLISHING.md) for prerequisites.

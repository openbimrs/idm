---
layout: home

hero:
  name: openbim-idm
  text: Lossless idmXML, with an honest standards boundary
  tagline: A canonical Rust engine, Rust and Python CLIs, and a thin PyO3 facade for recursive ISO 29481-3 documents.
  image:
    src: /logo.svg
    alt: openbim-idm mark
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: Architecture
      link: /architecture/
    - theme: alt
      text: View on GitHub
      link: https://github.com/openbimrs/idm

features:
  - title: Lossless by design
    details: Preserve namespaces, extensions, attributes, order, comments, CDATA, processing instructions, and recursive structures.
  - title: One Rust type universe
    details: openbim-idm owns all behavior. idmxml is an exact-version re-export, so aliases never create competing types.
  - title: No hidden standards payload
    details: The declaration catalog contains metadata and hashes—not XSD bytes. Formal validation uses your explicit local schema path.
  - title: Defensive XML handling
    details: Input and depth limits, DOCTYPE rejection, offline schema resolution, and disabled entity expansion are defaults rather than options.
---

<span class="badge-boundary">Pre-release · publication blocked</span>

## Scope at a glance

`openbim-idm` reads, writes, inspects, validates, and schema-edits the machine-readable idmXML format associated with ISO 29481-3. It does not implement ISO 29481-2 BPMN process maps, repair source schemas, or redistribute standards material.

The project is useful locally today, but is deliberately not publishable until Annex B redistribution rights and package provenance are resolved. Start with the [capability guide](/guide/getting-started), then review the [schema boundary](/guide/schema-validation).

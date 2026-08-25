# Documentation instructions

Scope: `docs/`.

- State implemented capabilities and non-goals honestly.
- Never copy XSD/PDF/standards prose into docs or static assets.
- Keep stable URLs for guide, API, architecture, security, provenance, and changelog.
- Verify with `npm run docs:build` and scan `.vitepress/dist` using the leakage gate.
- Use VitePress tokens and scoped theme CSS; keep navigation accessible and responsive.

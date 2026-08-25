# Security policy

## Supported versions

No public release is supported yet. Security fixes target `main` while version `0.1.0` remains publication-blocked.

## Reporting

Use GitHub's private vulnerability reporting for `openbimrs/idm` when available. Otherwise contact the openbimrs maintainers privately; do not publish exploit details or copyrighted standards material in an issue.

## Security posture

- XML input is bounded to 64 MiB by default and nesting to 1,024 levels.
- `DOCTYPE` declarations and DTD-defined entity references are rejected by the Rust parser; predefined and valid numeric character references are preserved without external expansion.
- Python XSD parsing disables network, DTD loading, and entity resolution.
- XSD includes are restricted to the known six filenames in one explicit local directory.
- Unknown XML extensions are preserved and reported, not executed.
- No schema, PDF, credential, or network-fetched standards content is embedded.

This library does not provide sandboxing, digital-signature verification, or trust decisions. Treat untrusted files as untrusted data and apply stricter application-specific size limits where appropriate.

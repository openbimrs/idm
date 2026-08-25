# Security

The project treats idmXML documents and XSD files as untrusted data.

## Controls

- Rust parsing rejects inputs over 64 MiB by default and nesting beyond 1,024 elements.
- Applications may lower the per-document byte ceiling.
- `DOCTYPE` declarations are rejected; XML entities are not accepted as a processing path.
- Python `lxml` parsers use `no_network=True`, `resolve_entities=False`, and `load_dtd=False`.
- XSD include resolution accepts only the six known filenames in one explicit directory and blocks external URLs.
- No XSD, PDF, credential, or standards material is embedded in source, Cargo packages, Python artifacts, or Pages output.

## Non-guarantees

The library does not sandbox a caller, establish authenticity, verify digital signatures, decide whether an input schema is trustworthy, or repair malformed/defective standards files. Resource limits are defenses, not a replacement for process isolation in high-risk ingestion services.

See the repository [SECURITY.md](https://github.com/openbimrs/idm/blob/main/SECURITY.md) for reporting instructions.

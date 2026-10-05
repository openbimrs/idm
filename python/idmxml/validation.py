"""Optional formal XSD validation against explicitly supplied local schemas."""

from __future__ import annotations

from functools import lru_cache
from pathlib import Path
from typing import Any, Union
from urllib.parse import unquote, urlparse

from .document import Document
from .schema import FILES, resolve_dir

DEFAULT_MAX_XML_BYTES = 64 * 1024 * 1024
PathLike = Union[str, Path]


@lru_cache(maxsize=8)
def _compiled_xsd_schema(root_schema: str) -> Any:
    """Compile one explicitly supplied, offline schema graph."""
    try:
        from lxml import etree
    except ImportError as exc:  # pragma: no cover - environment dependent
        raise RuntimeError("XSD validation requires: pip install 'idmxml[xsd]'") from exc

    root_path = Path(root_schema).resolve(strict=True)
    schema_dir = root_path.parent
    allowed = {name: (schema_dir / name).resolve(strict=True) for name in FILES}
    if root_path not in allowed.values():
        raise ValueError(f"root schema must be one of {FILES}: {root_path}")
    for path in allowed.values():
        source = path.read_bytes().upper()
        if b"<!DOCTYPE" in source or b"<!ENTITY" in source:
            raise ValueError(f"DOCTYPE and ENTITY declarations are blocked in schemas: {path}")

    class _SchemaResolver(etree.Resolver):
        def resolve(self, url: str, public_id: str | None, context: Any) -> Any:
            del public_id
            parsed = urlparse(url)
            if parsed.scheme not in ("", "file"):
                raise OSError(f"external schema URL is blocked: {url}")
            filename = Path(unquote(parsed.path)).name
            candidate = allowed.get(filename)
            if candidate is None:
                raise OSError(f"schema include outside the six-file allowlist is blocked: {url}")
            return self.resolve_filename(str(candidate), context)

    parser = etree.XMLParser(no_network=True, resolve_entities=False, load_dtd=False)
    parser.resolvers.add(_SchemaResolver())
    schema_doc = etree.parse(str(root_path), parser)
    return etree.XMLSchema(schema_doc)


def xsd_validate(
    document: Document | str,
    *,
    schema_dir: str | Path | None = None,
    schema_path: str | Path | None = None,
) -> list[dict[str, Any]]:
    """Validate against an explicitly supplied local XSD graph, offline and entity-safe.

    Pass either ``schema_dir`` (whose root is ``idm.xsd``), ``schema_path``, or
    set ``IDMXML_SCHEMA_DIR``. The six Annex B files are not redistributed.
    """
    try:
        from lxml import etree
    except ImportError as exc:  # pragma: no cover - environment dependent
        raise RuntimeError("XSD validation requires: pip install 'idmxml[xsd]'") from exc

    if schema_dir is not None and schema_path is not None:
        raise ValueError("pass only one of schema_dir and schema_path")
    root_schema = (
        Path(schema_path).expanduser().resolve(strict=True)
        if schema_path is not None
        else resolve_dir(schema_dir) / "idm.xsd"
    )
    schema = _compiled_xsd_schema(str(root_schema))
    xml = document.to_xml(pretty=False) if isinstance(document, Document) else document
    actual_bytes = len(xml.encode())
    if actual_bytes > DEFAULT_MAX_XML_BYTES:
        raise ValueError(
            f"XML input is {actual_bytes} bytes; the maximum is {DEFAULT_MAX_XML_BYTES} bytes"
        )
    tree = etree.fromstring(
        xml.encode(),
        etree.XMLParser(no_network=True, resolve_entities=False, load_dtd=False),
    )
    if tree.getroottree().docinfo.doctype:
        raise ValueError("DOCTYPE declarations are blocked in IDM documents")
    if schema.validate(tree):
        return []
    return [
        {
            "severity": "error",
            "code": "xsd",
            "line": entry.line,
            "column": entry.column,
            "message": entry.message,
        }
        for entry in schema.error_log
    ]

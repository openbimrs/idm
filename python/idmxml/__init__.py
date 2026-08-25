"""Python facade for the Rust-backed ISO 29481-3 idmXML engine."""

from __future__ import annotations

import json
import os
from functools import lru_cache
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlparse

from . import _native

SCHEMA_FILES = (
    "specId.xsd",
    "authoring.xsd",
    "uc.xsd",
    "businessContextMap.xsd",
    "er.xsd",
    "idm.xsd",
)
DEFAULT_MAX_XML_BYTES = 64 * 1024 * 1024


class Document:
    """Lossless recursive IDM document with schema-aware mutation."""

    __slots__ = ("_inner",)

    def __init__(self, inner: _native.Document) -> None:
        self._inner = inner

    @classmethod
    def parse(cls, xml: str) -> Document:
        return cls(_native.Document.parse(xml))

    @classmethod
    def new(cls, full_title: str, idm_code: str) -> Document:
        return cls(_native.Document.new(full_title, idm_code))

    @classmethod
    def from_dict(cls, value: dict[str, Any]) -> Document:
        return cls(_native.Document.from_json(json.dumps(value, ensure_ascii=False)))

    @property
    def root_name(self) -> str:
        return self._inner.root_name

    @property
    def root(self) -> str:
        """Root local name; concise alias used by document-store adapters."""
        return self.root_name

    @property
    def namespace(self) -> str | None:
        return self._inner.namespace

    def count(self, name: str) -> int:
        return self._inner.count(name)

    def element_paths(self, name: str) -> list[str]:
        return self._inner.element_paths(name)

    def to_xml(self, *, pretty: bool = True) -> str:
        return self._inner.to_xml(pretty)

    def to_dict(self) -> dict[str, Any]:
        return json.loads(self._inner.to_json(False))

    def validate(self) -> list[dict[str, Any]]:
        return json.loads(self._inner.validate_json())

    def text(self, path: str) -> str:
        return self._inner.text(path)

    def set_text(self, path: str, value: str) -> None:
        self._inner.set_text(path, value)

    def attribute(self, path: str, name: str) -> str:
        return self._inner.attribute(path, name)

    def get_attribute(self, path: str, name: str) -> str:
        """Read an attribute; symmetric alias for :meth:`set_attribute`."""
        return self.attribute(path, name)

    def set_attribute(self, path: str, name: str, value: str) -> None:
        self._inner.set_attribute(path, name, value)

    def allowed_children(self, parent_path: str) -> list[dict[str, Any]]:
        return json.loads(self._inner.allowed_children_json(parent_path))

    def append_schema_child(self, parent_path: str, name: str) -> str:
        return self._inner.append_schema_child(parent_path, name)

    def remove_schema_node(self, path: str) -> None:
        self._inner.remove_schema_node(path)

    def move_schema_node(self, path: str, target_path: str, *, after: bool) -> str:
        return self._inner.move_schema_node(path, target_path, after)

    def __repr__(self) -> str:
        return repr(self._inner)


def loads(xml: str) -> Document:
    return Document.parse(xml)


def load(path: str | Path) -> Document:
    return loads(Path(path).read_text(encoding="utf-8"))


def dumps(document: Document, *, pretty: bool = True) -> str:
    return document.to_xml(pretty=pretty)


def dump(document: Document, path: str | Path, *, pretty: bool = True) -> None:
    Path(path).write_text(dumps(document, pretty=pretty), encoding="utf-8")


def schema_catalog() -> dict[str, Any]:
    """Return all schema declarations, cardinalities and localization handles."""
    return json.loads(_native.schema_catalog_json())


def _schema_directory(schema_dir: str | Path | None) -> Path:
    configured = schema_dir or os.environ.get("IDMXML_SCHEMA_DIR")
    if configured is None:
        raise ValueError(
            "provide schema_dir or set IDMXML_SCHEMA_DIR to a lawfully obtained Annex B schema set"
        )
    directory = Path(configured).expanduser().resolve(strict=True)
    if not directory.is_dir():
        raise ValueError(f"schema directory is not a directory: {directory}")
    return directory


def schema_text(name: str, *, schema_dir: str | Path | None = None) -> str:
    """Read a recognized schema from an explicit directory; nothing is bundled."""
    if name not in SCHEMA_FILES:
        raise ValueError(f"unknown schema filename: {name}")
    return _native.read_schema_text(str(_schema_directory(schema_dir)), name)


@lru_cache(maxsize=8)
def _compiled_xsd_schema(root_schema: str) -> Any:
    """Compile one explicitly supplied, offline schema graph."""
    try:
        from lxml import etree
    except ImportError as exc:  # pragma: no cover - environment dependent
        raise RuntimeError("XSD validation requires: pip install 'idmxml[xsd]'") from exc

    root_path = Path(root_schema).resolve(strict=True)
    schema_dir = root_path.parent
    allowed = {name: (schema_dir / name).resolve(strict=True) for name in SCHEMA_FILES}
    if root_path not in allowed.values():
        raise ValueError(f"root schema must be one of {SCHEMA_FILES}: {root_path}")
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
        else _schema_directory(schema_dir) / "idm.xsd"
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


__all__ = [
    "DEFAULT_MAX_XML_BYTES",
    "SCHEMA_FILES",
    "Document",
    "dump",
    "dumps",
    "load",
    "loads",
    "schema_catalog",
    "schema_text",
    "xsd_validate",
]

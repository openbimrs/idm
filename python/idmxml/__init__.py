"""Python facade for the Rust-backed ISO 29481-3 idmXML engine.

The public API is organised in namespaces:

* :class:`idmxml.Document` - the lossless, schema-aware document
* :mod:`idmxml.fileio` - load/dump files and bytes, with encoding detection
* :mod:`idmxml.schema` - the declaration catalog and local schema-directory helpers
* :mod:`idmxml.validation` - optional formal XSD validation (needs ``idmxml[xsd]``)
* :mod:`idmxml.errors` - exception hierarchy (every error has a ``code``)
* :mod:`idmxml.models` - typed shapes of returned values

The flat names that predate the namespaces (``idmxml.load``, ``idmxml.schema_catalog``,
...) still resolve with a ``DeprecationWarning`` and will be removed in a later release.
"""

from __future__ import annotations

import warnings
from typing import Any

from . import errors, fileio, models, schema, validation
from .document import Document

__all__ = ["Document", "errors", "fileio", "models", "schema", "validation"]

# legacy flat name -> (module, attribute, replacement spelling)
_DEPRECATED = {
    "SCHEMA_FILES": (schema, "FILES", "idmxml.schema.FILES"),
    "DEFAULT_MAX_XML_BYTES": (
        validation,
        "DEFAULT_MAX_XML_BYTES",
        "idmxml.validation.DEFAULT_MAX_XML_BYTES",
    ),
    "load": (fileio, "load", "idmxml.fileio.load"),
    "loads": (fileio, "loads", "idmxml.fileio.loads"),
    "dump": (fileio, "dump", "idmxml.fileio.dump"),
    "dumps": (fileio, "dumps", "idmxml.fileio.dumps"),
    "schema_catalog": (schema, "catalog", "idmxml.schema.catalog"),
    "schema_text": (schema, "read_text", "idmxml.schema.read_text"),
    "xsd_validate": (validation, "xsd_validate", "idmxml.validation.xsd_validate"),
}


def __getattr__(name: str) -> Any:
    try:
        module, attribute, replacement = _DEPRECATED[name]
    except KeyError:
        raise AttributeError(f"module 'idmxml' has no attribute {name!r}") from None
    warnings.warn(
        f"idmxml.{name} is deprecated; use {replacement}",
        DeprecationWarning,
        stacklevel=2,
    )
    return getattr(module, attribute)


def __dir__() -> list[str]:
    return sorted([*__all__, *_DEPRECATED])

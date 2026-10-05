"""Exception hierarchy. Every error derives from :class:`IdmError` (a ``ValueError``)
and carries a stable machine-readable ``code`` attribute."""

from __future__ import annotations

from ._native import (
    CardinalityError,
    ContentModelError,
    EncodingError,
    IdmError,
    IdmIoError,
    JsonError,
    PathError,
    SchemaError,
    XmlError,
)

__all__ = [
    "CardinalityError",
    "ContentModelError",
    "EncodingError",
    "IdmError",
    "IdmIoError",
    "JsonError",
    "PathError",
    "SchemaError",
    "XmlError",
]

"""The generated declaration catalog and local schema-directory helpers."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Union, cast

from . import _native
from .models import SchemaVerification

FILES = (
    "specId.xsd",
    "authoring.xsd",
    "uc.xsd",
    "businessContextMap.xsd",
    "er.xsd",
    "idm.xsd",
)

PathLike = Union[str, Path]


def catalog() -> dict[str, Any]:
    """Return all schema declarations, cardinalities and localization handles."""
    return cast("dict[str, Any]", json.loads(_native.schema_catalog_json()))


def resolve_dir(schema_dir: PathLike | None = None) -> Path:
    """The explicit schema directory, or ``IDMXML_SCHEMA_DIR``; never a fallback."""
    configured = schema_dir or os.environ.get("IDMXML_SCHEMA_DIR")
    if configured is None:
        raise ValueError(
            "provide schema_dir or set IDMXML_SCHEMA_DIR to a lawfully obtained Annex B schema set"
        )
    directory = Path(configured).expanduser().resolve(strict=True)
    if not directory.is_dir():
        raise ValueError(f"schema directory is not a directory: {directory}")
    return directory


def read_text(name: str, *, schema_dir: PathLike | None = None) -> str:
    """Read a recognized schema from an explicit directory; nothing is bundled."""
    if name not in FILES:
        raise ValueError(f"unknown schema filename: {name}")
    return _native.read_schema_text(str(resolve_dir(schema_dir)), name)


def verify(schema_dir: PathLike | None = None) -> SchemaVerification:
    """Compare a local schema directory with the catalog's recorded source hashes.

    Tells you whether the catalog-driven validator applies to your schema set.
    """
    report = _native.verify_schema_dir_json(str(resolve_dir(schema_dir)))
    return cast(SchemaVerification, json.loads(report))

"""Reading and writing documents on disk and from bytes."""

from __future__ import annotations

import contextlib
import json
import os
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Union

from . import _native
from .document import Document

PathLike = Union[str, Path]


@dataclass(frozen=True)
class LoadResult:
    """A document plus what was learned about its byte encoding."""

    document: Document
    encoding: str
    declared_encoding: str | None
    had_bom: bool
    warnings: list[str] = field(default_factory=list)


def loads(xml: str) -> Document:
    return Document.parse(xml)


def load_bytes(data: bytes) -> LoadResult:
    inner, metadata = _native.Document.parse_bytes(data)
    info = json.loads(metadata)
    return LoadResult(
        document=Document(inner),
        encoding=info["encoding"],
        declared_encoding=info["declared_encoding"],
        had_bom=info["had_bom"],
        warnings=list(info["warnings"]),
    )


def load(path: PathLike) -> Document:
    """Read a file; the encoding is detected from its BOM/declaration."""
    return load_bytes(Path(path).read_bytes()).document


def load_with_info(path: PathLike) -> LoadResult:
    return load_bytes(Path(path).read_bytes())


def dumps(document: Document, *, pretty: bool = True) -> str:
    return document.to_xml(pretty=pretty)


def dump(
    document: Document, path: PathLike, *, pretty: bool = True, encoding: str = "utf-8"
) -> None:
    """Write a file atomically: serialize first, write a temporary file in the
    same directory, then replace the target, so a failure never truncates it."""
    data = document.to_bytes(pretty=pretty, encoding=encoding)
    target = Path(path)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{target.name}.", dir=target.parent or ".")
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(data)
        os.replace(temporary, target)
    except BaseException:
        with contextlib.suppress(FileNotFoundError):
            os.unlink(temporary)
        raise

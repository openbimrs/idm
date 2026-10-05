"""Reading and writing documents on disk and from bytes."""

from __future__ import annotations

import json
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
    """Write a file; the content is serialized before the file is touched."""
    Path(path).write_bytes(document.to_bytes(pretty=pretty, encoding=encoding))

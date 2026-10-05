"""Typed shapes of the JSON-like values the API returns and accepts."""

from __future__ import annotations

from typing import Any, TypedDict, Union


class _IssueRequired(TypedDict):
    severity: str
    code: str
    path: str
    message: str


class Issue(_IssueRequired, total=False):
    """A validation diagnostic. ``attribute``/``expected`` are present when relevant."""

    attribute: str
    expected: dict[str, Any]


class ChildAction(TypedDict):
    name: str
    definition: str
    label_key: str
    min_occurs: int
    max_occurs: int | None
    current: int
    can_add: bool
    recursive: bool


class AttributeInfo(TypedDict):
    name: str
    qualified_name: str
    value: str
    declared: bool


class ChildInfo(TypedDict):
    name: str
    path: str
    declared: bool


class NodeInfo(TypedDict):
    path: str
    name: str
    namespace: str | None
    handle: str | None
    text: str
    attributes: list[AttributeInfo]
    children: list[ChildInfo]


class AttributeSlot(TypedDict):
    rule: dict[str, Any]
    present: bool
    value: str | None


class SchemaFileCheck(TypedDict):
    file: str
    status: str
    expected_sha256: str
    actual_sha256: str | None


class SchemaVerification(TypedDict):
    checks: list[SchemaFileCheck]
    ok: bool


# An Edit is a tagged dict, e.g. {"op": "set_text", "path": "...", "value": "..."}.
Edit = dict[str, Any]


class EditBatch(TypedDict):
    version: int
    edits: list[Edit]


# A lossless XML element tree as produced by ``Document.element``.
ElementTree = dict[str, Any]

Locator = Union[str]
"""An indexed path (``/idm/uc[0]``) or a ``guid:<value>`` / ``id:<value>`` locator."""

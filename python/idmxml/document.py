from __future__ import annotations

import json
from typing import Any, cast

from . import _native
from .models import (
    AttributeInfo,
    AttributeSlot,
    ChildAction,
    Edit,
    EditBatch,
    ElementTree,
    Issue,
    NodeInfo,
)


class Document:
    """Lossless recursive IDM document with schema-aware mutation.

    Wherever a ``path`` is accepted you may also pass a ``guid:<value>`` or
    ``id:<value>`` locator, which keeps working across inserts and removals.
    """

    __slots__ = ("_inner",)
    __hash__ = None  # type: ignore[assignment]  # mutable, compared by content

    def __init__(self, inner: _native.Document) -> None:
        self._inner = inner

    # -- construction -----------------------------------------------------
    @classmethod
    def parse(cls, xml: str) -> Document:
        return cls(_native.Document.parse(xml))

    @classmethod
    def from_bytes(cls, data: bytes) -> Document:
        """Decode bytes (UTF-8/16, ISO-8859-1, windows-1252; BOM aware).

        Use :func:`idmxml.io.load_bytes` to also receive encoding metadata.
        """
        return cls(_native.Document.parse_bytes(data)[0])

    @classmethod
    def new(cls, full_title: str, idm_code: str) -> Document:
        return cls(_native.Document.new(full_title, idm_code))

    @classmethod
    def from_dict(cls, value: dict[str, Any]) -> Document:
        return cls(_native.Document.from_json(json.dumps(value, ensure_ascii=False)))

    def copy(self) -> Document:
        return Document(self._inner.copy())

    def __copy__(self) -> Document:
        return self.copy()

    def __deepcopy__(self, memo: Any) -> Document:
        return self.copy()

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Document) and self._inner == other._inner

    # -- reading ----------------------------------------------------------
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

    def find_by_guid(self, guid: str) -> list[str]:
        return self._inner.find_by_guid(guid)

    def find_by_id(self, id: str) -> list[str]:
        return self._inner.find_by_id(id)

    def path_of(self, locator: str) -> str:
        """Resolve a path or ``guid:``/``id:`` locator to its current indexed path."""
        return self._inner.path_of(locator)

    def to_xml(self, *, pretty: bool = True) -> str:
        return self._inner.to_xml(pretty)

    def to_bytes(self, *, pretty: bool = True, encoding: str = "utf-8") -> bytes:
        """Serialize in ``utf-8``, ``utf-16le``, ``utf-16be``, ``iso-8859-1`` or
        ``windows-1252``; unrepresentable characters raise ``EncodingError``."""
        return self._inner.to_bytes(pretty, encoding)

    def to_dict(self) -> dict[str, Any]:
        return cast("dict[str, Any]", json.loads(self._inner.to_json(False)))

    def element(self, path: str) -> ElementTree:
        """The complete lossless subtree below ``path``."""
        return cast(ElementTree, json.loads(self._inner.element_json(path)))

    def validate(self) -> list[Issue]:
        return cast("list[Issue]", json.loads(self._inner.validate_json()))

    def text(self, path: str) -> str:
        return self._inner.text(path)

    def attribute(self, path: str, name: str) -> str:
        return self._inner.attribute(path, name)

    def get_attribute(self, path: str, name: str) -> str:
        """Read an attribute; symmetric alias for :meth:`set_attribute`."""
        return self.attribute(path, name)

    def attributes(self, path: str) -> list[AttributeInfo]:
        return cast("list[AttributeInfo]", json.loads(self._inner.attributes_json(path)))

    def attribute_slots(self, path: str) -> list[AttributeSlot]:
        """Every attribute the schema declares here, with current presence/value."""
        return cast("list[AttributeSlot]", json.loads(self._inner.attribute_slots_json(path)))

    def node(self, path: str) -> NodeInfo:
        """Attributes, text, children and schema handle of one element."""
        return cast(NodeInfo, json.loads(self._inner.node_info_json(path)))

    def schema_rule(self, path: str) -> dict[str, Any]:
        """The catalog rule applying at ``path`` (``SchemaError`` for extensions)."""
        return cast("dict[str, Any]", json.loads(self._inner.schema_rule_json(path)))

    def allowed_children(self, parent_path: str) -> list[ChildAction]:
        return cast("list[ChildAction]", json.loads(self._inner.allowed_children_json(parent_path)))

    # -- writing ----------------------------------------------------------
    def set_text(self, path: str, value: str) -> None:
        self._inner.set_text(path, value)

    def set_text_unchecked(self, path: str, value: str) -> None:
        """Like :meth:`set_text` without the element-only content check."""
        self._inner.set_text_unchecked(path, value)

    def set_attribute(self, path: str, name: str, value: str) -> None:
        self._inner.set_attribute(path, name, value)

    def remove_attribute(self, path: str, name: str) -> None:
        self._inner.remove_attribute(path, name)

    def add_schema_attribute(self, path: str, name: str) -> str:
        """Add a schema-declared attribute with its default; returns the value."""
        return self._inner.add_schema_attribute(path, name)

    def append_schema_child(self, parent_path: str, name: str) -> str:
        return self._inner.append_schema_child(parent_path, name)

    def insert_schema_child(self, parent_path: str, name: str, position: int) -> str:
        return self._inner.insert_schema_child(parent_path, name, position)

    def remove_schema_node(self, path: str) -> None:
        self._inner.remove_schema_node(path)

    def duplicate_schema_node(self, path: str) -> str:
        """Duplicate right after itself with fresh ``guid``/``id`` values."""
        return self._inner.duplicate_schema_node(path)

    def move_schema_node(self, path: str, target_path: str, *, after: bool) -> str:
        return self._inner.move_schema_node(path, target_path, after)

    def reparent_schema_node(
        self, path: str, new_parent_path: str, *, position: int | None = None
    ) -> str:
        """Move below another allowed parent (``position`` = final same-name index)."""
        return self._inner.reparent_schema_node(path, new_parent_path, position)

    def paste_schema_node(
        self, parent_path: str, element: ElementTree, *, position: int | None = None
    ) -> str:
        """Insert a copy of an element tree; colliding guid/id values are regenerated."""
        return self._inner.paste_schema_node_json(
            parent_path, json.dumps(element, ensure_ascii=False), position
        )

    # -- reversible edits ---------------------------------------------------
    def apply(self, edit: Edit) -> Edit:
        """Apply one edit and return its inverse; unchanged on error."""
        return cast(Edit, json.loads(self._inner.apply_json(json.dumps(edit))))

    def apply_batch(self, batch: EditBatch) -> EditBatch:
        """Apply a versioned batch atomically; returns the inverse batch."""
        return cast(EditBatch, json.loads(self._inner.apply_batch_json(json.dumps(batch))))

    def diff(self, other: Document) -> EditBatch:
        """Best-effort edits that turn ``self`` into ``other``."""
        return cast(EditBatch, json.loads(self._inner.diff_json(other._inner)))

    def __repr__(self) -> str:
        return repr(self._inner)

#!/usr/bin/env python3
"""Generate the idmXML declaration catalog from an explicit local schema set.

The generated JSON is committed so Rust/Python/browser consumers do not need an
XSD parser at runtime. Re-running this script is the provenance check: schema
SHA-256s and declaration counts must change whenever the source schemas change.
No schema file is copied into the repository.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from lxml import etree

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "openbim-idm" / "catalog" / "catalog.json"
XSD_NS = "http://www.w3.org/2001/XMLSchema"
X = f"{{{XSD_NS}}}"
FILES = ("specId.xsd", "authoring.xsd", "uc.xsd", "businessContextMap.xsd", "er.xsd", "idm.xsd")
RECURSIVE_WRAPPERS = {
    ("idm", "subIdm"),
    ("uc", "subUc"),
    ("er", "subEr"),
    ("informationUnit", "subInformationUnit"),
    ("businessContextMap", "subBusinessContextMap"),
    ("pm", "subPm"),
}


def occurs(value: str | None, default: int = 1) -> int | None:
    if value == "unbounded":
        return None
    return int(value) if value is not None else default


def multiply(left: int | None, right: int | None) -> int | None:
    return None if left is None or right is None else left * right


def direct_complex_type(element: etree._Element) -> etree._Element | None:
    return element.find(f"{X}complexType")


def type_constraints(type_name: str | None, element: etree._Element, simple_types: dict[str, etree._Element]) -> tuple[str | None, str | None, list[str]]:
    simple = element.find(f"{X}simpleType")
    if simple is None and type_name:
        simple = simple_types.get(type_name.split(":")[-1])
    if simple is None:
        return type_name, None, []
    restriction = simple.find(f"{X}restriction")
    if restriction is None:
        return type_name, None, []
    pattern_node = restriction.find(f"{X}pattern")
    enum_nodes = restriction.findall(f"{X}enumeration")
    return (
        restriction.get("base") or type_name,
        pattern_node.get("value") if pattern_node is not None else None,
        [node.get("value") for node in enum_nodes if node.get("value") is not None],
    )


def main(schema_dir: Path, output: Path) -> None:
    schema_dir = schema_dir.expanduser().resolve(strict=True)
    parser = etree.XMLParser(resolve_entities=False, no_network=True, load_dtd=False)
    documents = {name: etree.parse(str(schema_dir / name), parser) for name in FILES}
    globals_by_name: dict[str, tuple[str, etree._Element]] = {}
    simple_types: dict[str, etree._Element] = {}
    for filename, document in documents.items():
        root = document.getroot()
        for element in root.findall(f"{X}element"):
            if element.get("name"):
                globals_by_name.setdefault(element.get("name"), (filename, element))
        for simple in root.findall(f"{X}simpleType"):
            if simple.get("name"):
                simple_types.setdefault(simple.get("name"), simple)

    definitions: dict[str, dict[str, Any]] = {}
    all_element_names: set[str] = set()
    all_attributes: set[str] = set()
    all_enums: set[str] = set()
    choice_counter = 0

    def build_definition(handle: str, name: str, source_file: str, element: etree._Element, *, global_element: bool) -> None:
        nonlocal choice_counter
        if handle in definitions:
            return
        all_element_names.add(name)
        definition: dict[str, Any] = {
            "handle": handle,
            "name": name,
            "global": global_element,
            "source_file": source_file,
            "source_line": element.sourceline,
            "label_key": f"schema.element.{name}",
            "attributes": [],
            "children": [],
            "choice_groups": [],
        }
        definitions[handle] = definition
        complex_type = direct_complex_type(element)
        if complex_type is None:
            data_type, pattern, enum_values = type_constraints(element.get("type"), element, simple_types)
            definition["data_type"] = data_type
            definition["pattern"] = pattern
            definition["enum_values"] = enum_values
            all_enums.update(enum_values)
            return

        for attribute in complex_type.findall(f"{X}attribute"):
            attr_name = attribute.get("name") or attribute.get("ref")
            if not attr_name:
                continue
            attr_name = attr_name.split(":")[-1]
            all_attributes.add(attr_name)
            data_type, pattern, enum_values = type_constraints(attribute.get("type"), attribute, simple_types)
            all_enums.update(enum_values)
            definition["attributes"].append({
                "name": attr_name,
                "required": attribute.get("use") == "required",
                "default": attribute.get("default"),
                "data_type": data_type,
                "pattern": pattern,
                "enum_values": enum_values,
                "label_key": f"schema.attribute.{attr_name}",
                "source_file": source_file,
                "source_line": attribute.sourceline,
            })

        def walk_particle(node: etree._Element, parent_min: int = 1, parent_max: int | None = 1, choice_group: str | None = None) -> None:
            nonlocal choice_counter
            tag = etree.QName(node).localname
            node_min = occurs(node.get("minOccurs"))
            node_max = occurs(node.get("maxOccurs"))
            effective_min = multiply(parent_min, node_min)
            effective_max = multiply(parent_max, node_max)
            if tag == "choice":
                group = f"{handle}.choice.{choice_counter}"
                choice_counter += 1
                definition["choice_groups"].append({
                    "handle": group,
                    "min_occurs": effective_min,
                    "max_occurs": effective_max,
                    "children": [],
                })
                for child in node:
                    if etree.QName(child).namespace == XSD_NS:
                        walk_particle(child, 0, effective_max, group)
                return
            if tag in {"sequence", "all"}:
                for child in node:
                    if etree.QName(child).namespace == XSD_NS:
                        walk_particle(child, effective_min or 0, effective_max, choice_group)
                return
            if tag != "element":
                return

            ref = node.get("ref")
            child_name = (ref or node.get("name") or "").split(":")[-1]
            if not child_name:
                return
            all_element_names.add(child_name)
            child_min = effective_min
            child_max = effective_max
            if ref:
                child_handle = child_name
            else:
                child_handle = f"{handle}/{child_name}"
                build_definition(child_handle, child_name, source_file, node, global_element=False)
            if choice_group is not None:
                group = next(
                    candidate
                    for candidate in definition["choice_groups"]
                    if candidate["handle"] == choice_group
                )
                group["children"].append(child_name)
            definition["children"].append({
                "name": child_name,
                "definition": child_handle,
                "min_occurs": child_min,
                "max_occurs": child_max,
                "choice_group": choice_group,
                "recursive": (name, child_name) in RECURSIVE_WRAPPERS,
                "label_key": f"schema.element.{child_name}",
                "source_file": source_file,
                "source_line": node.sourceline,
            })

        for particle in complex_type:
            if etree.QName(particle).namespace == XSD_NS and etree.QName(particle).localname in {"sequence", "choice", "all"}:
                walk_particle(particle)

    for name, (source_file, element) in sorted(globals_by_name.items()):
        build_definition(name, name, source_file, element, global_element=True)

    # Every named declaration/attribute in the source belongs in the inventory,
    # including declarations nested below anonymous local types.
    for document in documents.values():
        for element in document.findall(f".//{X}element"):
            value = element.get("name") or element.get("ref")
            if value:
                all_element_names.add(value.split(":")[-1])
        for attribute in document.findall(f".//{X}attribute"):
            value = attribute.get("name") or attribute.get("ref")
            if value:
                all_attributes.add(value.split(":")[-1])
        for enum in document.findall(f".//{X}enumeration"):
            if enum.get("value"):
                all_enums.add(enum.get("value"))

    recursive_edges = []
    for definition in definitions.values():
        for child in definition["children"]:
            if not child["recursive"]:
                continue
            wrapper = definitions.get(child["definition"])
            target = wrapper["children"][0]["name"] if wrapper and wrapper["children"] else definition["name"]
            recursive_edges.append(
                {"from": definition["name"], "wrapper": child["name"], "to": target}
            )
    recursive_edges.sort(key=lambda edge: (edge["from"], edge["wrapper"]))

    catalog = {
        "profile": "ISO 29481-3:2022 idmXML 0.2",
        "root": "idm",
        "namespace": None,
        "schemas": [
            {
                "file": name,
                "sha256": hashlib.sha256((schema_dir / name).read_bytes()).hexdigest(),
            }
            for name in FILES
        ],
        "element_names": sorted(all_element_names),
        "global_elements": sorted(globals_by_name),
        "attribute_names": sorted(all_attributes),
        "enum_values": sorted(all_enums),
        "recursive_edges": recursive_edges,
        "elements": definitions,
        "semantic_overlays": [
            {
                "code": "idm.er.required_by_standard",
                "path": "/idm",
                "child": "er",
                "min_occurs": 1,
                "source": "DIN EN ISO 29481-3:2024-09, Clause 5, p. 15",
                "reason": "The normative prose requires one ER although idm.xsd declares er minOccurs=0.",
            }
        ],
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(catalog, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(
        f"wrote {output}: {len(all_element_names)} elements, "
        f"{len(globals_by_name)} globals, {len(all_attributes)} attributes, "
        f"{len(all_enums)} enums, {len(definitions)} context definitions"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--schema-dir",
        required=True,
        type=Path,
        help="directory containing the six lawfully obtained Annex B XSD files",
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


if __name__ == "__main__":
    arguments = parse_args()
    main(arguments.schema_dir, arguments.output)

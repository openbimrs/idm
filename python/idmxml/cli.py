from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Callable

from . import fileio, schema, validation
from .document import Document
from .errors import IdmError


def _add_output(command: argparse.ArgumentParser) -> None:
    command.add_argument("-o", "--output", type=Path)
    command.add_argument("-i", "--in-place", action="store_true", help="overwrite the input file")
    command.add_argument("--encoding", default="utf-8")


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="idmpy",
        description="ISO 29481-3 idmXML tools",
        epilog=(
            "Exit codes: 0 success; 1 operational error; 2 validation errors or schema hash "
            "mismatch. Paths accept /idm/uc[0] or guid:<value> / id:<value> locators."
        ),
    )
    parser.add_argument("--json-errors", action="store_true", help="print errors as JSON")
    commands = parser.add_subparsers(dest="command", required=True)

    inspect = commands.add_parser("inspect", help="show recursive IDM metadata")
    inspect.add_argument("input", type=Path)
    inspect.add_argument("--json", action="store_true")

    validate = commands.add_parser("validate", help="run structural and optional XSD validation")
    validate.add_argument("input", type=Path)
    validate.add_argument("--json", action="store_true")
    validate.add_argument("--xsd", action="store_true")
    validate.add_argument("--schema-dir", type=Path)
    validate.add_argument("--schema", type=Path, help="explicit root XSD path")

    new = commands.add_parser("new", help="create a complete IDM skeleton")
    new.add_argument("title")
    new.add_argument("code")
    new.add_argument("-o", "--output", type=Path)
    new.add_argument("--encoding", default="utf-8")

    schema_cmd = commands.add_parser("schema", help="show all Annex B declarations")
    schema_cmd.add_argument("--json", action="store_true")
    schema_cmd.add_argument("--verify", type=Path, metavar="DIR", help="check local XSD hashes")

    for name, helptext in (
        ("format", "reformat XML"),
        ("duplicate", "duplicate a subtree with fresh guid/id values"),
        ("remove", "remove a node the content model permits removing"),
    ):
        command = commands.add_parser(name, help=helptext)
        command.add_argument("input", type=Path)
        if name != "format":
            command.add_argument("path")
        _add_output(command)

    get = commands.add_parser("get", help="read element text")
    get.add_argument("input", type=Path)
    get.add_argument("path")

    set_text = commands.add_parser("set", help="set element text")
    set_text.add_argument("input", type=Path)
    set_text.add_argument("path")
    set_text.add_argument("value")
    _add_output(set_text)

    set_attribute = commands.add_parser("set-attribute", help="set an attribute")
    set_attribute.add_argument("input", type=Path)
    set_attribute.add_argument("path")
    set_attribute.add_argument("name")
    set_attribute.add_argument("value")
    _add_output(set_attribute)

    for name in ("remove-attribute", "add-attribute"):
        command = commands.add_parser(name, help=name.replace("-", " "))
        command.add_argument("input", type=Path)
        command.add_argument("path")
        command.add_argument("name")
        _add_output(command)

    allowed = commands.add_parser("allowed", help="list permitted child actions")
    allowed.add_argument("input", type=Path)
    allowed.add_argument("path")
    allowed.add_argument("--json", action="store_true")

    add = commands.add_parser("add", help="add a cardinality-checked child")
    add.add_argument("input", type=Path)
    add.add_argument("parent_path")
    add.add_argument("child")
    _add_output(add)

    insert = commands.add_parser("insert", help="add a child at a sibling position")
    insert.add_argument("input", type=Path)
    insert.add_argument("parent_path")
    insert.add_argument("child")
    insert.add_argument("--position", type=int, required=True)
    _add_output(insert)

    move = commands.add_parser("move", help="move a subtree below another parent")
    move.add_argument("input", type=Path)
    move.add_argument("path")
    move.add_argument("new_parent")
    move.add_argument("--position", type=int)
    _add_output(move)

    find = commands.add_parser("find", help="find element paths")
    find.add_argument("input", type=Path)
    find.add_argument("--name")
    find.add_argument("--guid")
    find.add_argument("--id")

    listing = commands.add_parser("list", help="list element paths")
    listing.add_argument("input", type=Path)
    listing.add_argument("name", nargs="?")

    node = commands.add_parser("node", help="show one element")
    node.add_argument("input", type=Path)
    node.add_argument("path")

    rule = commands.add_parser("rule", help="show the catalog rule at a path")
    rule.add_argument("input", type=Path)
    rule.add_argument("path")

    diff = commands.add_parser("diff", help="print the edits between two documents")
    diff.add_argument("before", type=Path)
    diff.add_argument("after", type=Path)

    apply = commands.add_parser("apply", help="apply a JSON edit batch atomically")
    apply.add_argument("input", type=Path)
    apply.add_argument("edits", type=Path)
    apply.add_argument("--undo", type=Path, help="write the inverse batch here")
    _add_output(apply)
    return parser


def _read(path: Path) -> Document:
    info = fileio.load_with_info(path)
    for warning in info.warnings:
        print(f"idmpy: note: {warning}", file=sys.stderr)
    return info.document


def _emit(args: argparse.Namespace, document: Document) -> None:
    if args.in_place:
        fileio.dump(document, args.input, encoding=args.encoding)
    elif args.output:
        fileio.dump(document, args.output, encoding=args.encoding)
    else:
        sys.stdout.write(document.to_xml())
        sys.stdout.write("\n")


def _edit(args: argparse.Namespace, change: Callable[[Document], object]) -> int:
    document = _read(args.input)
    change(document)
    _emit(args, document)
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        return _run(args)
    except IdmError as error:
        if args.json_errors:
            print(json.dumps({"code": error.code, "message": str(error)}), file=sys.stderr)
        else:
            print(f"idmpy: {error}", file=sys.stderr)
        return 1


def _run(args: argparse.Namespace) -> int:
    command = args.command
    if command == "inspect":
        document = _read(args.input)
        issues = document.validate()
        summary = {
            "root": document.root_name,
            "use_cases": document.count("uc"),
            "business_context_maps": document.count("businessContextMap"),
            "exchange_requirements": document.count("er"),
            "errors": sum(issue["severity"] == "error" for issue in issues),
            "warnings": sum(issue["severity"] == "warning" for issue in issues),
            "issues": issues,
        }
        if args.json:
            print(json.dumps(summary, indent=2))
        else:
            for key in (
                "root",
                "use_cases",
                "business_context_maps",
                "exchange_requirements",
                "errors",
                "warnings",
            ):
                print(f"{key}: {summary[key]}")
        return 0
    if command == "validate":
        document = _read(args.input)
        issues = list(document.validate())
        if args.xsd:
            issues.extend(
                validation.xsd_validate(  # type: ignore[arg-type]
                    document, schema_dir=args.schema_dir, schema_path=args.schema
                )
            )
        if args.json:
            print(json.dumps(issues, indent=2))
        else:
            for issue in issues:
                print(f"{issue['severity']} {issue['code']}: {issue['message']}")
        return 2 if any(issue["severity"] == "error" for issue in issues) else 0
    if command == "new":
        document = Document.new(args.title, args.code)
        if args.output:
            fileio.dump(document, args.output, encoding=args.encoding)
        else:
            print(document.to_xml())
        return 0
    if command == "schema":
        if args.verify:
            report = schema.verify(args.verify)
            if args.json:
                print(json.dumps(report, indent=2))
            else:
                for check in report["checks"]:
                    print(f"{check['status'].upper()} {check['file']}")
            return 0 if report["ok"] else 2
        catalog = schema.catalog()
        if args.json:
            print(json.dumps(catalog, indent=2))
        else:
            print(f"profile: {catalog['profile']}")
            print(f"elements: {len(catalog['element_names'])}")
            print(f"attributes: {len(catalog['attribute_names'])}")
        return 0
    if command == "format":
        return _edit(args, lambda document: None)
    if command == "get":
        print(_read(args.input).text(args.path))
        return 0
    if command == "set":
        return _edit(args, lambda d: d.set_text(args.path, args.value))
    if command == "set-attribute":
        return _edit(args, lambda d: d.set_attribute(args.path, args.name, args.value))
    if command == "remove-attribute":
        return _edit(args, lambda d: d.remove_attribute(args.path, args.name))
    if command == "add-attribute":
        return _edit(args, lambda d: d.add_schema_attribute(args.path, args.name))
    if command == "allowed":
        actions = _read(args.input).allowed_children(args.path)
        if args.json:
            print(json.dumps(actions, indent=2))
        else:
            for action in actions:
                maximum = "*" if action["max_occurs"] is None else action["max_occurs"]
                print(
                    f"{action['name']} [{action['min_occurs']}, {maximum}] "
                    f"current={action['current']} add={str(action['can_add']).lower()}"
                )
        return 0
    if command == "add":
        return _edit(args, lambda d: d.append_schema_child(args.parent_path, args.child))
    if command == "insert":
        return _edit(
            args, lambda d: d.insert_schema_child(args.parent_path, args.child, args.position)
        )
    if command == "remove":
        return _edit(args, lambda d: d.remove_schema_node(args.path))
    if command == "duplicate":
        return _edit(args, lambda d: d.duplicate_schema_node(args.path))
    if command == "move":
        return _edit(
            args,
            lambda d: d.reparent_schema_node(args.path, args.new_parent, position=args.position),
        )
    if command == "find":
        if args.name is None and args.guid is None and args.id is None:
            raise IdmError("pass at least one of --name, --guid or --id")
        document = _read(args.input)
        found: list[str] | None = None
        for candidate in (
            document.element_paths(args.name) if args.name else None,
            document.find_by_guid(args.guid) if args.guid else None,
            document.find_by_id(args.id) if args.id else None,
        ):
            if candidate is not None:
                found = candidate if found is None else [p for p in found if p in candidate]
        for path in found or []:
            print(path)
        return 0 if found else 1
    if command == "list":
        document = _read(args.input)
        if args.name:
            paths = document.element_paths(args.name)
        else:
            paths, queue = [], ["/idm"]
            while queue:
                current = queue.pop(0)
                paths.append(current)
                queue.extend(child["path"] for child in document.node(current)["children"])
        print("\n".join(paths))
        return 0
    if command == "node":
        print(json.dumps(_read(args.input).node(args.path), indent=2))
        return 0
    if command == "rule":
        print(json.dumps(_read(args.input).schema_rule(args.path), indent=2))
        return 0
    if command == "diff":
        print(json.dumps(_read(args.before).diff(_read(args.after)), indent=2))
        return 0
    if command == "apply":
        document = _read(args.input)
        batch = json.loads(args.edits.read_text(encoding="utf-8"))
        inverse = document.apply_batch(batch)
        if args.undo:
            args.undo.write_text(json.dumps(inverse, indent=2), encoding="utf-8")
        _emit(args, document)
        return 0
    raise AssertionError(f"unhandled command: {command}")


def entrypoint() -> None:
    raise SystemExit(main())

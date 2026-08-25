from __future__ import annotations

import argparse
import json
from collections.abc import Sequence
from pathlib import Path

from . import Document, dump, load, schema_catalog, xsd_validate


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="idmpy", description="ISO 29481-3 idmXML tools")
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

    schema = commands.add_parser("schema", help="show all Annex B declarations")
    schema.add_argument("--json", action="store_true")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "inspect":
        document = load(args.input)
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
            for key in ("root", "use_cases", "business_context_maps", "exchange_requirements", "errors", "warnings"):
                print(f"{key}: {summary[key]}")
        return 0
    if args.command == "validate":
        document = load(args.input)
        issues = document.validate()
        if args.xsd:
            issues.extend(
                xsd_validate(document, schema_dir=args.schema_dir, schema_path=args.schema)
            )
        if args.json:
            print(json.dumps(issues, indent=2))
        else:
            for issue in issues:
                print(f"{issue['severity']} {issue['code']}: {issue['message']}")
        return 2 if any(issue["severity"] == "error" for issue in issues) else 0
    if args.command == "new":
        document = Document.new(args.title, args.code)
        if args.output:
            dump(document, args.output)
        else:
            print(document.to_xml())
        return 0
    if args.command == "schema":
        catalog = schema_catalog()
        if args.json:
            print(json.dumps(catalog, indent=2))
        else:
            print(f"profile: {catalog['profile']}")
            print(f"elements: {len(catalog['element_names'])}")
            print(f"attributes: {len(catalog['attribute_names'])}")
        return 0
    raise AssertionError(f"unhandled command: {args.command}")


def entrypoint() -> None:
    raise SystemExit(main())

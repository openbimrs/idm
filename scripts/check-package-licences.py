#!/usr/bin/env python3
"""Check that each packaged .crate ships the root licence files byte for byte
and declares the dual-licence expression.

usage: check-package-licences.py CRATE...
"""

import pathlib
import re
import sys
import tarfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
EXPR = "AGPL-3.0-or-later OR LicenseRef-Commercial"
FILES = ("LICENSE", "NOTICE", "LICENSES/LicenseRef-Commercial.txt")


def check(crate: pathlib.Path) -> list[str]:
    errors = []
    with tarfile.open(crate) as tar:
        members = {
            m.name.split("/", 1)[1]: m for m in tar.getmembers() if m.isfile() and "/" in m.name
        }
        for name in FILES:
            m = members.get(name)
            if m is None:
                errors.append(f"{crate.name}: {name} missing")
            elif tar.extractfile(m).read() != (ROOT / name).read_bytes():
                errors.append(f"{crate.name}: {name} differs from the root")
        manifest = tar.extractfile(members["Cargo.toml"]).read().decode()
        if not re.search(rf'^license = "{re.escape(EXPR)}"$', manifest, re.MULTILINE):
            errors.append(f"{crate.name}: license expression is not {EXPR!r}")
    return errors


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    errors = [e for a in argv for e in check(pathlib.Path(a))]
    for e in errors:
        print(f"licence check: {e}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

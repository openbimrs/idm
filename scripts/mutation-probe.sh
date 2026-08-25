#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="$ROOT/idmxml/src/lib.rs"
BEFORE="$(sha256sum "$SOURCE" | cut -d' ' -f1)"
TEMP="$(mktemp -d)"
LEAKAGE_MUTATION="$ROOT/legal-boundary-mutation.txt"
trap 'rm -rf "$TEMP"; rm -f "$LEAKAGE_MUTATION"' EXIT

cp "$ROOT/Cargo.toml" "$ROOT/pyproject.toml" "$TEMP/"
cp -a "$ROOT/openbim-idm" "$ROOT/idmxml" "$TEMP/"

REPO_ROOT="$TEMP" python3 "$ROOT/scripts/check-alias-purity.py" >/dev/null
printf '\npub struct IndependentAliasType;\n' >> "$TEMP/idmxml/src/lib.rs"
if REPO_ROOT="$TEMP" python3 "$ROOT/scripts/check-alias-purity.py" >/dev/null 2>&1; then
  echo "mutation-probe: FAIL (independent alias type was not detected)" >&2
  exit 1
fi
cp "$SOURCE" "$TEMP/idmxml/src/lib.rs"
REPO_ROOT="$TEMP" python3 "$ROOT/scripts/check-alias-purity.py" >/dev/null

python3 - "$TEMP/openbim-idm/Cargo.toml" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
source = path.read_text()
path.write_text(source.replace('repository = "https://github.com/openbimrs/idm"', 'repository.workspace = true', 1))
PY
if REPO_ROOT="$TEMP" python3 "$ROOT/scripts/check-alias-purity.py" >/dev/null 2>&1; then
  echo "mutation-probe: FAIL (superproject-breaking workspace metadata was not detected)" >&2
  exit 1
fi
cp "$ROOT/openbim-idm/Cargo.toml" "$TEMP/openbim-idm/Cargo.toml"
REPO_ROOT="$TEMP" python3 "$ROOT/scripts/check-alias-purity.py" >/dev/null

printf '%s%s xmlns:xs="http://www.w3.org/2001/XMLSchema"/>\n' '<xs:' 'schema' > "$LEAKAGE_MUTATION"
if python3 "$ROOT/scripts/check-leakage.py" >/dev/null 2>&1; then
  echo "mutation-probe: FAIL (renamed XSD payload was not detected)" >&2
  exit 1
fi
rm "$LEAKAGE_MUTATION"
python3 "$ROOT/scripts/check-leakage.py" >/dev/null

AFTER="$(sha256sum "$SOURCE" | cut -d' ' -f1)"
[[ "$BEFORE" == "$AFTER" ]] || { echo "mutation-probe: FAIL (working source changed)" >&2; exit 1; }
echo "mutation-probe: PASS (clean -> mutated failure -> restored clean; source hash unchanged)"

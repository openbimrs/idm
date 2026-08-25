#!/usr/bin/env python3
"""Enforce the canonical-crate/alias boundary and publication guards."""
from __future__ import annotations

import os
import sys
from pathlib import Path

import tomllib

ROOT = Path(os.environ.get("REPO_ROOT", Path(__file__).resolve().parents[1]))
CANONICAL = ROOT / "openbim-idm"
ALIAS = ROOT / "idmxml"
EXPECTED_SOURCE = "pub use openbim_idm::*;\n"


def fail(message: str) -> None:
    print(f"alias-purity: {message}", file=sys.stderr)
    raise SystemExit(1)


def read_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


root_manifest = read_toml(ROOT / "Cargo.toml")
canonical_manifest = read_toml(CANONICAL / "Cargo.toml")
alias_manifest = read_toml(ALIAS / "Cargo.toml")
pyproject = read_toml(ROOT / "pyproject.toml")
version = root_manifest["workspace"]["package"]["version"]

workspace_package = root_manifest["workspace"]["package"]
for label, manifest in (("canonical", canonical_manifest), ("alias", alias_manifest)):
    package = manifest["package"]
    for key in ("version", "edition", "rust-version", "license", "authors", "repository"):
        if package.get(key) != workspace_package.get(key):
            fail(
                f"{label} package must declare explicit {key} metadata matching the "
                "standalone workspace; workspace inheritance breaks when pinned in a superproject"
            )
    if manifest.get("lints") != root_manifest["workspace"]["lints"]:
        fail(
            f"{label} package must declare explicit lint settings matching the standalone "
            "workspace; workspace lint inheritance breaks when pinned in a superproject"
        )

if canonical_manifest["package"].get("publish") is not False:
    fail("canonical crate must set publish = false")
if alias_manifest["package"].get("publish") is not False:
    fail("alias crate must set publish = false")
if pyproject.get("tool", {}).get("openbim-idm", {}).get("publish") is not False:
    fail("Python publication guard tool.openbim-idm.publish must be false")

dependency = alias_manifest.get("dependencies", {}).get("openbim-idm")
if not isinstance(dependency, dict):
    fail("alias must have one structured openbim-idm dependency")
if dependency.get("version") != f"={version}":
    fail(f"canonical dependency must be pinned to ={version}")
if dependency.get("path") != "../openbim-idm":
    fail("canonical dependency path must be ../openbim-idm")
if dependency.get("default-features") is not False:
    fail("pure Rust alias must disable canonical default features")
if set(alias_manifest.get("dependencies", {})) != {"openbim-idm"}:
    fail("alias must not have other dependencies")
if "features" in alias_manifest:
    fail("pure Rust alias must not define feature-forwarding behavior")

source_files = sorted(path.relative_to(ALIAS).as_posix() for path in (ALIAS / "src").rglob("*") if path.is_file())
if source_files != ["src/lib.rs"]:
    fail(f"alias source must contain only src/lib.rs, found {source_files}")
if (ALIAS / "src" / "lib.rs").read_text(encoding="utf-8") != EXPECTED_SOURCE:
    fail("src/lib.rs must be exactly the canonical glob re-export")
for forbidden in ("build.rs", "tests", "examples", "benches"):
    if (ALIAS / forbidden).exists():
        fail(f"alias must not contain {forbidden}")
for section in ("lib", "bin", "example", "test", "bench", "build-dependencies", "dev-dependencies"):
    if section in alias_manifest:
        fail(f"alias manifest must not define [{section}]")

print(f"alias-purity: PASS (idmxml -> openbim-idm ={version}; publication blocked)")

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

import idmxml

FIXTURE = Path(__file__).parent / "fixtures" / "recursive-extension.xml"


def _write_synthetic_schema_set(directory: Path) -> None:
    """Write non-normative XSDs that exercise the six-file offline include graph."""
    schema_element = "xs:" + "schema"
    auxiliary = idmxml.SCHEMA_FILES[:-1]
    for index, name in enumerate(auxiliary):
        directory.joinpath(name).write_text(
            f"""<?xml version="1.0"?>
<{schema_element} xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:simpleType name="SyntheticType{index}">
    <xs:restriction base="xs:string"/>
  </xs:simpleType>
</{schema_element}>
""",
            encoding="utf-8",
        )
    includes = "\n".join(f'  <xs:include schemaLocation="{name}"/>' for name in auxiliary)
    directory.joinpath("idm.xsd").write_text(
        f"""<?xml version="1.0"?>
<{schema_element} xmlns:xs="http://www.w3.org/2001/XMLSchema">
{includes}
  <xs:element name="idm">
    <xs:complexType>
      <xs:sequence><xs:any minOccurs="0" maxOccurs="unbounded" processContents="lax"/></xs:sequence>
      <xs:anyAttribute processContents="lax"/>
    </xs:complexType>
  </xs:element>
</{schema_element}>
""",
        encoding="utf-8",
    )


def test_python_document_is_lossless_and_schema_aware() -> None:
    document = idmxml.load(FIXTURE)
    assert document.root_name == "idm"
    assert document.namespace is None
    assert document.count("uc") == 2
    before = document.to_dict()

    path = document.append_schema_child("/idm/uc[0]", "subUc")
    assert path.endswith("subUc[1]")
    assert document.count("uc") == 3
    assert document.to_dict() != before
    document.remove_schema_node(path)
    assert document.to_dict() == before


def test_python_builds_complete_idm_and_reports_catalog() -> None:
    document = idmxml.Document.new("Coordination", "IDM-001")
    assert document.text("/idm/specId[0]") == ""
    assert document.attribute("/idm/specId[0]", "fullTitle") == "Coordination"
    assert not [issue for issue in document.validate() if issue["severity"] == "error"]

    catalog = idmxml.schema_catalog()
    assert len(catalog["element_names"]) == 57
    assert catalog["elements"]["uc"]["children"][-1]["name"] == "subUc"


def test_python_module_cli_uses_native_idm_engine() -> None:
    result = subprocess.run(
        [sys.executable, "-m", "idmxml", "inspect", str(FIXTURE), "--json"],
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout)["root"] == "idm"


def test_optional_xsd_validation_is_offline_and_uses_explicit_synthetic_schemas(
    tmp_path: Path,
) -> None:
    pytest.importorskip("lxml")
    _write_synthetic_schema_set(tmp_path)
    document = idmxml.Document.new("Coordination", "IDM-001")
    assert idmxml.xsd_validate(document, schema_dir=tmp_path) == []
    assert "SyntheticType0" in idmxml.schema_text("specId.xsd", schema_dir=tmp_path)
    with pytest.raises(ValueError, match="DOCTYPE"):
        idmxml.xsd_validate("<!DOCTYPE idm><idm/>", schema_dir=tmp_path)


def test_xsd_validation_requires_an_explicit_schema_location(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    pytest.importorskip("lxml")
    monkeypatch.delenv("IDMXML_SCHEMA_DIR", raising=False)
    with pytest.raises(ValueError, match="provide schema_dir"):
        idmxml.xsd_validate("<idm/>")

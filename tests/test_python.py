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
    auxiliary = idmxml.schema.FILES[:-1]
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
    document = idmxml.fileio.load(FIXTURE)
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

    catalog = idmxml.schema.catalog()
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
    assert idmxml.validation.xsd_validate(document, schema_dir=tmp_path) == []
    assert "SyntheticType0" in idmxml.schema.read_text("specId.xsd", schema_dir=tmp_path)
    with pytest.raises(ValueError, match="DOCTYPE"):
        idmxml.validation.xsd_validate("<!DOCTYPE idm><idm/>", schema_dir=tmp_path)


def test_xsd_validation_requires_an_explicit_schema_location(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    pytest.importorskip("lxml")
    monkeypatch.delenv("IDMXML_SCHEMA_DIR", raising=False)
    with pytest.raises(ValueError, match="provide schema_dir"):
        idmxml.validation.xsd_validate("<idm/>")


def test_flat_legacy_names_still_resolve_with_a_deprecation_warning() -> None:
    with pytest.warns(DeprecationWarning, match=r"idmxml\.fileio\.load"):
        assert idmxml.load is idmxml.fileio.load
    with pytest.warns(DeprecationWarning, match=r"idmxml\.schema\.catalog"):
        assert idmxml.schema_catalog is idmxml.schema.catalog
    with pytest.raises(AttributeError):
        _ = idmxml.not_a_name


def test_errors_are_typed_value_errors_with_codes() -> None:
    document = idmxml.Document.new("T", "C")
    with pytest.raises(idmxml.errors.PathError) as missing:
        document.text("/idm/nope")
    assert missing.value.code == "path_not_found"
    assert isinstance(missing.value, ValueError)
    with pytest.raises(idmxml.errors.ContentModelError):
        document.set_text("/idm/uc", "text")
    with pytest.raises(idmxml.errors.CardinalityError):
        document.append_schema_child("/idm", "uc")
    with pytest.raises(idmxml.errors.XmlError) as bad:
        idmxml.Document.parse("<a>")
    assert bad.value.code == "invalid_xml"
    assert issubclass(idmxml.errors.XmlError, idmxml.errors.IdmError)


def test_validation_reports_datatypes_with_attribute_and_expectation() -> None:
    document = idmxml.Document.new("T", "C")
    assert document.validate() == []
    document.set_attribute("/idm/authoring/changeLog", "changeDateTime", "nope")
    issue = next(i for i in document.validate() if i["code"] == "attribute_datatype")
    assert issue["attribute"] == "changeDateTime"
    assert issue["expected"] == {"type": "xs:dateTime"}


def test_node_rule_slots_and_attribute_editing() -> None:
    document = idmxml.Document.new("T", "C")
    node = document.node("/idm/specId")
    assert node["handle"] == "specId"
    assert {a["name"] for a in node["attributes"]} >= {"guid", "fullTitle"}
    assert document.schema_rule("/idm/specId")["name"] == "specId"
    slots = {s["rule"]["name"]: s for s in document.attribute_slots("/idm/specId")}
    assert slots["guid"]["present"] and not slots["shortTitle"]["present"]
    document.add_schema_attribute("/idm/specId", "shortTitle")
    document.remove_attribute("/idm/specId", "shortTitle")
    with pytest.raises(idmxml.errors.ContentModelError):
        document.remove_attribute("/idm/specId", "guid")
    assert next(a["name"] for a in document.attributes("/idm/specId")) == "guid"
    assert document.element("/idm/specId")["local_name"] == "specId"


def test_locators_structure_ops_copy_and_equality() -> None:
    document = idmxml.Document.new("T", "C")
    document.append_schema_child("/idm/er", "informationUnit")
    snapshot = document.copy()
    assert snapshot == document and snapshot is not document
    unit_id = document.attribute("/idm/er/informationUnit[1]", "id")
    document.insert_schema_child("/idm/er", "informationUnit", 0)
    assert document.path_of(f"id:{unit_id}") == "/idm/er[0]/informationUnit[2]"
    assert document.find_by_id(unit_id) == ["/idm/er[0]/informationUnit[2]"]
    copy_path = document.duplicate_schema_node(f"id:{unit_id}")
    assert document.attribute(copy_path, "id") != unit_id
    document.reparent_schema_node(copy_path, "/idm/er", position=0)
    assert document != snapshot
    assert not [i for i in document.validate() if i["severity"] == "error"]
    with pytest.raises(TypeError):
        hash(document)


def test_paste_regenerates_colliding_identity() -> None:
    document = idmxml.Document.new("T", "C")
    unit = document.element("/idm/er/informationUnit[0]")
    path = document.paste_schema_node("/idm/er", unit, position=0)
    assert path == "/idm/er[0]/informationUnit[0]"
    assert not [i for i in document.validate() if i["code"] == "duplicate_id"]


def test_edits_are_reversible_and_diff_round_trips() -> None:
    document = idmxml.Document.new("T", "C")
    original = document.copy()
    edit = {"op": "set_attribute", "path": "/idm/specId", "name": "fullTitle", "value": "X"}
    inverse = document.apply(edit)
    assert document.attribute("/idm/specId", "fullTitle") == "X"
    document.apply(inverse)
    assert document == original

    changed = original.copy()
    changed.append_schema_child("/idm/er", "informationUnit")
    batch = original.diff(changed)
    assert batch["version"] == 1 and batch["edits"]
    undo = document.apply_batch(batch)
    assert document.to_xml() == changed.to_xml()
    document.apply_batch(undo)
    assert document == original
    with pytest.raises(idmxml.errors.PathError):
        document.apply({"op": "remove_child", "path": "/idm/er/missing[0]"})
    with pytest.raises(idmxml.errors.JsonError):
        document.apply({"op": "nope"})


def test_bytes_and_encodings_round_trip_with_metadata(tmp_path: Path) -> None:
    document = idmxml.Document.new("Gr\u00fc\u00dfe \u20ac", "C")
    for encoding in ("utf-8", "utf-16le", "utf-16be", "windows-1252"):
        info = idmxml.fileio.load_bytes(document.to_bytes(encoding=encoding))
        assert info.document.attribute("/idm/specId", "fullTitle") == "Gr\u00fc\u00dfe \u20ac"
        assert (info.encoding == "utf8") or info.warnings
    with pytest.raises(idmxml.errors.EncodingError):
        document.to_bytes(encoding="iso-8859-1")
    path = tmp_path / "doc.xml"
    idmxml.fileio.dump(document, path, pretty=False, encoding="utf-16le")
    assert idmxml.fileio.load(path) == document
    assert idmxml.Document.from_bytes(path.read_bytes()) == document


def test_schema_verify_reports_missing_files(tmp_path: Path) -> None:
    report = idmxml.schema.verify(tmp_path)
    assert not report["ok"]
    assert {c["status"] for c in report["checks"]} == {"missing"}
    assert len(report["checks"]) == 6


def _cli(*args: object) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, "-m", "idmxml", *map(str, args)],
        check=False,
        capture_output=True,
        text=True,
    )


def test_python_cli_edits_in_place_and_round_trips_edits(tmp_path: Path) -> None:
    model = tmp_path / "m.xml"
    assert _cli("new", "T", "C", "-o", model).returncode == 0
    assert _cli("add", model, "/idm/er", "informationUnit", "-i").returncode == 0
    assert (
        _cli("insert", model, "/idm/er", "informationUnit", "--position", "0", "-i").returncode == 0
    )
    assert _cli("duplicate", model, "/idm/er/informationUnit[0]", "-i").returncode == 0
    found = _cli("find", model, "--name", "informationUnit")
    assert found.stdout.split() == [f"/idm/er[0]/informationUnit[{i}]" for i in range(4)]
    assert _cli("validate", model).returncode == 0
    node = json.loads(_cli("node", model, "/idm/specId").stdout)
    assert node["name"] == "specId"
    failure = _cli("--json-errors", "get", model, "/idm/missing")
    assert failure.returncode == 1
    assert json.loads(failure.stderr.splitlines()[-1])["code"] == "path_not_found"

    other = tmp_path / "o.xml"
    assert (
        _cli("set-attribute", model, "/idm/specId", "fullTitle", "Z", "-o", other).returncode == 0
    )
    edits = tmp_path / "e.json"
    edits.write_text(_cli("diff", model, other).stdout, encoding="utf-8")
    undo = tmp_path / "u.json"
    applied = tmp_path / "a.xml"
    result = _cli("apply", model, edits, "--undo", undo, "-o", applied)
    assert result.returncode == 0, result.stderr
    assert json.loads(_cli("diff", applied, other).stdout)["edits"] == []
    assert json.loads(undo.read_text(encoding="utf-8"))["edits"]
    assert _cli("schema", "--verify", tmp_path).returncode == 2

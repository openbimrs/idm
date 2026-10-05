use openbim_idm::Document;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_idmxml"))
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recursive-extension.xml")
}

#[test]
fn cli_inspects_validates_and_exposes_the_complete_schema_catalog() {
    let inspect = Command::new(binary())
        .args(["inspect", fixture().to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let summary: Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(summary["root"], "idm");
    assert_eq!(summary["use_cases"], 2);
    assert_eq!(summary["business_context_maps"], 2);
    assert_eq!(summary["exchange_requirements"], 2);

    let schema = Command::new(binary())
        .args(["schema", "--json"])
        .output()
        .unwrap();
    assert!(schema.status.success());
    let catalog: Value = serde_json::from_slice(&schema.stdout).unwrap();
    assert_eq!(catalog["element_names"].as_array().unwrap().len(), 57);
    assert_eq!(catalog["attribute_names"].as_array().unwrap().len(), 38);
}

#[test]
fn cli_creates_and_schema_edits_a_recursive_idm() {
    let temporary = tempdir().unwrap();
    let created = temporary.path().join("created.xml");
    let recursive = temporary.path().join("recursive.xml");
    let new = Command::new(binary())
        .args([
            "new",
            "Coordination",
            "IDM-001",
            "-o",
            created.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        new.status.success(),
        "{}",
        String::from_utf8_lossy(&new.stderr)
    );

    let add = Command::new(binary())
        .args([
            "add",
            created.to_str().unwrap(),
            "/idm/uc[0]",
            "subUc",
            "-o",
            recursive.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let document = Document::parse(&std::fs::read_to_string(recursive).unwrap()).unwrap();
    assert_eq!(document.count("uc"), 2);
    assert!(
        document
            .element("/idm/uc[0]/subUc[0]/uc[0]/specId[0]")
            .is_ok()
    );
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(binary()).args(args).output().unwrap()
}

fn ok(args: &[&str]) -> String {
    let output = run(args);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn cli_edits_in_place_and_finds_by_locator() {
    let temporary = tempdir().unwrap();
    let file = temporary.path().join("m.xml");
    let file = file.to_str().unwrap();
    ok(&["new", "T", "C", "-o", file]);
    ok(&["add", file, "/idm/er", "informationUnit", "--in-place"]);
    ok(&[
        "insert",
        file,
        "/idm/er",
        "informationUnit",
        "--position",
        "0",
        "-i",
    ]);
    let units = ok(&["find", file, "--name", "informationUnit"]);
    assert_eq!(units.lines().count(), 3);

    let id = ok(&["get", file, "/idm/er/informationUnit[2]"]);
    assert_eq!(id.trim(), "");
    let node: Value =
        serde_json::from_str(&ok(&["node", file, "/idm/er/informationUnit[2]", "--json"])).unwrap();
    let unit_id = node["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "id")
        .unwrap()["value"]
        .as_str()
        .unwrap()
        .to_owned();
    let found = ok(&["find", file, "--id", &unit_id]);
    assert_eq!(found.trim(), "/idm/er[0]/informationUnit[2]");
    ok(&[
        "set-attribute",
        file,
        &format!("id:{unit_id}"),
        "name",
        "pump",
        "-i",
    ]);
    ok(&["duplicate", file, &format!("id:{unit_id}"), "-i"]);
    ok(&[
        "move",
        file,
        "/idm/er/informationUnit[3]",
        "/idm/er",
        "--position",
        "0",
        "-i",
    ]);
    ok(&["add-attribute", file, "/idm/specId", "shortTitle", "-i"]);
    ok(&["remove-attribute", file, "/idm/specId", "shortTitle", "-i"]);
    let validate = run(&["validate", file]);
    assert!(
        validate.status.success(),
        "{}",
        String::from_utf8_lossy(&validate.stdout)
    );
    let list = ok(&["list", file]);
    assert!(list.lines().any(|l| l == "/idm"));
    assert!(ok(&["rule", file, "/idm/specId"]).contains("\"guid\""));

    assert_eq!(
        run(&["find", file, "--name", "nope"]).status.code(),
        Some(1)
    );
    assert_eq!(run(&["find", file]).status.code(), Some(1));
}

#[test]
fn cli_diff_apply_and_undo_round_trip() {
    let temporary = tempdir().unwrap();
    let a = temporary.path().join("a.xml");
    let b = temporary.path().join("b.xml");
    let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
    ok(&["new", "T", "C", "-o", a]);
    ok(&[
        "set-attribute",
        a,
        "/idm/specId",
        "fullTitle",
        "Changed",
        "-o",
        b,
    ]);
    ok(&["add", b, "/idm/er", "informationUnit", "-i"]);
    let edits = temporary.path().join("edits.json");
    std::fs::write(&edits, ok(&["diff", a, b])).unwrap();
    let undo = temporary.path().join("undo.json");
    let applied = temporary.path().join("applied.xml");
    ok(&[
        "apply",
        a,
        edits.to_str().unwrap(),
        "--undo",
        undo.to_str().unwrap(),
        "-o",
        applied.to_str().unwrap(),
    ]);
    let leftover: Value =
        serde_json::from_str(&ok(&["diff", applied.to_str().unwrap(), b])).unwrap();
    assert!(leftover["edits"].as_array().unwrap().is_empty());
    let restored = temporary.path().join("restored.xml");
    ok(&[
        "apply",
        applied.to_str().unwrap(),
        undo.to_str().unwrap(),
        "-o",
        restored.to_str().unwrap(),
    ]);
    let leftover: Value =
        serde_json::from_str(&ok(&["diff", restored.to_str().unwrap(), a])).unwrap();
    assert!(leftover["edits"].as_array().unwrap().is_empty());
}

#[test]
fn cli_reports_json_errors_encodings_and_schema_verification() {
    let temporary = tempdir().unwrap();
    let file = temporary.path().join("m.xml");
    let file = file.to_str().unwrap();
    ok(&[
        "new",
        "Gr\u{fc}\u{df}e",
        "C",
        "-o",
        file,
        "--encoding",
        "iso-8859-1",
    ]);
    let bytes = std::fs::read(file).unwrap();
    assert!(bytes.contains(&0xFC));
    let reread = run(&["inspect", file]);
    assert!(reread.status.success());
    assert!(String::from_utf8_lossy(&reread.stderr).contains("output is UTF-8"));

    let failure = run(&["--json-errors", "get", file, "/idm/missing"]);
    assert_eq!(failure.status.code(), Some(1));
    let stderr = String::from_utf8(failure.stderr).unwrap();
    let error: Value = serde_json::from_str(stderr.lines().last().unwrap()).unwrap();
    assert_eq!(error["code"], "path_not_found");

    let verify = run(&[
        "schema",
        "--verify",
        temporary.path().to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(verify.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(report["checks"].as_array().unwrap().len(), 6);
    assert_eq!(report["checks"][0]["status"], "missing");
}

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

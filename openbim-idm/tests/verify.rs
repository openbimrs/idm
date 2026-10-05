use openbim_idm::{
    SCHEMA_FILES, SchemaFileStatus, SchemaSource, verify_schema_dir, verify_schema_files,
};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn sha(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

#[test]
fn reports_match_mismatch_and_missing_without_exposing_content() {
    let directory = tempdir().unwrap();
    std::fs::write(directory.path().join("specId.xsd"), b"synthetic-a").unwrap();
    std::fs::write(directory.path().join("er.xsd"), b"synthetic-b").unwrap();
    let expected = vec![
        SchemaSource {
            file: "specId.xsd".into(),
            sha256: sha(b"synthetic-a"),
        },
        SchemaSource {
            file: "er.xsd".into(),
            sha256: sha(b"other"),
        },
        SchemaSource {
            file: "idm.xsd".into(),
            sha256: sha(b"x"),
        },
    ];
    let report = verify_schema_files(directory.path(), &expected).unwrap();
    let statuses: Vec<_> = report.checks.iter().map(|c| c.status).collect();
    assert_eq!(
        statuses,
        [
            SchemaFileStatus::Match,
            SchemaFileStatus::Mismatch,
            SchemaFileStatus::Missing
        ]
    );
    assert!(!report.ok());
    assert_eq!(
        report.checks[1].actual_sha256.as_deref(),
        Some(sha(b"synthetic-b").as_str())
    );
    assert!(report.checks[2].actual_sha256.is_none());
}

#[test]
fn catalog_verification_covers_all_six_recognized_files() {
    let directory = tempdir().unwrap();
    let report = verify_schema_dir(directory.path()).unwrap();
    let files: Vec<_> = report.checks.iter().map(|c| c.file.as_str()).collect();
    for name in SCHEMA_FILES {
        assert!(files.contains(&name), "{name}");
    }
    assert!(
        report
            .checks
            .iter()
            .all(|c| c.status == SchemaFileStatus::Missing)
    );
}

#[test]
fn rejects_non_directories_and_unknown_filenames() {
    let directory = tempdir().unwrap();
    let file = directory.path().join("f");
    std::fs::write(&file, b"x").unwrap();
    assert!(verify_schema_dir(&file).is_err());
    let unknown = vec![SchemaSource {
        file: "../evil.xsd".into(),
        sha256: String::new(),
    }];
    assert!(verify_schema_files(directory.path(), &unknown).is_err());
}

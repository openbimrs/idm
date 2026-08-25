use openbim_idm::{SCHEMA_FILES, schema_text};
use std::fs;
use tempfile::tempdir;

#[test]
fn schema_text_requires_a_recognized_name_in_an_explicit_directory() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("idm.xsd"), "synthetic").unwrap();

    assert_eq!(
        schema_text(directory.path(), "idm.xsd").unwrap(),
        "synthetic"
    );
    assert!(schema_text(directory.path(), "../idm.xsd").is_err());
    assert_eq!(SCHEMA_FILES.len(), 6);
}

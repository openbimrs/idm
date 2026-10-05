//! Check a local schema directory against the hashes recorded in the catalog.
//!
//! The catalog-driven validator describes one specific schema revision. This
//! module lets a user confirm that the XSD files they hold are that revision,
//! without bundling, copying or exposing any schema bytes.

use super::*;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaFileStatus {
    Match,
    Mismatch,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaFileCheck {
    pub file: String,
    pub status: SchemaFileStatus,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVerification {
    pub checks: Vec<SchemaFileCheck>,
}

impl SchemaVerification {
    /// True when every recognized file is present and matches.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.checks
            .iter()
            .all(|check| check.status == SchemaFileStatus::Match)
    }
}

/// Compare the six recognized files in `schema_dir` with the catalog hashes.
///
/// Only the allowlisted filenames are read; nothing is downloaded or retained.
pub fn verify_schema_dir(schema_dir: impl AsRef<Path>) -> Result<SchemaVerification> {
    verify_schema_files(schema_dir, &cached_catalog()?.schemas)
}

/// Like [`verify_schema_dir`] against an explicit expectation list.
pub fn verify_schema_files(
    schema_dir: impl AsRef<Path>,
    expected: &[SchemaSource],
) -> Result<SchemaVerification> {
    let directory = schema_dir.as_ref();
    if !directory.is_dir() {
        return Err(Error::Schema(format!(
            "schema directory `{}` is not a directory",
            directory.display()
        )));
    }
    let mut checks = Vec::with_capacity(expected.len());
    for source in expected {
        if !SCHEMA_FILES.contains(&source.file.as_str()) {
            return Err(Error::Schema(format!(
                "unknown schema filename `{}`",
                source.file
            )));
        }
        let path = directory.join(&source.file);
        let (status, actual) = match fs::read(&path) {
            Ok(bytes) => {
                let digest = hex(&Sha256::digest(&bytes));
                let status = if digest.eq_ignore_ascii_case(&source.sha256) {
                    SchemaFileStatus::Match
                } else {
                    SchemaFileStatus::Mismatch
                };
                (status, Some(digest))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (SchemaFileStatus::Missing, None)
            }
            Err(error) => {
                return Err(Error::Schema(format!(
                    "could not read `{}`: {error}",
                    path.display()
                )));
            }
        };
        checks.push(SchemaFileCheck {
            file: source.file.clone(),
            status,
            expected_sha256: source.sha256.clone(),
            actual_sha256: actual,
        });
    }
    Ok(SchemaVerification { checks })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

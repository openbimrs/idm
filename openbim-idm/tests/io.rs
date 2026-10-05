use openbim_idm::{Document, Encoding, Error};
use std::io::Cursor;
use tempfile::tempdir;

fn doc_with_text(text: &str) -> Document {
    let mut document = Document::new_idm("T", "C").unwrap();
    document
        .set_text("/idm/uc/standardProjectPhase/name", "design")
        .unwrap();
    document.append_element("/idm", "note", None).unwrap();
    document.set_text("/idm/note", text).unwrap();
    document
}

#[test]
fn utf8_bom_is_stripped_and_reported() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><idm/>");
    let parsed = Document::parse_bytes(&bytes).unwrap();
    assert!(parsed.had_bom);
    assert_eq!(parsed.encoding, Encoding::Utf8);
    assert!(
        parsed
            .warnings
            .iter()
            .any(|w| w.contains("byte order mark"))
    );
    assert_eq!(parsed.document.root().local_name(), "idm");
}

#[test]
fn round_trips_every_supported_encoding_and_reports_non_utf8_input() {
    let original = doc_with_text("Grüße — € 5");
    for (encoding, lossless) in [
        (Encoding::Utf8, true),
        (Encoding::Utf16Le, true),
        (Encoding::Utf16Be, true),
        (Encoding::Windows1252, true),
    ] {
        let bytes = original.to_bytes_with_encoding(false, encoding).unwrap();
        let parsed = Document::parse_bytes(&bytes).unwrap();
        assert_eq!(parsed.encoding, encoding, "{encoding:?}");
        assert_eq!(parsed.document.text("/idm/note").unwrap(), "Grüße — € 5");
        if encoding != Encoding::Utf8 {
            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|w| w.contains("output is UTF-8")),
                "{encoding:?}"
            );
        }
        assert!(lossless);
    }
}

#[test]
fn latin1_declared_input_decodes_and_unrepresentable_output_is_an_error() {
    let mut bytes = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><idm><n>caf".to_vec();
    bytes.push(0xE9);
    bytes.extend_from_slice(b"</n></idm>");
    let parsed = Document::parse_bytes(&bytes).unwrap();
    assert_eq!(parsed.encoding, Encoding::Latin1);
    assert_eq!(parsed.declared_encoding.as_deref(), Some("ISO-8859-1"));
    assert_eq!(parsed.document.text("/idm/n").unwrap(), "café");
    assert!(
        parsed
            .warnings
            .iter()
            .any(|w| w.contains("output is UTF-8"))
    );

    let euro = doc_with_text("€");
    let error = euro
        .to_bytes_with_encoding(false, Encoding::Latin1)
        .unwrap_err();
    assert_eq!(error.code(), "encoding");
    let written = parsed
        .document
        .to_bytes_with_encoding(false, Encoding::Latin1)
        .unwrap();
    assert!(written.windows(3).any(|w| w == b"caf"));
    assert!(written.contains(&0xE9));
    assert!(String::from_utf8_lossy(&written).contains("ISO-8859-1"));
}

#[test]
fn unsupported_or_invalid_encodings_are_reported() {
    let sjis = b"<?xml version=\"1.0\" encoding=\"Shift_JIS\"?><idm/>";
    assert_eq!(Document::parse_bytes(sjis).unwrap_err().code(), "encoding");
    let invalid = [b"<idm>".as_slice(), &[0xFF, 0xFE, 0xFD], b"</idm>"].concat();
    assert_eq!(
        Document::parse_bytes(&invalid).unwrap_err().code(),
        "encoding"
    );
}

#[test]
fn size_limit_applies_to_bytes_and_streams() {
    let bytes = b"<idm>aaaaaaaaaaaaaaaaaaaa</idm>";
    assert!(matches!(
        Document::parse_bytes_with_limit(bytes, 8),
        Err(Error::InputTooLarge { .. })
    ));
    assert!(matches!(
        Document::from_reader_with_limit(Cursor::new(bytes.to_vec()), 8),
        Err(Error::InputTooLarge { maximum: 8, .. })
    ));
    assert!(Document::from_reader(Cursor::new(bytes.to_vec())).is_ok());
}

#[test]
fn paths_and_streams_round_trip_and_writes_are_atomic() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("doc.xml");
    let original = doc_with_text("hello");
    original.write_path(&path, true).unwrap();
    let parsed = Document::from_path(&path).unwrap();
    assert_eq!(
        parsed.document,
        Document::parse(&original.to_xml(true).unwrap()).unwrap()
    );
    assert!(parsed.warnings.is_empty());

    let mut buffer = Vec::new();
    original.write_to(&mut buffer, false).unwrap();
    assert_eq!(buffer, original.to_bytes(false).unwrap());

    // Unrepresentable output: the existing file must be left intact.
    let before = std::fs::read(&path).unwrap();
    let euro = doc_with_text("€");
    assert!(
        euro.write_path_with_encoding(&path, true, Encoding::Latin1)
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let leftovers: Vec<_> = std::fs::read_dir(directory.path()).unwrap().collect();
    assert_eq!(leftovers.len(), 1, "no temporary file may remain");

    let missing = directory.path().join("nope").join("doc.xml");
    assert_eq!(
        original.write_path(&missing, true).unwrap_err().code(),
        "io"
    );
    assert_eq!(
        Document::from_path(directory.path().join("absent.xml"))
            .unwrap_err()
            .code(),
        "io"
    );
}

use openbim_idm::{Document, Error, ValidationSeverity};

fn skeleton() -> Document {
    Document::new_idm("Title", "IDM-1").expect("skeleton")
}

fn codes(document: &Document) -> Vec<String> {
    document.validate().into_iter().map(|i| i.code).collect()
}

#[test]
fn fresh_skeleton_has_no_issues_including_references() {
    assert!(skeleton().validate().is_empty());
}

#[test]
fn rejects_invalid_attribute_datatypes_with_attribute_and_expectation() {
    let mut document = skeleton();
    document
        .set_attribute("/idm/authoring/changeLog", "changeDateTime", "not-a-date")
        .unwrap();
    let issues = document.validate();
    let issue = issues
        .iter()
        .find(|issue| issue.code == "attribute_datatype")
        .expect("datatype issue");
    assert_eq!(issue.severity, ValidationSeverity::Error);
    assert_eq!(issue.attribute.as_deref(), Some("changeDateTime"));
    assert_eq!(issue.expected.as_ref().unwrap()["type"], "xs:dateTime");
    assert_eq!(issue.path, "/idm/authoring[0]/changeLog[0]");
}

#[test]
fn rejects_bad_boolean_integer_and_ncname_values() {
    let mut document = skeleton();
    document
        .set_attribute("/idm/authoring/changeLog", "changedBy", "1 bad")
        .unwrap();
    assert!(codes(&document).contains(&"attribute_datatype".to_owned()));

    let mut document = skeleton();
    document
        .set_attribute("/idm/er/informationUnit", "isMandatory", "maybe")
        .unwrap();
    let issue = document
        .validate()
        .into_iter()
        .find(|issue| issue.code == "attribute_datatype")
        .expect("boolean issue");
    assert_eq!(issue.expected.unwrap()["type"], "xs:boolean");
}

#[test]
fn rejects_element_enumeration_violation() {
    let mut document = skeleton();
    document
        .set_text("/idm/uc/standardProjectPhase/name", "bogus")
        .unwrap();
    let issue = document
        .validate()
        .into_iter()
        .find(|issue| issue.code == "element_enumeration")
        .expect("enum issue");
    assert!(issue.expected.unwrap()["enum"].as_array().unwrap().len() >= 6);
}

#[test]
fn flags_unknown_attributes_as_warnings_but_ignores_namespaced_extensions() {
    let mut document = skeleton();
    document.set_attribute("/idm/specId", "extra", "1").unwrap();
    document
        .set_attribute("/idm/specId", "vendor:note", "x")
        .unwrap();
    let issues = document.validate();
    let extras: Vec<_> = issues
        .iter()
        .filter(|issue| issue.code == "extension_attribute")
        .collect();
    assert_eq!(extras.len(), 1);
    assert_eq!(extras[0].severity, ValidationSeverity::Warning);
    assert_eq!(extras[0].attribute.as_deref(), Some("extra"));
}

#[test]
fn flags_text_in_element_only_content() {
    let mut document = skeleton();
    document
        .element_mut("/idm/uc")
        .unwrap()
        .children
        .insert(0, openbim_idm::Node::Text("stray".into()));
    assert!(codes(&document).contains(&"unexpected_text".to_owned()));
}

#[test]
fn flags_duplicate_ids_and_dangling_change_log_references() {
    let mut document = skeleton();
    document
        .set_attribute("/idm/authoring/changeLog", "changedBy", "nobody")
        .unwrap();
    let issue = document
        .validate()
        .into_iter()
        .find(|issue| issue.code == "dangling_reference")
        .expect("dangling reference");
    assert_eq!(issue.attribute.as_deref(), Some("changedBy"));

    let mut document = skeleton();
    let id = document.attribute("/idm/authoring/author", "id").unwrap();
    document
        .set_attribute("/idm/authoring/changeLog", "id", &id)
        .unwrap();
    assert!(codes(&document).contains(&"duplicate_id".to_owned()));
}

#[test]
fn set_text_rejects_element_only_content_but_allows_extensions() {
    let mut document = skeleton();
    let error = document.set_text("/idm/uc", "text").unwrap_err();
    assert!(matches!(error, Error::Content(_)));
    assert_eq!(error.code(), "content_model");

    document.append_element("/idm", "vendor", None).unwrap();
    document.set_text("/idm/vendor", "ok").unwrap();
    document.set_text_unchecked("/idm/uc", "forced").unwrap();
}

#[test]
fn errors_expose_stable_codes_and_serialize() {
    let document = skeleton();
    let error = document.element("/idm/missing").unwrap_err();
    assert_eq!(error.code(), "path_not_found");
    assert_eq!(error.clone(), error);
    let value = serde_json::to_value(&error).unwrap();
    assert_eq!(value["code"], "path_not_found");
    assert!(value["message"].as_str().unwrap().contains("/idm/missing"));
    assert_eq!(Document::parse("<a>").unwrap_err().code(), "invalid_xml");
    assert_eq!(
        Document::parse_with_limit("<a/>", 1).unwrap_err().code(),
        "input_too_large"
    );
}

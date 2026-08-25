use openbim_idm::{Document, ValidationSeverity, schema_catalog};
use pretty_assertions::assert_eq;

const FIXTURE: &str = include_str!("fixtures/recursive-extension.xml");

#[test]
fn preserves_recursive_idm_and_unknown_xml_losslessly() {
    let document = Document::parse(FIXTURE).expect("fixture parses");
    assert_eq!(document.root().local_name(), "idm");
    assert_eq!(document.root().namespace_uri(), None);
    assert_eq!(document.count("uc"), 2);
    assert_eq!(document.count("businessContextMap"), 2);
    assert_eq!(document.count("er"), 2);

    let xml = document.to_xml(false).expect("serializes");
    assert!(xml.contains("preserve this comment"));
    assert!(xml.contains("<![CDATA[future <content>]]>"));
    assert!(xml.contains("vendor:Extension"));
    assert!(xml.contains("vendor:flag=\"kept\""));
    assert!(xml.contains("<?idm-editor keep-me?>"));

    let reparsed = Document::parse(&xml).expect("serialized XML reparses");
    assert_eq!(reparsed.to_value(), document.to_value());
}

#[test]
fn creates_a_semantically_complete_schema_ordered_idm() {
    let document = Document::new_idm("Coordination", "IDM-001").expect("creates IDM");
    let root_children = document.child_element_names("/idm").unwrap();
    assert_eq!(root_children, ["specId", "authoring", "uc", "er"]);
    assert_eq!(
        document.attribute("/idm/specId[0]", "fullTitle").unwrap(),
        "Coordination"
    );
    assert_eq!(
        document.attribute("/idm/specId[0]", "idmCode").unwrap(),
        "IDM-001"
    );
    assert!(
        document
            .validate()
            .iter()
            .all(|issue| issue.severity != ValidationSeverity::Error)
    );
}

#[test]
fn schema_catalog_covers_all_official_declarations_and_cardinalities() {
    let catalog = schema_catalog().expect("generated catalog parses");
    assert_eq!(catalog.global_elements.len(), 17);
    assert_eq!(catalog.element_names.len(), 57);
    assert_eq!(catalog.attribute_names.len(), 38);
    assert_eq!(catalog.enum_values.len(), 6);
    assert_eq!(catalog.recursive_edges.len(), 6);

    let idm = catalog.element("idm").unwrap();
    assert_eq!(idm.child("uc").unwrap().min_occurs, 1);
    assert_eq!(idm.child("uc").unwrap().max_occurs, Some(1));
    assert_eq!(idm.child("businessContextMap").unwrap().min_occurs, 0);
    assert_eq!(idm.child("businessContextMap").unwrap().max_occurs, None);
    assert_eq!(idm.child("er").unwrap().max_occurs, Some(1));
    assert!(idm.child("subIdm").unwrap().recursive);

    assert!(
        catalog
            .element("uc")
            .unwrap()
            .child("subUc")
            .unwrap()
            .recursive
    );
    assert!(
        catalog
            .element("er")
            .unwrap()
            .child("subEr")
            .unwrap()
            .recursive
    );
    assert!(
        catalog
            .element("informationUnit")
            .unwrap()
            .child("subInformationUnit")
            .unwrap()
            .recursive
    );
    assert!(
        catalog
            .element("businessContextMap")
            .unwrap()
            .child("subBusinessContextMap")
            .unwrap()
            .recursive
    );
    assert!(
        catalog
            .element("pm")
            .unwrap()
            .child("subPm")
            .unwrap()
            .recursive
    );

    let spec_id = catalog.element("specId").unwrap();
    assert!(spec_id.attribute("guid").unwrap().required);
    assert!(spec_id.attribute("fullTitle").unwrap().required);
    assert_eq!(
        spec_id.attribute("guid").unwrap().pattern.as_deref(),
        Some("[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}")
    );
}

#[test]
fn schema_aware_actions_enforce_cardinality_and_build_recursive_skeletons() {
    let mut document = Document::new_idm("Coordination", "IDM-001").unwrap();
    let actions = document.allowed_children("/idm/uc[0]").unwrap();
    let sub_uc = actions
        .iter()
        .find(|action| action.name == "subUc")
        .unwrap();
    assert!(sub_uc.can_add);
    assert_eq!(sub_uc.label_key, "schema.element.subUc");

    let path = document.append_schema_child("/idm/uc[0]", "subUc").unwrap();
    assert_eq!(path, "/idm/uc[0]/subUc[0]");
    assert_eq!(document.child_element_names(&path).unwrap(), ["uc"]);
    assert!(document.element(&format!("{path}/uc[0]/specId[0]")).is_ok());

    let error = document.remove_schema_node("/idm/uc[0]").unwrap_err();
    assert!(error.to_string().contains("minimum cardinality"));
    let error = document.remove_schema_node("/idm/er[0]").unwrap_err();
    assert!(error.to_string().contains("ISO 29481-3"));
    document
        .remove_schema_node(&path)
        .expect("optional recursive child can be removed");
}

#[test]
fn enumerates_element_paths_and_reorders_same_name_schema_siblings() {
    let mut document = Document::new_idm("Coordination", "IDM-001").unwrap();
    let log_path = "/idm/authoring[0]/changeLog[0]";
    for label in ["A", "B", "C"] {
        let path = document.append_schema_child(log_path, "change").unwrap();
        document
            .set_attribute(&path, "changedElement", label)
            .unwrap();
    }

    assert_eq!(
        document.element_paths("change"),
        [
            "/idm/authoring[0]/changeLog[0]/change[0]",
            "/idm/authoring[0]/changeLog[0]/change[1]",
            "/idm/authoring[0]/changeLog[0]/change[2]",
        ]
    );

    let moved_path = document
        .move_schema_node(
            "/idm/authoring[0]/changeLog[0]/change[0]",
            "/idm/authoring[0]/changeLog[0]/change[2]",
            true,
        )
        .unwrap();
    assert_eq!(moved_path, "/idm/authoring[0]/changeLog[0]/change[2]");
    assert_eq!(
        [0, 1, 2].map(|index| {
            document
                .attribute(
                    &format!("/idm/authoring[0]/changeLog[0]/change[{index}]"),
                    "changedElement",
                )
                .unwrap()
        }),
        ["B", "C", "A"]
    );

    let error = document
        .move_schema_node(
            "/idm/authoring[0]/changeLog[0]/change[0]",
            "/idm/authoring[0]/author[0]",
            false,
        )
        .unwrap_err();
    assert!(error.to_string().contains("same-name siblings"));
}

#[test]
fn preserves_predefined_and_numeric_character_references() {
    let source = "<idm>a&amp;b&lt;&gt;&apos;&quot;&#65;&#x41;</idm>";
    let document = Document::parse(source).expect("valid XML references parse");

    assert_eq!(document.text("/idm").unwrap(), "a&b<>'\"AA");
    assert_eq!(
        document.to_xml(false).unwrap(),
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>{source}")
    );
}

#[test]
fn rejects_undefined_and_invalid_general_entity_references() {
    for source in [
        "<idm>&undefined;</idm>",
        "<idm>&#0;</idm>",
        "<idm>&#xD800;</idm>",
        "<idm>&#x110000;</idm>",
    ] {
        assert!(Document::parse(source).is_err(), "accepted {source}");
    }
}

#[test]
fn rejects_malformed_oversized_doctype_and_excessively_deep_xml() {
    assert!(Document::parse("<broken>").is_err());
    assert!(Document::parse("<!DOCTYPE idm><idm/>").is_err());
    assert!(Document::parse(&"x".repeat(openbim_idm::DEFAULT_MAX_XML_BYTES + 1)).is_err());
    let deep = format!(
        "{}{}",
        "<subIdm>".repeat(openbim_idm::DEFAULT_MAX_XML_DEPTH + 1),
        "</subIdm>".repeat(openbim_idm::DEFAULT_MAX_XML_DEPTH + 1)
    );
    assert!(Document::parse(&deep).is_err());
}

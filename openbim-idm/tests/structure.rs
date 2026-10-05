use openbim_idm::{Document, Error};

fn doc_with_units(extra: usize) -> Document {
    let mut document = Document::new_idm("T", "C").unwrap();
    for _ in 0..extra {
        document
            .append_schema_child("/idm/er", "informationUnit")
            .unwrap();
    }
    document
}

fn unit_ids(document: &Document) -> Vec<String> {
    document
        .element_paths("informationUnit")
        .iter()
        .map(|path| document.attribute(path, "id").unwrap())
        .collect()
}

#[test]
fn node_info_and_schema_rule_describe_a_selected_element() {
    let document = doc_with_units(0);
    let info = document.node_info("/idm/specId").unwrap();
    assert_eq!(info.name, "specId");
    assert_eq!(info.handle.as_deref(), Some("specId"));
    assert!(info.attributes.iter().all(|a| a.declared));
    let rule = document.schema_rule("/idm/specId").unwrap();
    assert!(rule.attribute("guid").unwrap().required);

    let root = document.node_info("/idm").unwrap();
    let names: Vec<_> = root.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["specId", "authoring", "uc", "er"]);
    assert_eq!(root.children[2].path, "/idm/uc[0]");
}

#[test]
fn extension_elements_are_reported_as_undeclared() {
    let mut document = doc_with_units(0);
    document.append_element("/idm", "vendor", None).unwrap();
    let info = document.node_info("/idm/vendor").unwrap();
    assert_eq!(info.handle, None);
    assert!(matches!(
        document.schema_rule("/idm/vendor"),
        Err(Error::Schema(_))
    ));
    let root = document.node_info("/idm").unwrap();
    assert!(
        root.children
            .iter()
            .any(|c| c.name == "vendor" && !c.declared)
    );
}

#[test]
fn attribute_slots_add_and_remove_respect_the_schema() {
    let mut document = doc_with_units(0);
    let slots = document.attribute_slots("/idm/specId").unwrap();
    let short = slots.iter().find(|s| s.rule.name == "shortTitle").unwrap();
    assert!(!short.present && !short.rule.required);

    document
        .add_schema_attribute("/idm/specId", "shortTitle")
        .unwrap();
    assert!(document.attribute("/idm/specId", "shortTitle").is_ok());
    assert!(matches!(
        document.add_schema_attribute("/idm/specId", "shortTitle"),
        Err(Error::Content(_))
    ));
    assert!(matches!(
        document.add_schema_attribute("/idm/specId", "nope"),
        Err(Error::Schema(_))
    ));
    document
        .remove_attribute("/idm/specId", "shortTitle")
        .unwrap();
    assert!(document.attribute("/idm/specId", "shortTitle").is_err());
    assert!(matches!(
        document.remove_attribute("/idm/specId", "guid"),
        Err(Error::Content(_))
    ));
    assert!(matches!(
        document.remove_attribute("/idm/specId", "shortTitle"),
        Err(Error::PathNotFound(_))
    ));
}

#[test]
fn locators_survive_structural_edits() {
    let mut document = doc_with_units(1);
    let ids = unit_ids(&document);
    let target = format!("id:{}", ids[1]);
    assert_eq!(
        document.path_of(&target).unwrap(),
        "/idm/er[0]/informationUnit[1]"
    );
    document
        .insert_schema_child("/idm/er", "informationUnit", 0)
        .unwrap();
    assert_eq!(
        document.path_of(&target).unwrap(),
        "/idm/er[0]/informationUnit[2]"
    );
    document.set_attribute(&target, "name", "renamed").unwrap();
    assert_eq!(document.attribute(&target, "name").unwrap(), "renamed");

    let guid = document.attribute("/idm/specId", "guid").unwrap();
    assert_eq!(
        document.text(&format!("guid:{guid}")).unwrap(),
        document.text("/idm/specId").unwrap()
    );
    assert!(matches!(
        document.path_of("guid:00000000-0000-0000-0000-000000000000"),
        Err(Error::PathNotFound(_))
    ));
    assert!(matches!(
        document.path_of("name:x"),
        Err(Error::InvalidPath(_))
    ));
}

#[test]
fn insert_at_position_places_the_new_child_and_checks_range() {
    let mut document = doc_with_units(2);
    let before = unit_ids(&document);
    let path = document
        .insert_schema_child("/idm/er", "informationUnit", 1)
        .unwrap();
    assert_eq!(path, "/idm/er[0]/informationUnit[1]");
    let after = unit_ids(&document);
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(after[0], before[0]);
    assert_eq!(after[2], before[1]);
    assert!(matches!(
        document.insert_schema_child("/idm/er", "informationUnit", 99),
        Err(Error::InvalidPath(_))
    ));
    assert!(document.validate().iter().all(|i| i.code != "schema_order"));
}

#[test]
fn duplicate_regenerates_identity_and_stays_valid() {
    let mut document = doc_with_units(0);
    let copy = document
        .duplicate_schema_node("/idm/er/informationUnit[0]")
        .unwrap();
    assert_eq!(copy, "/idm/er[0]/informationUnit[1]");
    let ids = unit_ids(&document);
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
    assert!(document.validate().is_empty(), "{:?}", document.validate());

    // A whole use case carries guids and ids; the copy must not collide.
    let mut document = Document::new_idm("T", "C").unwrap();
    document.append_schema_child("/idm/er", "subEr").unwrap();
    let copy = document.duplicate_schema_node("/idm/er/subEr[0]").unwrap();
    assert_eq!(copy, "/idm/er[0]/subEr[1]");
    let issues = document.validate();
    assert!(
        issues
            .iter()
            .all(|i| i.code != "duplicate_guid" && i.code != "duplicate_id"),
        "{issues:?}"
    );
}

#[test]
fn duplicate_refuses_to_exceed_maximum_cardinality() {
    let mut document = doc_with_units(0);
    assert!(matches!(
        document.duplicate_schema_node("/idm/uc"),
        Err(Error::Cardinality(_))
    ));
    assert!(matches!(
        document.duplicate_schema_node("/idm"),
        Err(Error::InvalidPath(_))
    ));
}

#[test]
fn paste_checks_membership_and_remaps_identity() {
    let source = doc_with_units(0);
    let unit = source.element("/idm/er/informationUnit").unwrap().clone();
    let mut target = doc_with_units(0);
    // Same ids as the existing unit's after a clone of the document.
    let mut pasted = unit.clone();
    pasted.set_attribute(
        "id",
        &target.attribute("/idm/er/informationUnit", "id").unwrap(),
    );
    let path = target
        .paste_schema_node("/idm/er", pasted, Some(0))
        .unwrap();
    assert_eq!(path, "/idm/er[0]/informationUnit[0]");
    assert!(target.validate().iter().all(|i| i.code != "duplicate_id"));

    assert!(matches!(
        target.paste_schema_node("/idm", unit, None),
        Err(Error::Schema(_))
    ));
}

#[test]
fn reparent_moves_between_parents_and_rolls_back_on_failure() {
    let mut document = doc_with_units(2);
    document
        .append_schema_child("/idm/er/informationUnit[0]", "subInformationUnit")
        .unwrap();
    let moving = unit_ids(&document).pop().unwrap();
    let new_path = document
        .reparent_schema_node(
            &format!("id:{moving}"),
            "/idm/er/informationUnit[0]/subInformationUnit[0]",
            None,
        )
        .unwrap();
    assert!(new_path.contains("subInformationUnit[0]/informationUnit"));
    assert_eq!(document.attribute(&new_path, "id").unwrap(), moving);
    assert_eq!(document.element_paths("informationUnit").len(), 4);
    assert!(document.validate().is_empty(), "{:?}", document.validate());

    let snapshot = document.clone();
    // Cannot move below itself.
    assert!(matches!(
        document.reparent_schema_node(
            "/idm/er/informationUnit[0]",
            "/idm/er/informationUnit[0]/subInformationUnit[0]",
            None
        ),
        Err(Error::Content(_))
    ));
    // Different declaration.
    assert!(
        document
            .reparent_schema_node("/idm/er/informationUnit[0]", "/idm/uc", None)
            .is_err()
    );
    assert_eq!(document, snapshot);
}

#[test]
fn reparent_within_the_same_parent_reorders() {
    let mut document = doc_with_units(2);
    let ids = unit_ids(&document);
    let new_path = document
        .reparent_schema_node("/idm/er/informationUnit[2]", "/idm/er", Some(0))
        .unwrap();
    assert_eq!(new_path, "/idm/er[0]/informationUnit[0]");
    let after = unit_ids(&document);
    assert_eq!(after, [ids[2].clone(), ids[0].clone(), ids[1].clone()]);
}

#[test]
fn schema_rule_follows_recursive_wrappers() {
    let mut document = Document::new_idm("T", "C").unwrap();
    document.append_schema_child("/idm/uc", "subUc").unwrap();
    let nested = document.schema_rule("/idm/uc/subUc/uc").unwrap();
    assert_eq!(nested.name, "uc");
    assert_eq!(document.schema_rule("/idm/uc").unwrap(), nested);
    assert_eq!(document.schema_rule("/idm/uc/subUc").unwrap().name, "subUc");

    document.append_schema_child("/idm", "subIdm").unwrap();
    assert_eq!(document.schema_rule("/idm/subIdm/idm").unwrap().name, "idm");
    assert!(
        document
            .node_info("/idm/subIdm/idm/uc")
            .unwrap()
            .handle
            .is_some()
    );
}

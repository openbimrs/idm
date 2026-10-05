use openbim_idm::{Document, EDIT_FORMAT_VERSION, Edit, EditBatch, Error};

fn base() -> Document {
    let mut document = Document::new_idm("T", "C").unwrap();
    document
        .append_schema_child("/idm/er", "informationUnit")
        .unwrap();
    document
        .append_schema_child("/idm/er", "informationUnit")
        .unwrap();
    document
        .append_schema_child("/idm/er/informationUnit[0]", "subInformationUnit")
        .unwrap();
    document
}

fn roundtrip(edit: Edit) {
    let mut document = base();
    let original = document.clone();
    let inverse = document.apply(&edit).expect("edit applies");
    assert_ne!(
        document, original,
        "edit should change the document: {edit:?}"
    );
    let redo = document.apply(&inverse).expect("inverse applies");
    assert_eq!(
        document, original,
        "inverse must restore the original: {edit:?}"
    );
    document.apply(&redo).expect("redo applies");
    assert_ne!(document, original);
}

#[test]
fn every_edit_kind_is_reversible() {
    roundtrip(Edit::SetAttribute {
        path: "/idm/specId".into(),
        name: "fullTitle".into(),
        value: "Renamed".into(),
        index: None,
    });
    roundtrip(Edit::SetAttribute {
        path: "/idm/specId".into(),
        name: "shortTitle".into(),
        value: "New".into(),
        index: None,
    });
    roundtrip(Edit::RemoveAttribute {
        path: "/idm/specId".into(),
        name: "documentStatus".into(),
        force: true,
    });
    roundtrip(Edit::SetText {
        path: "/idm/uc/standardProjectPhase/name".into(),
        value: "design".into(),
    });
    roundtrip(Edit::RemoveChild {
        path: "/idm/er/informationUnit[0]".into(),
    });
    roundtrip(Edit::RemoveChild {
        path: "/idm/er/informationUnit[1]".into(),
    });
    let other = base();
    let node = other.element("/idm/er/informationUnit[1]").unwrap().clone();
    roundtrip(Edit::InsertChild {
        parent: "/idm/er".into(),
        node: node.clone(),
        position: Some(0),
        at: None,
    });
    roundtrip(Edit::InsertChild {
        parent: "/idm/er".into(),
        node,
        position: None,
        at: Some(3),
    });
    roundtrip(Edit::Move {
        path: "/idm/er/informationUnit[1]".into(),
        new_parent: "/idm/er".into(),
        position: Some(0),
    });
    roundtrip(Edit::Move {
        path: "/idm/er/informationUnit[1]".into(),
        new_parent: "/idm/er/informationUnit[0]/subInformationUnit[0]".into(),
        position: None,
    });
    roundtrip(Edit::SetChildren {
        path: "/idm/er/informationUnit[0]/subInformationUnit[0]".into(),
        children: vec![],
    });
}

#[test]
fn failed_edits_leave_the_document_unchanged() {
    let mut document = base();
    let snapshot = document.clone();
    for edit in [
        Edit::SetText {
            path: "/idm/uc".into(),
            value: "x".into(),
        },
        Edit::RemoveAttribute {
            path: "/idm/specId".into(),
            name: "guid".into(),
            force: false,
        },
        Edit::RemoveChild {
            path: "/idm/uc".into(),
        },
        Edit::RemoveChild {
            path: "/idm/er/missing[0]".into(),
        },
        Edit::Move {
            path: "/idm/er/informationUnit[0]".into(),
            new_parent: "/idm/uc".into(),
            position: None,
        },
    ] {
        assert!(document.apply(&edit).is_err(), "{edit:?}");
        assert_eq!(document, snapshot, "{edit:?}");
    }
}

#[test]
fn batches_are_atomic_versioned_and_invertible() {
    let mut document = base();
    let original = document.clone();
    let batch = EditBatch::new(vec![
        Edit::SetAttribute {
            path: "/idm/specId".into(),
            name: "fullTitle".into(),
            value: "A".into(),
            index: None,
        },
        Edit::RemoveChild {
            path: "/idm/er/informationUnit[1]".into(),
        },
    ]);
    let inverse = document.apply_batch(&batch).unwrap();
    assert_ne!(document, original);
    document.apply_batch(&inverse).unwrap();
    assert_eq!(document, original);

    let failing = EditBatch::new(vec![
        Edit::SetAttribute {
            path: "/idm/specId".into(),
            name: "fullTitle".into(),
            value: "B".into(),
            index: None,
        },
        Edit::RemoveChild {
            path: "/idm/uc".into(),
        },
    ]);
    assert!(document.apply_batch(&failing).is_err());
    assert_eq!(document, original);

    let wrong_version = EditBatch {
        version: EDIT_FORMAT_VERSION + 1,
        edits: vec![],
    };
    assert!(matches!(
        document.apply_batch(&wrong_version),
        Err(Error::Json(_))
    ));
}

#[test]
fn edit_json_is_tagged_and_versioned() {
    let batch = EditBatch::new(vec![Edit::SetText {
        path: "/idm/uc/standardProjectPhase/name".into(),
        value: "design".into(),
    }]);
    let value = serde_json::to_value(&batch).unwrap();
    assert_eq!(value["version"], 1);
    assert_eq!(value["edits"][0]["op"], "set_text");
    let back: EditBatch = serde_json::from_value(value).unwrap();
    assert_eq!(back, batch);
}

#[test]
fn edits_accept_guid_and_id_locators() {
    let mut document = base();
    let id = document
        .attribute("/idm/er/informationUnit[1]", "id")
        .unwrap();
    document
        .apply(&Edit::SetAttribute {
            path: format!("id:{id}"),
            name: "name".into(),
            value: "x".into(),
            index: None,
        })
        .unwrap();
    assert_eq!(
        document
            .attribute("/idm/er/informationUnit[1]", "name")
            .unwrap(),
        "x"
    );
}

#[test]
fn diff_produces_edits_that_transform_one_document_into_the_other() {
    let a = base();
    let mut b = a.clone();
    b.set_attribute("/idm/specId", "fullTitle", "Changed")
        .unwrap();
    b.add_schema_attribute("/idm/specId", "shortTitle").unwrap();
    b.remove_attribute("/idm/er/informationUnit[0]", "name")
        .ok();
    b.set_text("/idm/uc/standardProjectPhase/name", "brief")
        .unwrap();
    b.append_schema_child("/idm/er", "informationUnit").unwrap();
    b.remove_schema_node("/idm/er/informationUnit[0]/subInformationUnit[0]")
        .unwrap();
    b.append_schema_child("/idm/uc", "scopeKeyword").unwrap();

    let batch = a.diff(&b).unwrap();
    assert!(!batch.edits.is_empty());
    let mut applied = a.clone();
    applied.apply_batch(&batch).unwrap();
    assert_eq!(applied.to_xml(true).unwrap(), b.to_xml(true).unwrap());

    assert!(a.diff(&a).unwrap().edits.is_empty());
}

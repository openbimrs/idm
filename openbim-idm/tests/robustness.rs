//! Stability guarantees for downstream use: bounded depth everywhere, no stack
//! exhaustion at the limit, and serialization that always reads back.

use openbim_idm::{DEFAULT_MAX_XML_DEPTH, Document, Edit, EditBatch, Error, Node};

/// `<idm>` plus `levels` nested `<x>` elements.
fn nested(levels: usize) -> String {
    let mut xml = String::from("<idm>");
    for _ in 0..levels {
        xml.push_str("<x a=\"1\">");
    }
    for _ in 0..levels {
        xml.push_str("</x>");
    }
    xml.push_str("</idm>");
    xml
}

#[test]
fn documents_at_the_depth_limit_work_on_a_default_thread_stack() {
    // std::thread's default 2 MiB stack, in a debug build: the worst case a
    // downstream test suite or async runtime worker will see.
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(|| {
            let document = Document::parse(&nested(DEFAULT_MAX_XML_DEPTH - 1)).unwrap();
            let _ = document.validate();
            let xml = document.to_xml(true).unwrap();
            assert_eq!(
                Document::parse(&xml).unwrap(),
                Document::parse(&xml).unwrap()
            );
            let json = serde_json::to_string(&document.to_value()).unwrap();
            let back = Document::from_json_str(&json).unwrap();
            assert_eq!(back, document);
            assert_eq!(
                Document::from_value(&document.to_value()).unwrap(),
                document
            );
            assert!(document.diff(&document.clone()).unwrap().edits.is_empty());
        })
        .unwrap()
        .join()
        .expect("no stack overflow at the depth limit");
}

#[test]
fn deeper_input_is_rejected_on_every_entry_point() {
    let too_deep = nested(DEFAULT_MAX_XML_DEPTH + 1);
    assert!(matches!(
        Document::parse(&too_deep),
        Err(Error::MaxDepthExceeded { .. })
    ));
    assert!(matches!(
        Document::parse_bytes(too_deep.as_bytes()),
        Err(Error::MaxDepthExceeded { .. })
    ));

    // JSON nested far beyond the limit is refused before any recursion.
    let json = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    assert!(matches!(
        Document::from_json_str(&json),
        Err(Error::MaxDepthExceeded { .. })
    ));
    assert!(matches!(
        EditBatch::from_json_str(&json),
        Err(Error::MaxDepthExceeded { .. })
    ));
}

#[test]
fn deep_subtrees_round_trip_through_json_edits() {
    // A subtree deeper than serde_json's default recursion limit (128).
    let source = Document::parse(&nested(120)).unwrap();
    let subtree = source.element("/idm/x").unwrap().clone();
    let mut target = Document::parse("<idm/>").unwrap();
    let edit = Edit::InsertChild {
        parent: "/idm".into(),
        node: subtree,
        position: None,
        at: None,
    };
    let text = serde_json::to_string(&EditBatch::new(vec![edit])).unwrap();
    let batch = EditBatch::from_json_str(&text).unwrap();
    target.apply_batch(&batch).unwrap();
    assert_eq!(target, source);
}

#[test]
fn malformed_json_and_trailing_data_are_errors_not_panics() {
    for text in [
        "",
        "{",
        "[1,",
        "{\"version\":1,\"edits\":[]} trailing",
        "\"\\",
    ] {
        assert!(
            matches!(EditBatch::from_json_str(text), Err(Error::Json(_))),
            "{text:?}"
        );
    }
    assert!(matches!(
        Document::from_json_str(r#"{"prolog":[],"root":{"qualified_name":1},"epilog":[]}"#),
        Err(Error::Json(_))
    ));
}

#[test]
fn serialization_never_produces_unreadable_xml() {
    let mut document = Document::new_idm("T", "C").unwrap();
    // Setters refuse characters XML 1.0 cannot carry.
    assert!(matches!(
        document.set_attribute("/idm/specId", "fullTitle", "bad\u{1}"),
        Err(Error::Content(_))
    ));
    assert!(matches!(
        document.set_text_unchecked("/idm/uc/standardProjectPhase/name", "\u{FFFE}"),
        Err(Error::Content(_))
    ));

    // Low-level edits that bypass the setters are caught at serialization.
    for broken in [
        Node::CData("a]]>b".into()),
        Node::Comment("a--b".into()),
        Node::ProcessingInstruction("pi ?> x".into()),
        Node::Text("bell\u{7}".into()),
    ] {
        let mut copy = document.clone();
        copy.element_mut("/idm/uc")
            .unwrap()
            .children
            .push(broken.clone());
        assert!(
            matches!(copy.to_xml(false), Err(Error::Write(_))),
            "{broken:?}"
        );
    }
    let mut copy = document.clone();
    copy.append_element("/idm", "bad name", None).unwrap();
    assert!(matches!(copy.to_xml(false), Err(Error::Write(_))));

    // Characters that are legal round-trip unchanged, including whitespace
    // that readers would otherwise normalize.
    document
        .set_text_unchecked("/idm/uc/standardProjectPhase/name", "a\r\nb\rc\td\n")
        .unwrap();
    let back = Document::parse(&document.to_xml(true).unwrap()).unwrap();
    assert_eq!(
        back.text("/idm/uc/standardProjectPhase/name").unwrap(),
        "a\r\nb\rc\td\n"
    );
    document
        .set_attribute(
            "/idm/specId",
            "fullTitle",
            "A & <B> \"q\" 'x' \u{1F600}\t\n\r end",
        )
        .unwrap();
    let back = Document::parse(&document.to_xml(false).unwrap()).unwrap();
    assert_eq!(
        back.attribute("/idm/specId", "fullTitle").unwrap(),
        "A & <B> \"q\" 'x' \u{1F600}\t\n\r end"
    );
}

#[test]
fn paths_use_the_actual_root_name_for_generic_xml() {
    let document = Document::parse("<foo><bar/><bar/></foo>").unwrap();
    assert_eq!(
        document.element_paths("bar"),
        ["/foo/bar[0]", "/foo/bar[1]"]
    );
    for path in document.element_paths("bar") {
        assert!(document.element(&path).is_ok(), "{path}");
    }
}

/// Deterministic pseudo-random generator (no extra dependency).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % bound
    }

    fn text(&mut self) -> String {
        const ALPHABET: &[char] = &[
            'a',
            'Z',
            '0',
            ' ',
            '\t',
            '\n',
            '\r',
            '&',
            '<',
            '>',
            '"',
            '\'',
            ']',
            '-',
            '?',
            'é',
            '€',
            '\u{1F600}',
            ';',
            '#',
        ];
        let length = 1 + self.next(12);
        (0..length)
            .map(|_| ALPHABET[self.next(ALPHABET.len())])
            .collect()
    }
}

fn random_element(rng: &mut Lcg, depth: usize) -> openbim_idm::Element {
    let names = ["a", "b", "x:c", "d-e", "f.g"];
    let mut element = openbim_idm::Element::new(names[rng.next(names.len())], None);
    if element.prefix.is_some() {
        element.set_attribute("xmlns:x", "urn:example");
        element.namespace = Some("urn:example".into());
    }
    for index in 0..rng.next(3) {
        element.set_attribute(&format!("at{index}"), &rng.text());
    }
    let mut previous_was_text = false;
    for _ in 0..rng.next(if depth > 6 { 1 } else { 5 }) {
        let node = match rng.next(5) {
            0 if !previous_was_text => Node::Text(rng.text()),
            1 => Node::Comment(rng.text().replace('-', "_")),
            2 => Node::CData(rng.text().replace(']', ")")),
            _ => Node::Element(random_element(rng, depth + 1)),
        };
        previous_was_text = matches!(node, Node::Text(_));
        element.children.push(node);
    }
    element
}

#[test]
fn random_trees_round_trip_through_xml_bytes_and_json() {
    let mut rng = Lcg(0x1d_5eed);
    for _ in 0..300 {
        let mut root = random_element(&mut rng, 0);
        root.qualified_name = "idm".into();
        root.local_name = "idm".into();
        root.prefix = None;
        root.namespace = None;
        root.attributes
            .retain(|a| !a.qualified_name.starts_with("xmlns"));
        let tree = serde_json::json!({ "prolog": [], "root": root, "epilog": [] });
        let document = Document::from_value(&tree).expect("generated tree is valid");
        for pretty in [false, true] {
            let xml = document.to_xml(pretty).unwrap();
            let reparsed = Document::parse(&xml).unwrap();
            if !pretty {
                assert_eq!(reparsed, document, "{xml}");
            }
            let again = reparsed.to_xml(pretty).unwrap();
            assert_eq!(Document::parse(&again).unwrap(), reparsed, "{again}");
        }
        let bytes = document
            .to_bytes_with_encoding(false, openbim_idm::Encoding::Utf16Le)
            .unwrap();
        assert_eq!(Document::parse_bytes(&bytes).unwrap().document, document);
        let json = serde_json::to_string(&document.to_value()).unwrap();
        assert_eq!(Document::from_json_str(&json).unwrap(), document);
    }
}

#[test]
fn api_edited_documents_equal_their_reloaded_form() {
    let mut document = Document::new_idm("Tom & Jerry <draft>", "C").unwrap();
    document
        .set_text("/idm/uc/standardProjectPhase/name", "a & b < c ]]> d\r\n")
        .unwrap();
    document.set_attribute("/idm", "xml:lang", "de").unwrap();
    document.set_attribute("/idm", "xmlns:v", "urn:v").unwrap();
    let reloaded = Document::parse(&document.to_xml(false).unwrap()).unwrap();
    assert_eq!(reloaded, document);
    assert_eq!(
        Document::from_value(&document.to_value()).unwrap(),
        document
    );
    assert!(document.diff(&reloaded).unwrap().edits.is_empty());
    assert_eq!(
        reloaded.text("/idm/uc/standardProjectPhase/name").unwrap(),
        "a & b < c ]]> d\r\n"
    );
}

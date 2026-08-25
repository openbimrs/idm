//! Lossless ISO 29481-3 idmXML reader, writer and schema-aware editor.
//!
//! The core model is an editable, namespace-aware XML tree. It intentionally
//! does not project XML into a narrow application DTO: idmXML is recursive and
//! its official schemas contain extension-prone structures. Unknown/vendor
//! elements, comments, CDATA, processing instructions and ordering survive a
//! read-modify-write cycle. The generated schema catalog retains every official
//! declaration and drives cardinality-aware creation, validation and UI labels.

use quick_xml::XmlVersion;
use quick_xml::escape::unescape;
use quick_xml::events::{BytesCData, BytesDecl, BytesPI, BytesStart, BytesText, Event};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use quick_xml::writer::Writer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use thiserror::Error;
use uuid::Uuid;

pub const IDM_SCHEMA_LOCATION: &str = "idm.xsd";
pub const DEFAULT_MAX_XML_BYTES: usize = 64 * 1024 * 1024;
pub const DEFAULT_MAX_XML_DEPTH: usize = 1_024;

/// Annex B filenames expected by schema-aware APIs.
///
/// The files themselves are intentionally not redistributed. Supply a directory
/// containing a lawfully obtained schema set to [`schema_text`] and
/// [`local_schema_inventory`].
pub const SCHEMA_FILES: [&str; 6] = [
    "specId.xsd",
    "authoring.xsd",
    "uc.xsd",
    "businessContextMap.xsd",
    "er.xsd",
    "idm.xsd",
];

const SCHEMA_CATALOG_JSON: &str = include_str!("../catalog/catalog.json");

#[derive(Debug, Error)]
pub enum Error {
    #[error("XML input is {actual} bytes; the configured maximum is {maximum} bytes")]
    InputTooLarge { actual: usize, maximum: usize },
    #[error("XML nesting exceeds the maximum depth of {maximum}")]
    MaxDepthExceeded { maximum: usize },
    #[error("invalid XML: {0}")]
    Xml(String),
    #[error("could not serialize XML: {0}")]
    Write(String),
    #[error("serialized XML was not UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("invalid path `{0}`")]
    InvalidPath(String),
    #[error("path not found: `{0}`")]
    PathNotFound(String),
    #[error("schema inventory error: {0}")]
    Schema(String),
    #[error("JSON document error: {0}")]
    Json(String),
    #[error("schema cardinality violation: {0}")]
    Cardinality(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    /// Qualified spelling used in the source, such as `dt:GUID`.
    pub qualified_name: String,
    pub local_name: String,
    pub prefix: Option<String>,
    pub namespace: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Node {
    Element(Element),
    Text(String),
    CData(String),
    Comment(String),
    ProcessingInstruction(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Element {
    /// Qualified spelling used in the source, such as `dt:Name`.
    pub qualified_name: String,
    pub local_name: String,
    pub prefix: Option<String>,
    pub namespace: Option<String>,
    /// Attributes retain source order and qualified names.
    pub attributes: Vec<Attribute>,
    /// Child order, comments, CDATA and processing instructions are preserved.
    pub children: Vec<Node>,
}

impl Element {
    #[must_use]
    pub fn new(qualified_name: &str, namespace: Option<&str>) -> Self {
        let (prefix, local_name) = split_qname(qualified_name);
        Self {
            qualified_name: qualified_name.into(),
            local_name: local_name.into(),
            prefix: prefix.map(str::to_owned),
            namespace: namespace.map(str::to_owned),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    #[must_use]
    pub fn local_name(&self) -> &str {
        &self.local_name
    }

    #[must_use]
    pub fn namespace_uri(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    #[must_use]
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    pub fn set_attribute(&mut self, qualified_name: &str, value: &str) {
        if let Some(attribute) = self
            .attributes
            .iter_mut()
            .find(|attribute| attribute.qualified_name == qualified_name)
        {
            attribute.value = value.into();
            return;
        }
        let (prefix, local_name) = split_qname(qualified_name);
        self.attributes.push(Attribute {
            qualified_name: qualified_name.into(),
            local_name: local_name.into(),
            prefix: prefix.map(str::to_owned),
            namespace: None,
            value: value.into(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    prolog: Vec<Node>,
    root: Element,
    epilog: Vec<Node>,
    #[serde(skip, default = "default_max_xml_bytes")]
    max_xml_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DocumentValue {
    prolog: Vec<Node>,
    root: Element,
    epilog: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaInventory {
    pub elements: BTreeSet<String>,
    pub complex_types: BTreeSet<String>,
    pub simple_types: BTreeSet<String>,
    pub attributes: BTreeSet<String>,
    pub enum_values: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaCatalog {
    pub profile: String,
    pub root: String,
    pub namespace: Option<String>,
    pub schemas: Vec<SchemaSource>,
    pub element_names: BTreeSet<String>,
    pub global_elements: BTreeSet<String>,
    pub attribute_names: BTreeSet<String>,
    pub enum_values: BTreeSet<String>,
    pub recursive_edges: Vec<RecursiveEdge>,
    pub elements: BTreeMap<String, ElementRule>,
    pub semantic_overlays: Vec<SemanticOverlay>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaSource {
    pub file: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecursiveEdge {
    pub from: String,
    pub wrapper: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementRule {
    pub handle: String,
    pub name: String,
    pub global: bool,
    pub source_file: String,
    pub source_line: usize,
    pub label_key: String,
    #[serde(default)]
    pub data_type: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub enum_values: Vec<String>,
    pub attributes: Vec<AttributeRule>,
    pub children: Vec<ChildRule>,
    pub choice_groups: Vec<ChoiceGroup>,
}

impl ElementRule {
    #[must_use]
    pub fn child(&self, name: &str) -> Option<&ChildRule> {
        self.children.iter().find(|child| child.name == name)
    }

    #[must_use]
    pub fn attribute(&self, name: &str) -> Option<&AttributeRule> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeRule {
    pub name: String,
    pub required: bool,
    pub default: Option<String>,
    pub data_type: Option<String>,
    pub pattern: Option<String>,
    pub enum_values: Vec<String>,
    pub label_key: String,
    pub source_file: String,
    pub source_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildRule {
    pub name: String,
    pub definition: String,
    pub min_occurs: usize,
    pub max_occurs: Option<usize>,
    pub choice_group: Option<String>,
    pub recursive: bool,
    pub label_key: String,
    pub source_file: String,
    pub source_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceGroup {
    pub handle: String,
    pub min_occurs: usize,
    pub max_occurs: Option<usize>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticOverlay {
    pub code: String,
    pub path: String,
    pub child: String,
    pub min_occurs: usize,
    pub source: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildAction {
    pub name: String,
    pub definition: String,
    pub label_key: String,
    pub min_occurs: usize,
    pub max_occurs: Option<usize>,
    pub current: usize,
    pub can_add: bool,
    pub recursive: bool,
}

impl SchemaCatalog {
    #[must_use]
    pub fn element(&self, handle: &str) -> Option<&ElementRule> {
        self.elements.get(handle)
    }
}

#[derive(Debug, Clone)]
struct PathSegment {
    name: String,
    index: usize,
}

impl Document {
    pub fn parse(xml: &str) -> Result<Self> {
        Self::parse_with_limit(xml, DEFAULT_MAX_XML_BYTES)
    }

    pub fn parse_with_limit(xml: &str, max_xml_bytes: usize) -> Result<Self> {
        if xml.len() > max_xml_bytes {
            return Err(Error::InputTooLarge {
                actual: xml.len(),
                maximum: max_xml_bytes,
            });
        }
        let (prolog, root, epilog) = parse_xml(xml)?;
        Ok(Self {
            prolog,
            root,
            epilog,
            max_xml_bytes,
        })
    }

    #[must_use]
    pub fn new() -> Self {
        let mut root = Element::new("idm", None);
        root.attributes.push(namespace_attribute(
            "xmlns:xsi",
            "http://www.w3.org/2001/XMLSchema-instance",
        ));
        let mut schema_location = Attribute {
            qualified_name: "xsi:noNamespaceSchemaLocation".into(),
            local_name: "noNamespaceSchemaLocation".into(),
            prefix: Some("xsi".into()),
            namespace: Some("http://www.w3.org/2001/XMLSchema-instance".into()),
            value: IDM_SCHEMA_LOCATION.into(),
        };
        schema_location.value = IDM_SCHEMA_LOCATION.into();
        root.attributes.push(schema_location);
        Self {
            prolog: Vec::new(),
            root,
            epilog: Vec::new(),
            max_xml_bytes: DEFAULT_MAX_XML_BYTES,
        }
    }

    #[must_use]
    pub fn root(&self) -> &Element {
        &self.root
    }

    #[must_use]
    pub fn max_xml_bytes(&self) -> usize {
        self.max_xml_bytes
    }

    /// Resolve an indexed path and borrow the complete element, including all
    /// namespace and extension data.
    pub fn element(&self, path: &str) -> Result<&Element> {
        resolve(&self.root, &parse_path(path)?).ok_or_else(|| Error::PathNotFound(path.into()))
    }

    /// Mutable low-level access for callers that need operations beyond the
    /// convenience editing API.
    pub fn element_mut(&mut self, path: &str) -> Result<&mut Element> {
        resolve_mut(&mut self.root, &parse_path(path)?)
            .ok_or_else(|| Error::PathNotFound(path.into()))
    }

    #[must_use]
    pub fn count(&self, name: &str) -> usize {
        let mut count = 0;
        walk_elements(&self.root, "/idm", &mut |element, _| {
            if element.local_name == name {
                count += 1;
            }
        });
        count
    }

    #[must_use]
    pub fn element_paths(&self, name: &str) -> Vec<String> {
        let mut found = Vec::new();
        walk_elements(&self.root, "/idm", &mut |element, path| {
            if element.local_name == name {
                found.push(path.to_owned());
            }
        });
        found
    }

    pub fn child_element_names(&self, path: &str) -> Result<Vec<String>> {
        let element = self.element(path)?;
        Ok(element
            .children
            .iter()
            .filter_map(|node| match node {
                Node::Element(child) => Some(child.local_name.clone()),
                _ => None,
            })
            .collect())
    }

    #[must_use]
    pub fn find_by_guid(&self, guid: &str) -> Vec<String> {
        let mut found = Vec::new();
        walk_elements(&self.root, "/idm", &mut |element, path| {
            if attribute_by_local(element, "guid") == Some(guid) {
                found.push(path.to_owned());
            }
        });
        found
    }

    pub fn text(&self, path: &str) -> Result<String> {
        let element = resolve(&self.root, &parse_path(path)?)
            .ok_or_else(|| Error::PathNotFound(path.into()))?;
        Ok(element
            .children
            .iter()
            .filter_map(|node| match node {
                Node::Text(value) | Node::CData(value) => Some(value.as_str()),
                _ => None,
            })
            .collect())
    }

    pub fn set_text(&mut self, path: &str, value: &str) -> Result<()> {
        let element = resolve_mut(&mut self.root, &parse_path(path)?)
            .ok_or_else(|| Error::PathNotFound(path.into()))?;
        element
            .children
            .retain(|node| !matches!(node, Node::Text(_) | Node::CData(_)));
        element.children.insert(0, Node::Text(value.into()));
        Ok(())
    }

    pub fn attribute(&self, path: &str, name: &str) -> Result<String> {
        let element = resolve(&self.root, &parse_path(path)?)
            .ok_or_else(|| Error::PathNotFound(path.into()))?;
        element
            .attributes
            .iter()
            .find(|attribute| attribute.qualified_name == name)
            .or_else(|| {
                (!name.contains(':')).then(|| {
                    element
                        .attributes
                        .iter()
                        .find(|attribute| attribute.local_name == name)
                })?
            })
            .map(|attribute| attribute.value.clone())
            .ok_or_else(|| Error::PathNotFound(format!("{path}/@{name}")))
    }

    pub fn set_attribute(&mut self, path: &str, name: &str, value: &str) -> Result<()> {
        let segments = parse_path(path)?;
        let namespace = split_qname(name)
            .0
            .and_then(|prefix| namespace_for_prefix(&self.root, &segments, prefix));
        let element = resolve_mut(&mut self.root, &segments)
            .ok_or_else(|| Error::PathNotFound(path.into()))?;
        element.set_attribute(name, value);
        if let Some(attribute) = element
            .attributes
            .iter_mut()
            .find(|attribute| attribute.qualified_name == name)
        {
            attribute.namespace = namespace;
        }
        Ok(())
    }

    pub fn append_element(
        &mut self,
        parent_path: &str,
        qualified_name: &str,
        namespace: Option<&str>,
    ) -> Result<String> {
        let segments = parse_path(parent_path)?;
        let resolved_namespace = namespace.map(str::to_owned).or_else(|| {
            split_qname(qualified_name)
                .0
                .and_then(|prefix| namespace_for_prefix(&self.root, &segments, prefix))
        });
        let parent = resolve_mut(&mut self.root, &segments)
            .ok_or_else(|| Error::PathNotFound(parent_path.into()))?;
        let (_, local) = split_qname(qualified_name);
        let index = child_elements(parent, local).count();
        parent.children.push(Node::Element(Element::new(
            qualified_name,
            resolved_namespace.as_deref(),
        )));
        Ok(format!("{parent_path}/{local}[{index}]"))
    }

    pub fn remove(&mut self, path: &str) -> Result<()> {
        let mut segments = parse_path(path)?;
        if segments.len() <= 1 {
            return Err(Error::InvalidPath(
                "the root element cannot be removed".into(),
            ));
        }
        let target = segments.pop().expect("checked non-empty");
        let parent = resolve_mut(&mut self.root, &segments)
            .ok_or_else(|| Error::PathNotFound(path.into()))?;
        let position = child_element_position(parent, &target)
            .ok_or_else(|| Error::PathNotFound(path.into()))?;
        parent.children.remove(position);
        Ok(())
    }

    /// Create a complete, schema-ordered IDM authoring skeleton.
    ///
    /// The source XSD represented by the generated catalog makes the root ER optional, while DIN EN ISO 29481-3
    /// Clause 5 says an IDM has one ER. New documents therefore include it and
    /// validation reports its absence as a semantic conformance error.
    pub fn new_idm(full_title: &str, idm_code: &str) -> Result<Self> {
        let catalog = schema_catalog()?;
        let mut document = Self::new();
        let mut ancestors = Vec::new();
        document.root = build_schema_element(&catalog, "idm", &mut ancestors)?;
        document.root.attributes.push(namespace_attribute(
            "xmlns:xsi",
            "http://www.w3.org/2001/XMLSchema-instance",
        ));
        document.root.attributes.push(Attribute {
            qualified_name: "xsi:noNamespaceSchemaLocation".into(),
            local_name: "noNamespaceSchemaLocation".into(),
            prefix: Some("xsi".into()),
            namespace: Some("http://www.w3.org/2001/XMLSchema-instance".into()),
            value: IDM_SCHEMA_LOCATION.into(),
        });
        if child_elements(&document.root, "er").next().is_none() {
            let mut er_ancestors = vec!["idm".to_owned()];
            let er = build_schema_element(&catalog, "er", &mut er_ancestors)?;
            insert_schema_ordered(
                &mut document.root,
                Node::Element(er),
                catalog.element("idm").expect("catalog root"),
            );
        }
        document.set_attribute("/idm/specId[0]", "fullTitle", full_title)?;
        document.set_attribute("/idm/specId[0]", "idmCode", idm_code)?;
        Ok(document)
    }

    pub fn allowed_children(&self, parent_path: &str) -> Result<Vec<ChildAction>> {
        let catalog = schema_catalog()?;
        let handle = schema_handle_for_path(self, &catalog, parent_path)?;
        let definition = catalog
            .element(&handle)
            .ok_or_else(|| Error::Schema(format!("missing schema definition `{handle}`")))?;
        let parent = self.element(parent_path)?;
        Ok(definition
            .children
            .iter()
            .map(|rule| {
                let current = child_elements(parent, &rule.name).count();
                ChildAction {
                    name: rule.name.clone(),
                    definition: rule.definition.clone(),
                    label_key: rule.label_key.clone(),
                    min_occurs: rule.min_occurs,
                    max_occurs: rule.max_occurs,
                    current,
                    can_add: rule.max_occurs.is_none_or(|maximum| current < maximum),
                    recursive: rule.recursive,
                }
            })
            .collect())
    }

    pub fn append_schema_child(&mut self, parent_path: &str, name: &str) -> Result<String> {
        let catalog = schema_catalog()?;
        let parent_handle = schema_handle_for_path(self, &catalog, parent_path)?;
        let parent_rule = catalog
            .element(&parent_handle)
            .ok_or_else(|| Error::Schema(format!("missing schema definition `{parent_handle}`")))?;
        let child_rule = parent_rule.child(name).cloned().ok_or_else(|| {
            Error::Schema(format!("`{name}` is not allowed below `{parent_handle}`"))
        })?;
        let current = child_elements(self.element(parent_path)?, name).count();
        if child_rule
            .max_occurs
            .is_some_and(|maximum| current >= maximum)
        {
            return Err(Error::Cardinality(format!(
                "`{name}` already reached maximum cardinality {} below `{parent_path}`",
                child_rule.max_occurs.expect("checked")
            )));
        }
        let mut ancestors = path_schema_ancestors(self, &catalog, parent_path)?;
        let child = build_schema_element(&catalog, &child_rule.definition, &mut ancestors)?;
        let parent = self.element_mut(parent_path)?;
        insert_schema_ordered(parent, Node::Element(child), parent_rule);
        Ok(format!("{parent_path}/{name}[{current}]"))
    }

    pub fn remove_schema_node(&mut self, path: &str) -> Result<()> {
        let mut segments = parse_path(path)?;
        if segments.len() <= 1 {
            return Err(Error::InvalidPath("the IDM root cannot be removed".into()));
        }
        let target = segments.pop().expect("checked non-empty");
        let parent_path = format_path(&segments);
        if parent_path == "/idm" && target.name == "er" {
            return Err(Error::Cardinality(
                "ISO 29481-3 requires exactly one root exchange requirement".into(),
            ));
        }
        let catalog = schema_catalog()?;
        let parent_handle = schema_handle_for_path(self, &catalog, &parent_path)?;
        let parent_rule = catalog
            .element(&parent_handle)
            .ok_or_else(|| Error::Schema(format!("missing schema definition `{parent_handle}`")))?;
        let child_rule = parent_rule.child(&target.name).ok_or_else(|| {
            Error::Schema(format!(
                "`{}` is not declared below `{parent_handle}`",
                target.name
            ))
        })?;
        let parent = self.element(&parent_path)?;
        let current = child_elements(parent, &target.name).count();
        if current <= child_rule.min_occurs {
            return Err(Error::Cardinality(format!(
                "cannot remove `{}`: minimum cardinality {} below `{parent_path}`",
                target.name, child_rule.min_occurs
            )));
        }
        if let Some(group_handle) = &child_rule.choice_group {
            let group = parent_rule
                .choice_groups
                .iter()
                .find(|group| &group.handle == group_handle)
                .expect("catalog choice reference");
            let aggregate = parent_rule
                .children
                .iter()
                .filter(|rule| rule.choice_group.as_ref() == Some(group_handle))
                .map(|rule| child_elements(parent, &rule.name).count())
                .sum::<usize>();
            if aggregate <= group.min_occurs {
                return Err(Error::Cardinality(format!(
                    "cannot remove `{}`: choice group minimum cardinality {} below `{parent_path}`",
                    target.name, group.min_occurs
                )));
            }
        }
        self.remove(path)
    }

    /// Move an element before or after a same-name sibling.
    ///
    /// Restricting movement to same-name siblings under one parent keeps the
    /// generated XSD particle order valid while allowing repeatable authoring
    /// components to be sorted without re-serializing their contents.
    pub fn move_schema_node(
        &mut self,
        path: &str,
        target_path: &str,
        after: bool,
    ) -> Result<String> {
        let mut source_segments = parse_path(path)?;
        let mut target_segments = parse_path(target_path)?;
        if source_segments.len() <= 1 || target_segments.len() <= 1 {
            return Err(Error::InvalidPath("the IDM root cannot be moved".into()));
        }
        let source = source_segments.pop().expect("checked non-empty");
        let target = target_segments.pop().expect("checked non-empty");
        let source_parent_path = path.rsplit_once('/').map_or("", |(parent, _)| parent);
        let target_parent_path = target_path
            .rsplit_once('/')
            .map_or("", |(parent, _)| parent);
        if source_parent_path != target_parent_path || source.name != target.name {
            return Err(Error::Cardinality(
                "schema reorder is limited to same-name siblings under one parent".into(),
            ));
        }
        if source.index == target.index {
            return Ok(path.to_owned());
        }

        let parent = resolve_mut(&mut self.root, &source_segments)
            .ok_or_else(|| Error::PathNotFound(source_parent_path.to_owned()))?;
        let source_position = child_element_position(parent, &source)
            .ok_or_else(|| Error::PathNotFound(path.into()))?;
        let target_position = child_element_position(parent, &target)
            .ok_or_else(|| Error::PathNotFound(target_path.into()))?;
        let node = parent.children.remove(source_position);
        let mut insertion_position = target_position + usize::from(after);
        if source_position < insertion_position {
            insertion_position -= 1;
        }
        parent.children.insert(insertion_position, node);
        let new_index = parent.children[..insertion_position]
            .iter()
            .filter(
                |node| matches!(node, Node::Element(element) if element.local_name == source.name),
            )
            .count();
        Ok(format!("{source_parent_path}/{}[{new_index}]", source.name))
    }

    /// Validate the complete tree against the generated XSD declaration catalog
    /// and the explicit standard-over-XSD semantic overlays.
    #[must_use]
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let Ok(catalog) = schema_catalog() else {
            return vec![issue(
                "schema_catalog",
                "/",
                "Generated schema catalog could not be loaded",
            )];
        };
        let mut issues = Vec::new();
        if self.root.local_name != "idm" || self.root.namespace.is_some() {
            issues.push(issue(
                "invalid_root",
                "/",
                "Root must be the unqualified idm element declared by ISO 29481-3",
            ));
            return issues;
        }
        validate_schema_element(&self.root, "/idm", "idm", &catalog, &mut issues);
        let mut guids = BTreeSet::new();
        walk_elements(&self.root, "/idm", &mut |element, path| {
            if let Some(guid) = attribute_by_local(element, "guid") {
                if !guids.insert(guid.to_owned()) {
                    issues.push(issue("duplicate_guid", path, "guid values must be unique"));
                }
            }
        });
        for overlay in &catalog.semantic_overlays {
            if overlay.path == "/idm"
                && child_elements(&self.root, &overlay.child).count() < overlay.min_occurs
            {
                issues.push(issue(&overlay.code, &overlay.path, &overlay.reason));
            }
        }
        issues
    }

    pub fn to_xml(&self, pretty: bool) -> Result<String> {
        let mut writer = if pretty {
            Writer::new_with_indent(Vec::new(), b' ', 2)
        } else {
            Writer::new(Vec::new())
        };
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(write_error)?;
        for node in &self.prolog {
            write_document_level_node(&mut writer, node)?;
        }
        write_element(&mut writer, &self.root)?;
        for node in &self.epilog {
            write_document_level_node(&mut writer, node)?;
        }
        Ok(String::from_utf8(writer.into_inner())?)
    }

    /// Complete lossless tree JSON. This is suitable for durable interchange,
    /// diffing, language bindings and editing without dropping schema content.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(DocumentValue {
            prolog: self.prolog.clone(),
            root: self.root.clone(),
            epilog: self.epilog.clone(),
        })
        .expect("Document serialization is infallible")
    }

    pub fn from_value(value: &Value) -> Result<Self> {
        let mut tree: DocumentValue = serde_json::from_value(value.clone())
            .map_err(|error| Error::Json(error.to_string()))?;
        normalize_empty_text(&mut tree.root);
        if tree.root.qualified_name.is_empty() || tree.root.local_name.is_empty() {
            return Err(Error::Json("root names cannot be empty".into()));
        }
        let document = Self {
            prolog: tree.prolog,
            root: tree.root,
            epilog: tree.epilog,
            max_xml_bytes: DEFAULT_MAX_XML_BYTES,
        };
        let xml = document.to_xml(false)?;
        let reparsed = Self::parse(&xml).map_err(|error| {
            Error::Json(format!(
                "tree does not serialize to valid namespace-aware XML: {error}"
            ))
        })?;
        if reparsed.prolog != document.prolog
            || reparsed.root != document.root
            || reparsed.epilog != document.epilog
        {
            return Err(Error::Json(
                "tree namespace or document-level metadata does not match the serialized XML"
                    .into(),
            ));
        }
        Ok(document)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// Read one recognized XSD from an explicit schema directory.
///
/// This API never downloads schemas and never falls back to embedded standards
/// material. `name` must be one of [`SCHEMA_FILES`], preventing path traversal.
pub fn schema_text(schema_dir: impl AsRef<Path>, name: &str) -> Result<String> {
    if !SCHEMA_FILES.contains(&name) {
        return Err(Error::Schema(format!("unknown schema filename `{name}`")));
    }
    let path = schema_dir.as_ref().join(name);
    fs::read_to_string(&path)
        .map_err(|error| Error::Schema(format!("could not read `{}`: {error}", path.display())))
}

/// Return the generated declaration catalog.
///
/// The catalog contains names, content models, source coordinates and source
/// hashes, but no XSD bytes.
pub fn schema_catalog() -> Result<SchemaCatalog> {
    serde_json::from_str(SCHEMA_CATALOG_JSON).map_err(|error| Error::Schema(error.to_string()))
}

/// Inspect a lawfully obtained six-file schema set in `schema_dir`.
pub fn local_schema_inventory(schema_dir: impl AsRef<Path>) -> Result<SchemaInventory> {
    let catalog = schema_catalog()?;
    let mut complex_types = BTreeSet::new();
    let mut simple_types = BTreeSet::new();
    for name in SCHEMA_FILES {
        let source = schema_text(&schema_dir, name)?;
        let schema = Document::parse(&source).map_err(|error| Error::Schema(error.to_string()))?;
        let mut partial = SchemaInventory {
            elements: BTreeSet::new(),
            complex_types: BTreeSet::new(),
            simple_types: BTreeSet::new(),
            attributes: BTreeSet::new(),
            enum_values: BTreeSet::new(),
        };
        collect_schema(schema.root(), &mut partial);
        complex_types.extend(partial.complex_types);
        simple_types.extend(partial.simple_types);
    }
    Ok(SchemaInventory {
        elements: catalog.element_names,
        complex_types,
        simple_types,
        attributes: catalog.attribute_names,
        enum_values: catalog.enum_values,
    })
}

fn parse_xml(xml: &str) -> Result<(Vec<Node>, Element, Vec<Node>)> {
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Element> = Vec::new();
    let mut prolog = Vec::new();
    let mut root = None;
    let mut epilog = Vec::new();

    loop {
        let (resolution, event) = reader
            .read_resolved_event()
            .map_err(|error| Error::Xml(error.to_string()))?;
        let namespace = resolved_namespace(resolution)?;
        let event = event.into_owned();
        match event {
            Event::Start(start) => {
                if stack.len() >= DEFAULT_MAX_XML_DEPTH {
                    return Err(Error::MaxDepthExceeded {
                        maximum: DEFAULT_MAX_XML_DEPTH,
                    });
                }
                stack.push(read_element(&reader, namespace, &start)?);
            }
            Event::Empty(start) => {
                let element = read_element(&reader, namespace, &start)?;
                push_parsed_node(&mut stack, &mut root, Node::Element(element))?;
            }
            Event::End(_) => {
                let element = stack
                    .pop()
                    .ok_or_else(|| Error::Xml("unexpected closing element".into()))?;
                push_parsed_node(&mut stack, &mut root, Node::Element(element))?;
            }
            Event::Text(text) => {
                let decoded = text
                    .decode()
                    .map_err(|error| Error::Xml(error.to_string()))?;
                let decoded = unescape(&decoded)
                    .map_err(|error| Error::Xml(error.to_string()))?
                    .into_owned();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::Text(decoded));
                } else if !decoded.trim().is_empty() {
                    return Err(Error::Xml("text outside the root element".into()));
                }
            }
            Event::CData(cdata) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::CData(
                        cdata
                            .decode()
                            .map_err(|error| Error::Xml(error.to_string()))?
                            .into_owned(),
                    ));
                }
            }
            Event::Comment(comment) => {
                let node = Node::Comment(
                    comment
                        .decode()
                        .map_err(|error| Error::Xml(error.to_string()))?
                        .into_owned(),
                );
                push_misc_node(&mut stack, &mut prolog, &root, &mut epilog, node);
            }
            Event::PI(pi) => {
                let node =
                    Node::ProcessingInstruction(String::from_utf8_lossy(pi.as_ref()).into_owned());
                push_misc_node(&mut stack, &mut prolog, &root, &mut epilog, node);
            }
            Event::Decl(_) => {}
            Event::DocType(_) => {
                return Err(Error::Xml(
                    "DOCTYPE declarations are not supported in IDM documents".into(),
                ));
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err(Error::Xml("unclosed element".into()));
    }
    Ok((
        prolog,
        root.ok_or_else(|| Error::Xml("document has no root element".into()))?,
        epilog,
    ))
}

fn read_element(
    reader: &NsReader<&[u8]>,
    namespace: Option<String>,
    start: &BytesStart<'_>,
) -> Result<Element> {
    let qualified_name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let local_name = String::from_utf8_lossy(start.local_name().as_ref()).into_owned();
    let prefix = split_qname(&qualified_name).0.map(str::to_owned);
    let mut attributes = Vec::new();
    for attribute in start.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| Error::Xml(error.to_string()))?;
        let qualified = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
        let (_, local) = split_qname(&qualified);
        let namespace = if qualified == "xmlns" || qualified.starts_with("xmlns:") {
            Some("http://www.w3.org/2000/xmlns/".into())
        } else {
            let (resolved, _) = reader.resolver().resolve_attribute(attribute.key);
            resolved_namespace(resolved)?
        };
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, start.decoder())
            .map_err(|error| Error::Xml(error.to_string()))?
            .into_owned();
        attributes.push(Attribute {
            qualified_name: qualified.clone(),
            local_name: local.into(),
            prefix: split_qname(&qualified).0.map(str::to_owned),
            namespace,
            value,
        });
    }
    Ok(Element {
        qualified_name,
        local_name,
        prefix,
        namespace,
        attributes,
        children: Vec::new(),
    })
}

fn resolved_namespace(resolution: ResolveResult<'_>) -> Result<Option<String>> {
    match resolution {
        ResolveResult::Unbound => Ok(None),
        ResolveResult::Bound(namespace) => Ok(Some(
            String::from_utf8_lossy(namespace.as_ref()).into_owned(),
        )),
        ResolveResult::Unknown(prefix) => Err(Error::Xml(format!(
            "unknown namespace prefix `{}`",
            String::from_utf8_lossy(&prefix)
        ))),
    }
}

fn push_parsed_node(stack: &mut [Element], root: &mut Option<Element>, node: Node) -> Result<()> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
        return Ok(());
    }
    let Node::Element(element) = node else {
        return Err(Error::Xml("non-element root".into()));
    };
    if root.replace(element).is_some() {
        return Err(Error::Xml("document has multiple root elements".into()));
    }
    Ok(())
}

fn push_misc_node(
    stack: &mut [Element],
    prolog: &mut Vec<Node>,
    root: &Option<Element>,
    epilog: &mut Vec<Node>,
    node: Node,
) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else if root.is_some() {
        epilog.push(node);
    } else {
        prolog.push(node);
    }
}

fn write_element(writer: &mut Writer<Vec<u8>>, element: &Element) -> Result<()> {
    let mut start = BytesStart::new(&element.qualified_name);
    for attribute in &element.attributes {
        start.push_attribute((attribute.qualified_name.as_str(), attribute.value.as_str()));
    }
    if element.children.is_empty() {
        writer
            .write_event(Event::Empty(start))
            .map_err(write_error)?;
        return Ok(());
    }
    writer
        .write_event(Event::Start(start.borrow()))
        .map_err(write_error)?;
    for child in &element.children {
        match child {
            Node::Element(child) => write_element(writer, child)?,
            Node::Text(value) => writer
                .write_event(Event::Text(BytesText::new(value)))
                .map_err(write_error)?,
            Node::CData(value) => writer
                .write_event(Event::CData(BytesCData::new(value)))
                .map_err(write_error)?,
            Node::Comment(value) => writer
                .write_event(Event::Comment(BytesText::new(value)))
                .map_err(write_error)?,
            Node::ProcessingInstruction(value) => writer
                .write_event(Event::PI(BytesPI::new(value)))
                .map_err(write_error)?,
        }
    }
    writer
        .write_event(Event::End(start.to_end()))
        .map_err(write_error)?;
    Ok(())
}

fn write_document_level_node(writer: &mut Writer<Vec<u8>>, node: &Node) -> Result<()> {
    match node {
        Node::Comment(value) => writer
            .write_event(Event::Comment(BytesText::new(value)))
            .map_err(write_error),
        Node::ProcessingInstruction(value) => writer
            .write_event(Event::PI(BytesPI::new(value)))
            .map_err(write_error),
        Node::Element(_) | Node::Text(_) | Node::CData(_) => Err(Error::Write(
            "only comments and processing instructions are valid outside the root element".into(),
        )),
    }
}

fn write_error(error: std::io::Error) -> Error {
    Error::Write(error.to_string())
}

fn collect_schema(element: &Element, inventory: &mut SchemaInventory) {
    if let Some(name) = attribute_by_local(element, "name") {
        match element.local_name.as_str() {
            "element" => {
                inventory.elements.insert(name.into());
            }
            "complexType" => {
                inventory.complex_types.insert(name.into());
            }
            "simpleType" => {
                inventory.simple_types.insert(name.into());
            }
            "attribute" => {
                inventory.attributes.insert(name.into());
            }
            _ => {}
        }
    }
    if element.local_name == "enumeration" {
        if let Some(value) = attribute_by_local(element, "value") {
            inventory.enum_values.insert(value.into());
        }
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            collect_schema(child, inventory);
        }
    }
}

fn issue(code: &str, path: &str, message: &str) -> ValidationIssue {
    ValidationIssue {
        severity: ValidationSeverity::Error,
        code: code.into(),
        path: path.into(),
        message: message.into(),
    }
}

fn namespace_attribute(name: &str, value: &str) -> Attribute {
    let (prefix, local) = split_qname(name);
    Attribute {
        qualified_name: name.into(),
        local_name: local.into(),
        prefix: prefix.map(str::to_owned),
        namespace: Some("http://www.w3.org/2000/xmlns/".into()),
        value: value.into(),
    }
}

fn schema_handle_for_path(
    document: &Document,
    catalog: &SchemaCatalog,
    path: &str,
) -> Result<String> {
    let segments = parse_path(path)?;
    if segments
        .first()
        .is_none_or(|segment| segment.name != catalog.root)
    {
        return Err(Error::Schema(format!(
            "`{path}` is outside the IDM schema root"
        )));
    }
    document.element(path)?;
    let mut handle = catalog.root.clone();
    for segment in &segments[1..] {
        let definition = catalog
            .element(&handle)
            .ok_or_else(|| Error::Schema(format!("missing schema definition `{handle}`")))?;
        let child = definition.child(&segment.name).ok_or_else(|| {
            Error::Schema(format!(
                "`{}` is not declared below `{handle}`",
                segment.name
            ))
        })?;
        handle = child.definition.clone();
    }
    Ok(handle)
}

fn path_schema_ancestors(
    document: &Document,
    catalog: &SchemaCatalog,
    path: &str,
) -> Result<Vec<String>> {
    let segments = parse_path(path)?;
    document.element(path)?;
    let mut ancestors = vec![catalog.root.clone()];
    let mut handle = catalog.root.clone();
    for segment in &segments[1..] {
        let definition = catalog
            .element(&handle)
            .ok_or_else(|| Error::Schema(format!("missing schema definition `{handle}`")))?;
        handle = definition
            .child(&segment.name)
            .ok_or_else(|| {
                Error::Schema(format!(
                    "`{}` is not declared below `{handle}`",
                    segment.name
                ))
            })?
            .definition
            .clone();
        ancestors.push(handle.clone());
    }
    Ok(ancestors)
}

fn build_schema_element(
    catalog: &SchemaCatalog,
    handle: &str,
    ancestors: &mut Vec<String>,
) -> Result<Element> {
    if ancestors.len() >= DEFAULT_MAX_XML_DEPTH {
        return Err(Error::Schema(format!(
            "schema skeleton exceeds maximum depth of {DEFAULT_MAX_XML_DEPTH} at `{handle}`"
        )));
    }
    let definition = catalog
        .element(handle)
        .ok_or_else(|| Error::Schema(format!("missing schema definition `{handle}`")))?;
    ancestors.push(handle.to_owned());
    let mut element = Element::new(&definition.name, None);
    for attribute in &definition.attributes {
        if attribute.required {
            element.set_attribute(&attribute.name, &default_attribute_value(attribute));
        }
    }
    if let Some(value) = definition.enum_values.first() {
        element.children.push(Node::Text(value.clone()));
    }

    for child in definition
        .children
        .iter()
        .filter(|child| child.choice_group.is_none())
    {
        for _ in 0..child.min_occurs {
            let nested = build_schema_element(catalog, &child.definition, ancestors)?;
            element.children.push(Node::Element(nested));
        }
    }
    for group in &definition.choice_groups {
        let Some(selected) = definition
            .children
            .iter()
            .find(|child| child.choice_group.as_ref() == Some(&group.handle))
        else {
            continue;
        };
        for _ in 0..group.min_occurs {
            let nested = build_schema_element(catalog, &selected.definition, ancestors)?;
            element.children.push(Node::Element(nested));
        }
    }
    // Annex B models these recursive wrappers through an optional nested
    // sequence. Once a user explicitly adds the wrapper, an empty wrapper is
    // useless; create its one target component while keeping recursion finite.
    if matches!(
        definition.name.as_str(),
        "subIdm" | "subUc" | "subEr" | "subInformationUnit" | "subBusinessContextMap" | "subPm"
    ) && !definition.children.is_empty()
        && !element
            .children
            .iter()
            .any(|node| matches!(node, Node::Element(_)))
    {
        let child = &definition.children[0];
        let nested = build_schema_element(catalog, &child.definition, ancestors)?;
        element.children.push(Node::Element(nested));
    }
    ancestors.pop();
    Ok(element)
}

fn default_attribute_value(attribute: &AttributeRule) -> String {
    if let Some(default) = &attribute.default {
        return default.clone();
    }
    if let Some(value) = attribute.enum_values.first() {
        return value.clone();
    }
    match attribute.name.as_str() {
        "guid" => Uuid::new_v4().to_string(),
        "id" => format!("id-{}", Uuid::new_v4().simple()),
        "documentStatus" => "draft".into(),
        "changeDateTime" => "1970-01-01T00:00:00Z".into(),
        "changedBy" => "author-1".into(),
        "isMandatory" => "false".into(),
        "publicationDate" => "1970".into(),
        _ => String::new(),
    }
}

fn insert_schema_ordered(parent: &mut Element, node: Node, parent_rule: &ElementRule) {
    let child_name = match &node {
        Node::Element(element) => element.local_name.as_str(),
        _ => {
            parent.children.push(node);
            return;
        }
    };
    let target_order = parent_rule
        .children
        .iter()
        .position(|rule| rule.name == child_name)
        .unwrap_or(usize::MAX);
    let position = parent.children.iter().position(|existing| {
        let Node::Element(existing) = existing else {
            return false;
        };
        parent_rule
            .children
            .iter()
            .position(|rule| rule.name == existing.local_name)
            .is_some_and(|order| order > target_order)
    });
    if let Some(position) = position {
        parent.children.insert(position, node);
    } else {
        parent.children.push(node);
    }
}

fn validate_schema_element(
    element: &Element,
    path: &str,
    handle: &str,
    catalog: &SchemaCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(definition) = catalog.element(handle) else {
        issues.push(issue(
            "schema_definition",
            path,
            &format!("No schema declaration for `{handle}`"),
        ));
        return;
    };
    for attribute in &definition.attributes {
        if attribute.required && attribute_by_local(element, &attribute.name).is_none() {
            issues.push(issue(
                "required_attribute",
                path,
                &format!("{} requires attribute {}", definition.name, attribute.name),
            ));
        }
        if attribute.name == "guid" {
            if let Some(value) = attribute_by_local(element, "guid") {
                if Uuid::parse_str(value).is_err() || value != value.to_ascii_lowercase() {
                    issues.push(issue(
                        "attribute_pattern",
                        path,
                        "guid must match the lowercase UUID pattern declared by specId.xsd",
                    ));
                }
            }
        }
        if !attribute.enum_values.is_empty() {
            if let Some(value) = attribute_by_local(element, &attribute.name) {
                if !attribute.enum_values.iter().any(|allowed| allowed == value) {
                    issues.push(issue(
                        "attribute_enumeration",
                        path,
                        &format!("{} is not an allowed value for {}", value, attribute.name),
                    ));
                }
            }
        }
    }

    for child_rule in &definition.children {
        let count = child_elements(element, &child_rule.name).count();
        if count < child_rule.min_occurs {
            issues.push(issue(
                "minimum_cardinality",
                path,
                &format!(
                    "{} requires at least {} {} child element(s)",
                    definition.name, child_rule.min_occurs, child_rule.name
                ),
            ));
        }
        if child_rule.max_occurs.is_some_and(|maximum| count > maximum) {
            issues.push(issue(
                "maximum_cardinality",
                path,
                &format!(
                    "{} permits at most {} {} child element(s)",
                    definition.name,
                    child_rule.max_occurs.expect("checked"),
                    child_rule.name
                ),
            ));
        }
    }
    for group in &definition.choice_groups {
        let count = definition
            .children
            .iter()
            .filter(|child| child.choice_group.as_ref() == Some(&group.handle))
            .map(|child| child_elements(element, &child.name).count())
            .sum::<usize>();
        if count < group.min_occurs || group.max_occurs.is_some_and(|maximum| count > maximum) {
            issues.push(issue(
                "choice_cardinality",
                path,
                &format!(
                    "{} violates choice group {} cardinality",
                    definition.name, group.handle
                ),
            ));
        }
    }

    let mut seen_order = 0;
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    for child in &element.children {
        let Node::Element(child) = child else {
            continue;
        };
        let index = names.entry(&child.local_name).or_default();
        let child_path = format!("{path}/{}[{}]", child.local_name, *index);
        *index += 1;
        if let Some((order, child_rule)) = definition
            .children
            .iter()
            .enumerate()
            .find(|(_, rule)| rule.name == child.local_name)
        {
            if order < seen_order {
                issues.push(issue(
                    "schema_order",
                    &child_path,
                    "Element is not in XSD declaration order",
                ));
            }
            seen_order = order;
            validate_schema_element(child, &child_path, &child_rule.definition, catalog, issues);
        } else {
            issues.push(warning(
                "extension_element",
                &child_path,
                "Element is not declared at this position; it is preserved as extension content",
            ));
        }
    }
}

fn format_path(segments: &[PathSegment]) -> String {
    let mut path = String::new();
    for segment in segments {
        path.push('/');
        path.push_str(&segment.name);
        if segment.index > 0 {
            path.push_str(&format!("[{}]", segment.index));
        }
    }
    path
}

fn warning(code: &str, path: &str, message: &str) -> ValidationIssue {
    ValidationIssue {
        severity: ValidationSeverity::Warning,
        code: code.into(),
        path: path.into(),
        message: message.into(),
    }
}

fn normalize_empty_text(element: &mut Element) {
    element
        .children
        .retain(|node| !matches!(node, Node::Text(value) if value.is_empty()));
    for child in &mut element.children {
        if let Node::Element(child) = child {
            normalize_empty_text(child);
        }
    }
}

fn split_qname(name: &str) -> (Option<&str>, &str) {
    match name.split_once(':') {
        Some((prefix, local)) => (Some(prefix), local),
        None => (None, name),
    }
}

fn local_part(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn attribute_by_local<'a>(element: &'a Element, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.local_name == name)
        .map(|attribute| attribute.value.as_str())
}

fn child_elements<'a>(element: &'a Element, name: &str) -> impl Iterator<Item = &'a Element> {
    let name = name.to_owned();
    element.children.iter().filter_map(move |node| match node {
        Node::Element(child) if child.local_name == name => Some(child),
        _ => None,
    })
}

fn default_max_xml_bytes() -> usize {
    DEFAULT_MAX_XML_BYTES
}

fn parse_path(path: &str) -> Result<Vec<PathSegment>> {
    if !path.starts_with('/') {
        return Err(Error::InvalidPath(path.into()));
    }
    let mut segments = Vec::new();
    for raw in path.split('/').filter(|part| !part.is_empty()) {
        let (name, index) = if let Some((name, suffix)) = raw.rsplit_once('[') {
            let index = suffix
                .strip_suffix(']')
                .ok_or_else(|| Error::InvalidPath(path.into()))?
                .parse::<usize>()
                .map_err(|_| Error::InvalidPath(path.into()))?;
            (name, index)
        } else {
            (raw, 0)
        };
        if name.is_empty() || name.starts_with('@') {
            return Err(Error::InvalidPath(path.into()));
        }
        segments.push(PathSegment {
            name: local_part(name).into(),
            index,
        });
    }
    if segments.is_empty() {
        return Err(Error::InvalidPath(path.into()));
    }
    Ok(segments)
}

fn resolve<'a>(root: &'a Element, segments: &[PathSegment]) -> Option<&'a Element> {
    let first = segments.first()?;
    if first.name != root.local_name || first.index != 0 {
        return None;
    }
    let mut current = root;
    for segment in &segments[1..] {
        current = child_elements(current, &segment.name).nth(segment.index)?;
    }
    Some(current)
}

fn resolve_mut<'a>(root: &'a mut Element, segments: &[PathSegment]) -> Option<&'a mut Element> {
    let first = segments.first()?;
    if first.name != root.local_name || first.index != 0 {
        return None;
    }
    resolve_mut_tail(root, &segments[1..])
}

fn resolve_mut_tail<'a>(
    element: &'a mut Element,
    segments: &[PathSegment],
) -> Option<&'a mut Element> {
    let Some(segment) = segments.first() else {
        return Some(element);
    };
    let position = child_element_position(element, segment)?;
    let Node::Element(child) = &mut element.children[position] else {
        unreachable!("position only points at elements")
    };
    resolve_mut_tail(child, &segments[1..])
}

fn child_element_position(element: &Element, target: &PathSegment) -> Option<usize> {
    element
        .children
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, Node::Element(child) if child.local_name == target.name))
        .nth(target.index)
        .map(|(position, _)| position)
}

fn namespace_for_prefix(root: &Element, segments: &[PathSegment], prefix: &str) -> Option<String> {
    let mut current = root;
    let mut namespace = namespace_declaration(current, prefix);
    for segment in &segments[1..] {
        current = child_elements(current, &segment.name).nth(segment.index)?;
        namespace = namespace_declaration(current, prefix).or(namespace);
    }
    namespace
}

fn namespace_declaration(element: &Element, prefix: &str) -> Option<String> {
    let qualified_name = format!("xmlns:{prefix}");
    element
        .attributes
        .iter()
        .find(|attribute| attribute.qualified_name == qualified_name)
        .map(|attribute| attribute.value.clone())
}

fn walk_elements(element: &Element, path: &str, visitor: &mut impl FnMut(&Element, &str)) {
    visitor(element, path);
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    for child in &element.children {
        if let Node::Element(child) = child {
            let index = names.entry(&child.local_name).or_default();
            let child_path = format!("{path}/{}[{}]", child.local_name, *index);
            *index += 1;
            walk_elements(child, &child_path, visitor);
        }
    }
}

#[cfg(feature = "python")]
mod python;

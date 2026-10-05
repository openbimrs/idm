//! Node inspection, attribute editing and stable locators.
//!
//! These APIs give UIs what they need per selected node without serializing the
//! whole document: current content, the schema rule that applies, and a way to
//! address elements by `guid`/`id` instead of positional indexes.

use super::*;
use std::borrow::Cow;

/// One attribute as stored on an element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeInfo {
    pub name: String,
    pub qualified_name: String,
    pub value: String,
    /// True when the schema declares this attribute on the element.
    pub declared: bool,
}

/// A child element reference in document order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildInfo {
    pub name: String,
    pub path: String,
    /// True when the schema declares this child below the parent.
    pub declared: bool,
}

/// Everything a property panel needs for one element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeInfo {
    pub path: String,
    pub name: String,
    pub namespace: Option<String>,
    /// Catalog definition handle, or `None` for extension/undeclared content.
    pub handle: Option<String>,
    pub text: String,
    pub attributes: Vec<AttributeInfo>,
    pub children: Vec<ChildInfo>,
}

/// A declared attribute together with its current state on one element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeSlot {
    pub rule: AttributeRule,
    pub present: bool,
    pub value: Option<String>,
}

impl Document {
    /// Resolve a path or a stable locator to an indexed path.
    ///
    /// Accepts an indexed path (`/idm/uc[0]`), `guid:<value>` or `id:<value>`.
    /// A locator must match exactly one element.
    pub fn resolve_locator<'a>(&self, locator: &'a str) -> Result<Cow<'a, str>> {
        if locator.starts_with('/') {
            return Ok(Cow::Borrowed(locator));
        }
        let (kind, value) = locator
            .split_once(':')
            .ok_or_else(|| Error::InvalidPath(locator.into()))?;
        let mut found = match kind {
            "guid" => self.find_by_guid(value),
            "id" => self.find_by_id(value),
            _ => return Err(Error::InvalidPath(locator.into())),
        };
        match found.len() {
            0 => Err(Error::PathNotFound(locator.into())),
            1 => Ok(Cow::Owned(found.remove(0))),
            _ => Err(Error::InvalidPath(format!(
                "locator `{locator}` is ambiguous ({} matches)",
                found.len()
            ))),
        }
    }

    /// Paths of every element whose `id` attribute equals `id`.
    #[must_use]
    pub fn find_by_id(&self, id: &str) -> Vec<String> {
        let mut found = Vec::new();
        walk_elements(&self.root, "/idm", &mut |element, path| {
            if attribute_by_local(element, "id") == Some(id) {
                found.push(path.to_owned());
            }
        });
        found
    }

    /// The indexed path of an element, given any locator.
    pub fn path_of(&self, locator: &str) -> Result<String> {
        let path = self.resolve_locator(locator)?.into_owned();
        self.element(&path)?;
        Ok(path)
    }

    /// The catalog rule that applies at `path`.
    ///
    /// Returns [`Error::Schema`] for extension elements the catalog does not
    /// declare, which callers can treat as "free-form".
    pub fn schema_rule(&self, path: &str) -> Result<&'static ElementRule> {
        let path = self.resolve_locator(path)?;
        let catalog = cached_catalog()?;
        let handle = schema_handle_for_path(self, catalog, &path)?;
        catalog
            .element(&handle)
            .ok_or_else(|| Error::Schema(format!("missing schema definition `{handle}`")))
    }

    /// Read one element: attributes, text and children, with schema flags.
    pub fn node_info(&self, path: &str) -> Result<NodeInfo> {
        let path = self.resolve_locator(path)?.into_owned();
        let element = self.element(&path)?;
        let rule = self.schema_rule(&path).ok();
        let handle = cached_catalog()
            .ok()
            .and_then(|catalog| schema_handle_for_path(self, catalog, &path).ok());
        let attributes = element
            .attributes
            .iter()
            .map(|attribute| AttributeInfo {
                name: attribute.local_name.clone(),
                qualified_name: attribute.qualified_name.clone(),
                value: attribute.value.clone(),
                declared: rule.is_some_and(|rule| {
                    attribute.prefix.is_none() && rule.attribute(&attribute.local_name).is_some()
                }),
            })
            .collect();
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut children = Vec::new();
        for node in &element.children {
            if let Node::Element(child) = node {
                let index = counts.entry(&child.local_name).or_default();
                children.push(ChildInfo {
                    name: child.local_name.clone(),
                    path: format!("{path}/{}[{}]", child.local_name, *index),
                    declared: rule.is_some_and(|rule| rule.child(&child.local_name).is_some()),
                });
                *index += 1;
            }
        }
        Ok(NodeInfo {
            path,
            name: element.local_name.clone(),
            namespace: element.namespace.clone(),
            handle,
            text: element_text(element),
            attributes,
            children,
        })
    }

    /// Stored attributes of an element.
    pub fn attributes(&self, path: &str) -> Result<Vec<AttributeInfo>> {
        Ok(self.node_info(path)?.attributes)
    }

    /// Every attribute the schema declares for the element, with current state.
    pub fn attribute_slots(&self, path: &str) -> Result<Vec<AttributeSlot>> {
        let path = self.resolve_locator(path)?.into_owned();
        let rule = self.schema_rule(&path)?;
        let element = self.element(&path)?;
        Ok(rule
            .attributes
            .iter()
            .map(|attribute| {
                let value = attribute_by_local(element, &attribute.name).map(str::to_owned);
                AttributeSlot {
                    rule: attribute.clone(),
                    present: value.is_some(),
                    value,
                }
            })
            .collect())
    }

    /// Remove an attribute. Declared required attributes are rejected with
    /// [`Error::Content`]; extension attributes can always be removed.
    pub fn remove_attribute(&mut self, path: &str, name: &str) -> Result<()> {
        let path = self.resolve_locator(path)?.into_owned();
        if let Ok(rule) = self.schema_rule(&path) {
            if rule
                .attribute(name)
                .is_some_and(|attribute| attribute.required)
            {
                return Err(Error::Content(format!(
                    "attribute `{name}` is required on `{}`",
                    rule.name
                )));
            }
        }
        self.remove_attribute_unchecked(&path, name)
    }

    pub(crate) fn remove_attribute_unchecked(&mut self, path: &str, name: &str) -> Result<()> {
        let element = self.element_mut(path)?;
        let position = element
            .attributes
            .iter()
            .position(|attribute| attribute.qualified_name == name)
            .or_else(|| {
                (!name.contains(':')).then(|| {
                    element
                        .attributes
                        .iter()
                        .position(|attribute| attribute.local_name == name)
                })?
            })
            .ok_or_else(|| Error::PathNotFound(format!("{path}/@{name}")))?;
        element.attributes.remove(position);
        Ok(())
    }

    /// Add a schema-declared optional attribute with its default value.
    ///
    /// Returns the value that was written.
    pub fn add_schema_attribute(&mut self, path: &str, name: &str) -> Result<String> {
        let path = self.resolve_locator(path)?.into_owned();
        let rule = self.schema_rule(&path)?;
        let attribute = rule
            .attribute(name)
            .ok_or_else(|| Error::Schema(format!("`{name}` is not declared on `{}`", rule.name)))?;
        if attribute_by_local(self.element(&path)?, name).is_some() {
            return Err(Error::Content(format!(
                "attribute `{name}` is already present on `{path}`"
            )));
        }
        let value = default_attribute_value(attribute);
        self.set_attribute(&path, name, &value)?;
        Ok(value)
    }
}

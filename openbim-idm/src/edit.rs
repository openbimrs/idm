//! Versioned, reversible edit operations.
//!
//! An [`Edit`] is a small serializable description of one change. Applying it
//! with [`Document::apply`] returns the inverse edit, so undo/redo is a stack of
//! edits. Structural edits go through the same schema, cardinality and order
//! checks as the convenience editing API, and a failed edit leaves the document
//! unchanged. [`Document::diff`] derives a best-effort edit list between two
//! documents.

use super::*;

/// Version written into serialized [`EditBatch`]es.
pub const EDIT_FORMAT_VERSION: u32 = 1;

/// One reversible document change. Paths may be indexed paths or `guid:`/`id:` locators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    /// Replace character data (rejected on element-only elements).
    SetText { path: String, value: String },
    /// Replace all child nodes verbatim. Used by inverses and for non-element changes.
    SetChildren { path: String, children: Vec<Node> },
    /// Set or add an attribute; `index` places a newly added attribute.
    SetAttribute {
        path: String,
        name: String,
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    /// Remove an attribute; `force` bypasses the required-attribute check.
    RemoveAttribute {
        path: String,
        name: String,
        #[serde(default)]
        force: bool,
    },
    /// Insert a subtree. `position` is the index among same-name siblings;
    /// `at` is an exact child-node index and takes precedence.
    InsertChild {
        parent: String,
        node: Element,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        position: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<usize>,
    },
    /// Remove a subtree, honoring minimum cardinality for declared children.
    RemoveChild { path: String },
    /// Reparent or reorder; `position` is the final same-name index at the destination.
    Move {
        path: String,
        new_parent: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        position: Option<usize>,
    },
}

/// A versioned list of edits, applied in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditBatch {
    pub version: u32,
    pub edits: Vec<Edit>,
}

impl EditBatch {
    #[must_use]
    pub fn new(edits: Vec<Edit>) -> Self {
        Self {
            version: EDIT_FORMAT_VERSION,
            edits,
        }
    }
}

impl Document {
    /// Apply one edit and return its inverse. The document is unchanged on error.
    pub fn apply(&mut self, edit: &Edit) -> Result<Edit> {
        match edit {
            Edit::SetText { path, value } => {
                let path = self.path_of(path)?;
                let prior = self.element(&path)?.children.clone();
                self.set_text(&path, value)?;
                Ok(Edit::SetChildren {
                    path,
                    children: prior,
                })
            }
            Edit::SetChildren { path, children } => {
                let path = self.path_of(path)?;
                let mut replacement = Element::new("canonical", None);
                replacement.children = children.clone();
                normalize_empty_text(&mut replacement);
                let element = self.element_mut(&path)?;
                let prior = std::mem::replace(&mut element.children, replacement.children);
                Ok(Edit::SetChildren {
                    path,
                    children: prior,
                })
            }
            Edit::SetAttribute {
                path,
                name,
                value,
                index,
            } => {
                let path = self.path_of(path)?;
                let prior = self.attribute_position(&path, name)?;
                let prior_value = prior.map(|position| {
                    self.element(&path)
                        .map(|e| e.attributes[position].value.clone())
                        .unwrap_or_default()
                });
                self.set_attribute(&path, name, value)?;
                if let (None, Some(index)) = (prior, index) {
                    let element = self.element_mut(&path)?;
                    let attribute = element.attributes.pop().expect("attribute was just added");
                    let index = (*index).min(element.attributes.len());
                    element.attributes.insert(index, attribute);
                }
                Ok(match prior_value {
                    Some(old) => Edit::SetAttribute {
                        path,
                        name: name.clone(),
                        value: old,
                        index: None,
                    },
                    None => Edit::RemoveAttribute {
                        path,
                        name: name.clone(),
                        force: true,
                    },
                })
            }
            Edit::RemoveAttribute { path, name, force } => {
                let path = self.path_of(path)?;
                let position = self
                    .attribute_position(&path, name)?
                    .ok_or_else(|| Error::PathNotFound(format!("{path}/@{name}")))?;
                let (qualified, value) = {
                    let attribute = &self.element(&path)?.attributes[position];
                    (attribute.qualified_name.clone(), attribute.value.clone())
                };
                if *force {
                    self.remove_attribute_unchecked(&path, name)?;
                } else {
                    self.remove_attribute(&path, name)?;
                }
                Ok(Edit::SetAttribute {
                    path,
                    name: qualified,
                    value,
                    index: Some(position),
                })
            }
            Edit::InsertChild {
                parent,
                node,
                position,
                at,
            } => {
                let parent = self.path_of(parent)?;
                let path = self.insert_checked(&parent, node.clone(), *position, true)?;
                let path = match at {
                    Some(at) => self.relocate_child(&parent, &path, *at)?,
                    None => path,
                };
                Ok(Edit::RemoveChild { path })
            }
            Edit::RemoveChild { path } => {
                let path = self.path_of(path)?;
                let (parent_path, segment) = split_parent(&path)?;
                let node = self.element(&path)?.clone();
                let at = {
                    let parent = self.element(&parent_path)?;
                    child_element_position(parent, &segment)
                        .ok_or_else(|| Error::PathNotFound(path.clone()))?
                };
                let declared = self
                    .schema_rule(&parent_path)
                    .ok()
                    .is_some_and(|rule| rule.child(&segment.name).is_some());
                if declared {
                    self.remove_schema_node(&path)?;
                } else {
                    self.remove(&path)?;
                }
                Ok(Edit::InsertChild {
                    parent: parent_path,
                    node,
                    position: None,
                    at: Some(at),
                })
            }
            Edit::Move {
                path,
                new_parent,
                position,
            } => {
                let path = self.path_of(path)?;
                let (old_parent, segment) = split_parent(&path)?;
                let new_path = self.reparent_schema_node(&path, new_parent, *position)?;
                // Inserting at the destination shifts later same-name siblings
                // there; the old parent may be one of them (or below one).
                let old_parent = adjust_path_after_insertion(&old_parent, &new_path)?;
                Ok(Edit::Move {
                    path: new_path,
                    new_parent: old_parent,
                    position: Some(segment.index),
                })
            }
        }
    }

    /// Apply a batch atomically and return the inverse batch (reverse order).
    pub fn apply_batch(&mut self, batch: &EditBatch) -> Result<EditBatch> {
        if batch.version != EDIT_FORMAT_VERSION {
            return Err(Error::Json(format!(
                "unsupported edit format version {}; expected {EDIT_FORMAT_VERSION}",
                batch.version
            )));
        }
        let mut work = self.clone();
        let mut inverses = Vec::with_capacity(batch.edits.len());
        for edit in &batch.edits {
            inverses.push(work.apply(edit)?);
        }
        inverses.reverse();
        *self = work;
        Ok(EditBatch::new(inverses))
    }

    /// Best-effort edits that turn `self` into `other`.
    ///
    /// Covers attributes, character data, comments/CDATA/PIs (as whole-child
    /// replacement of the affected element) and element insertions/removals
    /// matched by name and same-name index. Whitespace-only text is ignored.
    pub fn diff(&self, other: &Document) -> Result<EditBatch> {
        if self.root.local_name != other.root.local_name {
            return Err(Error::Content(format!(
                "root elements differ: `{}` vs `{}`",
                self.root.local_name, other.root.local_name
            )));
        }
        let mut edits = Vec::new();
        diff_element(&self.root, &other.root, "/idm", &mut edits);
        Ok(EditBatch::new(edits))
    }

    fn attribute_position(&self, path: &str, name: &str) -> Result<Option<usize>> {
        let element = self.element(path)?;
        Ok(element
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
            }))
    }

    /// Move a just-inserted child to an exact child-node index.
    fn relocate_child(&mut self, parent_path: &str, path: &str, at: usize) -> Result<String> {
        let segments = parse_path(path)?;
        let segment = segments.last().expect("non-empty path").clone();
        let parent = self.element_mut(parent_path)?;
        let from = child_element_position(parent, &segment)
            .ok_or_else(|| Error::PathNotFound(path.into()))?;
        let node = parent.children.remove(from);
        let at = at.min(parent.children.len());
        parent.children.insert(at, node);
        let index = parent.children[..at]
            .iter()
            .filter(|n| matches!(n, Node::Element(e) if e.local_name == segment.name))
            .count();
        canonical_path(&format!("{parent_path}/{}[{index}]", segment.name))
    }
}

/// Re-express `path` after an element was inserted at `inserted` (a canonical
/// path): same-name siblings at or after the insertion index shift up by one.
fn adjust_path_after_insertion(path: &str, inserted: &str) -> Result<String> {
    let mut segments = parse_path(path)?;
    let inserted = parse_path(inserted)?;
    let depth = inserted.len() - 1;
    let shares_parent = segments.len() > depth
        && segments[..depth]
            .iter()
            .zip(&inserted[..depth])
            .all(|(a, b)| a.name == b.name && a.index == b.index);
    if shares_parent {
        let segment = &mut segments[depth];
        let target = &inserted[depth];
        if segment.name == target.name && segment.index >= target.index {
            segment.index += 1;
        }
    }
    canonical_path(&format_path(&segments))
}

fn split_parent(path: &str) -> Result<(String, PathSegment)> {
    let mut segments = parse_path(path)?;
    if segments.len() <= 1 {
        return Err(Error::InvalidPath(
            "the IDM root cannot be edited this way".into(),
        ));
    }
    let segment = segments.pop().expect("checked non-empty");
    Ok((canonical_path(&format_path(&segments))?, segment))
}

fn significant_non_elements(element: &Element) -> Vec<&Node> {
    element
        .children
        .iter()
        .filter(|node| match node {
            Node::Element(_) => false,
            Node::Text(text) => !text.trim().is_empty(),
            _ => true,
        })
        .collect()
}

fn diff_element(a: &Element, b: &Element, path: &str, edits: &mut Vec<Edit>) {
    for attribute in &b.attributes {
        let current = a
            .attributes
            .iter()
            .find(|candidate| candidate.qualified_name == attribute.qualified_name);
        if current.is_none_or(|current| current.value != attribute.value) {
            edits.push(Edit::SetAttribute {
                path: path.into(),
                name: attribute.qualified_name.clone(),
                value: attribute.value.clone(),
                index: None,
            });
        }
    }
    for attribute in &a.attributes {
        if !b
            .attributes
            .iter()
            .any(|candidate| candidate.qualified_name == attribute.qualified_name)
        {
            edits.push(Edit::RemoveAttribute {
                path: path.into(),
                name: attribute.qualified_name.clone(),
                force: true,
            });
        }
    }
    if significant_non_elements(a) != significant_non_elements(b) {
        edits.push(Edit::SetChildren {
            path: path.into(),
            children: b.children.clone(),
        });
        return;
    }
    let mut names: Vec<&str> = Vec::new();
    for element in a
        .children
        .iter()
        .chain(&b.children)
        .filter_map(|node| match node {
            Node::Element(e) => Some(e),
            _ => None,
        })
    {
        if !names.contains(&element.local_name.as_str()) {
            names.push(&element.local_name);
        }
    }
    for name in names {
        let left: Vec<&Element> = child_elements(a, name).collect();
        let right: Vec<&Element> = child_elements(b, name).collect();
        let common = left.len().min(right.len());
        for index in 0..common {
            diff_element(
                left[index],
                right[index],
                &format!("{path}/{name}[{index}]"),
                edits,
            );
        }
        for index in (common..left.len()).rev() {
            edits.push(Edit::RemoveChild {
                path: format!("{path}/{name}[{index}]"),
            });
        }
        for (index, node) in right.iter().enumerate().skip(common) {
            edits.push(Edit::InsertChild {
                parent: path.into(),
                node: (*node).clone(),
                position: Some(index),
                at: None,
            });
        }
    }
}

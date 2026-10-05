//! Structural editing beyond append/remove: positional insert, duplicate,
//! paste and reparent, all cardinality- and order-checked, with `guid`/`id`
//! regeneration so copies never collide.

use super::*;

impl Document {
    /// Add a schema child at `position` among its same-name siblings.
    ///
    /// `position == current count` appends. Returns the new element's path.
    pub fn insert_schema_child(
        &mut self,
        parent_path: &str,
        name: &str,
        position: usize,
    ) -> Result<String> {
        let parent_path = self.resolve_locator(parent_path)?.into_owned();
        let current = child_elements(self.element(&parent_path)?, name).count();
        if position > current {
            return Err(Error::InvalidPath(format!(
                "position {position} is beyond the {current} existing `{name}` sibling(s)"
            )));
        }
        let appended = self.append_schema_child(&parent_path, name)?;
        if position == current {
            return canonical_path(&appended);
        }
        let target = format!("{parent_path}/{name}[{position}]");
        canonical_path(&self.move_schema_node(&appended, &target, false)?)
    }

    /// Insert a copy of `node` below `parent_path`, checking that the schema
    /// allows it there and that maximum cardinality is not exceeded.
    ///
    /// Colliding `guid`/`id` values inside the copy are regenerated (and
    /// `changedBy` references inside the copy remapped). `position` is the
    /// index among same-name siblings; `None` places it in schema order.
    pub fn paste_schema_node(
        &mut self,
        parent_path: &str,
        mut node: Element,
        position: Option<usize>,
    ) -> Result<String> {
        let parent_path = self.resolve_locator(parent_path)?.into_owned();
        self.regenerate_colliding_ids(&mut node);
        self.insert_checked(&parent_path, node, position, false)
    }

    /// Duplicate the element at `path` immediately after itself, with fresh
    /// `guid`/`id` values. Returns the duplicate's path.
    pub fn duplicate_schema_node(&mut self, path: &str) -> Result<String> {
        let path = self.resolve_locator(path)?.into_owned();
        let mut segments = parse_path(&path)?;
        if segments.len() <= 1 {
            return Err(Error::InvalidPath(
                "the IDM root cannot be duplicated".into(),
            ));
        }
        let source = segments.pop().expect("checked non-empty");
        let parent_path = format_path(&segments);
        let node = self.element(&path)?.clone();
        let mut copy = node;
        self.regenerate_colliding_ids(&mut copy);
        self.insert_checked(&parent_path, copy, Some(source.index + 1), true)
    }

    /// Move an element below another parent that the schema allows it under.
    ///
    /// The element keeps its identity (`guid`/`id` are not regenerated). The
    /// target must declare the same definition for the child, the source must
    /// stay above its minimum cardinality, and the target must be below its
    /// maximum. The document is unchanged if any check fails. `position` is the
    /// index among same-name siblings at the destination; `None` appends.
    pub fn reparent_schema_node(
        &mut self,
        path: &str,
        new_parent_path: &str,
        position: Option<usize>,
    ) -> Result<String> {
        let path = self.resolve_locator(path)?.into_owned();
        let new_parent_path = self.resolve_locator(new_parent_path)?.into_owned();
        let mut source_segments = parse_path(&path)?;
        if source_segments.len() <= 1 {
            return Err(Error::InvalidPath("the IDM root cannot be moved".into()));
        }
        let source = source_segments.pop().expect("checked non-empty");
        let old_parent_path = format_path(&source_segments);
        if new_parent_path == old_parent_path {
            return self.reorder_within_parent(&path, &source, position);
        }
        if new_parent_path == path || new_parent_path.starts_with(&format!("{path}/")) {
            return Err(Error::Content(
                "an element cannot be moved below itself".into(),
            ));
        }
        let catalog = cached_catalog()?;
        let source_handle = schema_handle_for_path(self, catalog, &path)?;
        let target_handle = schema_handle_for_path(self, catalog, &new_parent_path)?;
        let target_rule = catalog
            .element(&target_handle)
            .ok_or_else(|| Error::Schema(format!("missing schema definition `{target_handle}`")))?;
        let child_rule = target_rule.child(&source.name).ok_or_else(|| {
            Error::Schema(format!(
                "`{}` is not allowed below `{target_handle}`",
                source.name
            ))
        })?;
        if child_rule.definition != source_handle {
            return Err(Error::Schema(format!(
                "`{}` below `{target_handle}` has a different definition than `{source_handle}`",
                source.name
            )));
        }
        let node = self.element(&path)?.clone();
        let mut work = self.clone();
        work.remove_schema_node(&path)?;
        let adjusted = adjust_path_after_removal(&new_parent_path, &source_segments, &source)?;
        let new_path = work.insert_checked(&adjusted, node, position, false)?;
        *self = work;
        Ok(new_path)
    }

    fn reorder_within_parent(
        &mut self,
        path: &str,
        source: &PathSegment,
        position: Option<usize>,
    ) -> Result<String> {
        let parent_path = path.rsplit_once('/').map_or("", |(parent, _)| parent);
        let parent = self.element(parent_path)?;
        let count = child_elements(parent, &source.name).count();
        let (target_index, after) = match position {
            Some(index) if index < count => (index, false),
            Some(index) if index == count => (count - 1, true),
            Some(index) => {
                return Err(Error::InvalidPath(format!(
                    "position {index} is beyond the {count} existing `{}` sibling(s)",
                    source.name
                )));
            }
            None => (count - 1, true),
        };
        let target = format!("{parent_path}/{}[{target_index}]", source.name);
        canonical_path(&self.move_schema_node(path, &target, after)?)
    }

    /// Shared insertion: schema membership, maximum cardinality and placement.
    ///
    /// Undeclared (extension) names are rejected unless `allow_undeclared`.
    pub(crate) fn insert_checked(
        &mut self,
        parent_path: &str,
        node: Element,
        position: Option<usize>,
        allow_undeclared: bool,
    ) -> Result<String> {
        let catalog = cached_catalog()?;
        let parent_handle = schema_handle_for_path(self, catalog, parent_path)?;
        let parent_rule = catalog
            .element(&parent_handle)
            .ok_or_else(|| Error::Schema(format!("missing schema definition `{parent_handle}`")))?;
        let name = node.local_name.clone();
        let declared = parent_rule.child(&name);
        let current = child_elements(self.element(parent_path)?, &name).count();
        if let Some(rule) = declared {
            if rule.max_occurs.is_some_and(|maximum| current >= maximum) {
                return Err(Error::Cardinality(format!(
                    "`{name}` already reached maximum cardinality {} below `{parent_path}`",
                    rule.max_occurs.expect("checked")
                )));
            }
        } else if !allow_undeclared {
            return Err(Error::Schema(format!(
                "`{name}` is not allowed below `{parent_handle}`"
            )));
        }
        if position.is_some_and(|index| index > current) {
            return Err(Error::InvalidPath(format!(
                "position {} is beyond the {current} existing `{name}` sibling(s)",
                position.expect("checked")
            )));
        }
        let parent = self.element_mut(parent_path)?;
        match position {
            Some(index) if index < current => {
                let target = PathSegment {
                    name: name.clone(),
                    index,
                };
                let at = child_element_position(parent, &target).expect("index below count");
                parent.children.insert(at, Node::Element(node));
            }
            Some(_) if current > 0 => {
                let last = PathSegment {
                    name: name.clone(),
                    index: current - 1,
                };
                let at = child_element_position(parent, &last).expect("current above zero");
                parent.children.insert(at + 1, Node::Element(node));
            }
            _ => insert_schema_ordered(parent, Node::Element(node), parent_rule),
        }
        let index = position.unwrap_or(current).min(current);
        canonical_path(&format!("{parent_path}/{name}[{index}]"))
    }

    fn regenerate_colliding_ids(&self, node: &mut Element) {
        let mut guids = BTreeSet::new();
        let mut ids = BTreeSet::new();
        walk_elements(&self.root, "/idm", &mut |element, _| {
            if let Some(guid) = attribute_by_local(element, "guid") {
                guids.insert(guid.to_owned());
            }
            if let Some(id) = attribute_by_local(element, "id") {
                ids.insert(id.to_owned());
            }
        });
        let mut remap = BTreeMap::new();
        regenerate_in(node, &mut guids, &mut ids, &mut remap);
        remap_references(node, &remap);
    }
}

fn regenerate_in(
    element: &mut Element,
    guids: &mut BTreeSet<String>,
    ids: &mut BTreeSet<String>,
    remap: &mut BTreeMap<String, String>,
) {
    for attribute in &mut element.attributes {
        if attribute.prefix.is_some() {
            continue;
        }
        match attribute.local_name.as_str() {
            "guid" => {
                if !guids.insert(attribute.value.clone()) {
                    attribute.value = Uuid::new_v4().to_string();
                    guids.insert(attribute.value.clone());
                }
            }
            "id" => {
                if !ids.insert(attribute.value.clone()) {
                    let fresh = format!("id-{}", Uuid::new_v4().simple());
                    remap.insert(attribute.value.clone(), fresh.clone());
                    attribute.value = fresh;
                    ids.insert(attribute.value.clone());
                }
            }
            _ => {}
        }
    }
    for child in &mut element.children {
        if let Node::Element(child) = child {
            regenerate_in(child, guids, ids, remap);
        }
    }
}

fn remap_references(element: &mut Element, remap: &BTreeMap<String, String>) {
    if remap.is_empty() {
        return;
    }
    for attribute in &mut element.attributes {
        if attribute.local_name == "changedBy" {
            if let Some(fresh) = remap.get(&attribute.value) {
                attribute.value = fresh.clone();
            }
        }
    }
    for child in &mut element.children {
        if let Node::Element(child) = child {
            remap_references(child, remap);
        }
    }
}

/// Re-express `target_parent` after `source` was removed from its parent
/// (`source_parent` segments): same-name siblings after it shift down by one.
fn adjust_path_after_removal(
    target_parent: &str,
    source_parent: &[PathSegment],
    source: &PathSegment,
) -> Result<String> {
    let mut segments = parse_path(target_parent)?;
    let depth = source_parent.len();
    let shares_prefix = segments.len() > depth
        && segments[..depth]
            .iter()
            .zip(source_parent)
            .all(|(a, b)| a.name == b.name && a.index == b.index);
    if shares_prefix {
        let segment = &mut segments[depth];
        if segment.name == source.name && segment.index > source.index {
            segment.index -= 1;
        }
    }
    Ok(format_path(&segments))
}

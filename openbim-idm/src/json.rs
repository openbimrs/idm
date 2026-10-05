//! Depth-safe JSON input.
//!
//! The lossless tree JSON nests about three levels per XML element, so a
//! document at the XML depth limit exceeds serde_json's default recursion
//! limit of 128. JSON input is therefore pre-scanned against an explicit
//! nesting bound and then parsed without serde_json's limit, and element trees
//! are rebuilt iteratively so deep input cannot exhaust small thread stacks.

use super::*;
use serde::de::DeserializeOwned;

/// JSON nesting allowed for documents and edits: enough for an element tree
/// at [`DEFAULT_MAX_XML_DEPTH`] plus the surrounding document/edit wrappers.
pub(crate) const MAX_JSON_NESTING: usize = DEFAULT_MAX_XML_DEPTH * 3 + 16;

/// Parse JSON text into `T` after bounding its nesting depth.
pub(crate) fn from_json_str<T: DeserializeOwned>(text: &str) -> Result<T> {
    let depth = json_nesting(text);
    if depth > MAX_JSON_NESTING {
        return Err(Error::MaxDepthExceeded {
            maximum: DEFAULT_MAX_XML_DEPTH,
        });
    }
    let mut deserializer = serde_json::Deserializer::from_str(text);
    deserializer.disable_recursion_limit();
    let value =
        T::deserialize(&mut deserializer).map_err(|error| Error::Json(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| Error::Json(error.to_string()))?;
    Ok(value)
}

/// Maximum bracket nesting outside string literals (malformed input is
/// rejected later by the parser; this only bounds recursion).
fn json_nesting(text: &str) -> usize {
    let (mut depth, mut maximum) = (0usize, 0usize);
    let (mut in_string, mut escaped) = (false, false);
    for byte in text.bytes() {
        if in_string {
            match byte {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                maximum = maximum.max(depth);
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    maximum
}

impl<'de> Deserialize<'de> for Element {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        element_from_value(&value).map_err(serde::de::Error::custom)
    }
}

/// Rebuild an element tree from its JSON form without recursion.
pub(crate) fn element_from_value(value: &Value) -> std::result::Result<Element, String> {
    struct Frame<'a> {
        element: Element,
        children: &'a [Value],
        next: usize,
    }

    fn frame(value: &Value) -> std::result::Result<Frame<'_>, String> {
        let object = value.as_object().ok_or("element must be an object")?;
        let text = |key: &str| -> std::result::Result<String, String> {
            object
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("element field `{key}` must be a string"))
        };
        let optional = |key: &str| -> std::result::Result<Option<String>, String> {
            match object.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::String(text)) => Ok(Some(text.clone())),
                Some(_) => Err(format!("element field `{key}` must be a string or null")),
            }
        };
        let attributes = object
            .get("attributes")
            .ok_or("missing field `attributes`")?;
        let attributes: Vec<Attribute> =
            serde_json::from_value(attributes.clone()).map_err(|error| error.to_string())?;
        let children = object
            .get("children")
            .ok_or("missing field `children`")?
            .as_array()
            .ok_or("element field `children` must be an array")?;
        Ok(Frame {
            element: Element {
                qualified_name: text("qualified_name")?,
                local_name: text("local_name")?,
                prefix: optional("prefix")?,
                namespace: optional("namespace")?,
                attributes,
                children: Vec::with_capacity(children.len()),
            },
            children,
            next: 0,
        })
    }

    let mut stack = vec![frame(value)?];
    loop {
        let top = stack.last_mut().expect("stack is never empty here");
        if let Some(child) = top.children.get(top.next) {
            top.next += 1;
            let kind = child
                .get("kind")
                .and_then(Value::as_str)
                .ok_or("node needs a `kind`")?;
            if kind == "element" {
                if stack.len() >= DEFAULT_MAX_XML_DEPTH {
                    return Err(Error::MaxDepthExceeded {
                        maximum: DEFAULT_MAX_XML_DEPTH,
                    }
                    .to_string());
                }
                let inner = child.get("value").ok_or("element node needs a `value`")?;
                stack.push(frame(inner)?);
            } else {
                let node: Node =
                    serde_json::from_value(child.clone()).map_err(|error| error.to_string())?;
                top.element.children.push(node);
            }
        } else {
            let done = stack.pop().expect("checked non-empty").element;
            match stack.last_mut() {
                Some(parent) => parent.element.children.push(Node::Element(done)),
                None => return Ok(done),
            }
        }
    }
}

impl Document {
    /// Rebuild a document from lossless tree JSON text (the format of
    /// [`Document::to_value`]), accepting documents up to the XML depth limit.
    pub fn from_json_str(text: &str) -> Result<Self> {
        let value: Value = from_json_str(text)?;
        Self::from_value(&value)
    }
}

impl EditBatch {
    /// Parse a JSON edit batch, accepting subtrees up to the XML depth limit.
    pub fn from_json_str(text: &str) -> Result<Self> {
        from_json_str(text)
    }
}

impl Edit {
    /// Parse one JSON edit, accepting subtrees up to the XML depth limit.
    pub fn from_json_str(text: &str) -> Result<Self> {
        from_json_str(text)
    }
}

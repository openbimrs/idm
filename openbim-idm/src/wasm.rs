//! WebAssembly binding (`--features wasm`, target `wasm32-unknown-unknown`).
//!
//! The surface mirrors the Python facade: one `Document` class, structured
//! values exchanged as JSON strings (`*Json` methods), and errors thrown as
//! `Error` objects carrying a stable `code`. Importing the generated module as
//! a namespace (`import * as idmxml from "./idmxml.js"`) keeps every export
//! under that namespace; nothing is installed on `globalThis`.

use crate::{Document, Edit, EditBatch, Element, Encoding, Error, cached_catalog};
use js_sys::{Object, Reflect};
use serde::Serialize;
use wasm_bindgen::prelude::*;

type JsResult<T> = std::result::Result<T, JsValue>;

fn js_error(error: Error) -> JsValue {
    let value = js_sys::Error::new(&error.to_string());
    let _ = Reflect::set(&value, &"code".into(), &error.code().into());
    value.into()
}

fn json_error(error: serde_json::Error) -> JsValue {
    js_error(Error::Json(error.to_string()))
}

fn to_json<T: Serialize>(value: &T) -> JsResult<String> {
    serde_json::to_string(value).map_err(json_error)
}

/// A lossless, schema-aware IDM document.
#[wasm_bindgen(js_name = Document)]
#[derive(Clone)]
pub struct WasmDocument {
    inner: Document,
}

/// Result of decoding bytes: the document plus encoding metadata (JSON).
#[wasm_bindgen(js_name = LoadResult)]
pub struct WasmLoadResult {
    document: WasmDocument,
    metadata: String,
}

#[wasm_bindgen(js_class = LoadResult)]
impl WasmLoadResult {
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> WasmDocument {
        self.document.clone()
    }

    /// `{ encoding, declared_encoding, had_bom, warnings }` as JSON.
    #[wasm_bindgen(getter, js_name = metadataJson)]
    pub fn metadata_json(&self) -> String {
        self.metadata.clone()
    }
}

#[wasm_bindgen(js_class = Document)]
impl WasmDocument {
    pub fn parse(xml: &str) -> JsResult<WasmDocument> {
        Ok(Self {
            inner: Document::parse(xml).map_err(js_error)?,
        })
    }

    /// Decode bytes (UTF-8/16, ISO-8859-1, windows-1252; BOM aware).
    #[wasm_bindgen(js_name = parseBytes)]
    pub fn parse_bytes(bytes: &[u8]) -> JsResult<WasmLoadResult> {
        let parsed = Document::parse_bytes(bytes).map_err(js_error)?;
        let metadata = serde_json::json!({
            "encoding": parsed.encoding,
            "declared_encoding": parsed.declared_encoding,
            "had_bom": parsed.had_bom,
            "warnings": parsed.warnings,
        })
        .to_string();
        Ok(WasmLoadResult {
            document: Self {
                inner: parsed.document,
            },
            metadata,
        })
    }

    /// Create a complete IDM skeleton.
    #[wasm_bindgen(js_name = create)]
    pub fn create(full_title: &str, idm_code: &str) -> JsResult<WasmDocument> {
        Ok(Self {
            inner: Document::new_idm(full_title, idm_code).map_err(js_error)?,
        })
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: &str) -> JsResult<WasmDocument> {
        let value = serde_json::from_str(value).map_err(json_error)?;
        Ok(Self {
            inner: Document::from_value(&value).map_err(js_error)?,
        })
    }

    pub fn copy(&self) -> WasmDocument {
        self.clone()
    }

    pub fn equals(&self, other: &WasmDocument) -> bool {
        self.inner == other.inner
    }

    #[wasm_bindgen(getter, js_name = rootName)]
    pub fn root_name(&self) -> String {
        self.inner.root().local_name().to_owned()
    }

    #[wasm_bindgen(js_name = toXml)]
    pub fn to_xml(&self, pretty: Option<bool>) -> JsResult<String> {
        self.inner.to_xml(pretty.unwrap_or(true)).map_err(js_error)
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self, pretty: Option<bool>, encoding: Option<String>) -> JsResult<Vec<u8>> {
        let encoding =
            Encoding::from_label(encoding.as_deref().unwrap_or("utf-8")).map_err(js_error)?;
        self.inner
            .to_bytes_with_encoding(pretty.unwrap_or(true), encoding)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> JsResult<String> {
        to_json(&self.inner.to_value())
    }

    #[wasm_bindgen(js_name = validateJson)]
    pub fn validate_json(&self) -> JsResult<String> {
        to_json(&self.inner.validate())
    }

    pub fn count(&self, name: &str) -> usize {
        self.inner.count(name)
    }

    #[wasm_bindgen(js_name = elementPaths)]
    pub fn element_paths(&self, name: &str) -> Vec<String> {
        self.inner.element_paths(name)
    }

    #[wasm_bindgen(js_name = findByGuid)]
    pub fn find_by_guid(&self, guid: &str) -> Vec<String> {
        self.inner.find_by_guid(guid)
    }

    #[wasm_bindgen(js_name = findById)]
    pub fn find_by_id(&self, id: &str) -> Vec<String> {
        self.inner.find_by_id(id)
    }

    #[wasm_bindgen(js_name = pathOf)]
    pub fn path_of(&self, locator: &str) -> JsResult<String> {
        self.inner.path_of(locator).map_err(js_error)
    }

    pub fn text(&self, path: &str) -> JsResult<String> {
        self.inner.text(path).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setText)]
    pub fn set_text(&mut self, path: &str, value: &str) -> JsResult<()> {
        self.inner.set_text(path, value).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setTextUnchecked)]
    pub fn set_text_unchecked(&mut self, path: &str, value: &str) -> JsResult<()> {
        self.inner.set_text_unchecked(path, value).map_err(js_error)
    }

    pub fn attribute(&self, path: &str, name: &str) -> JsResult<String> {
        self.inner.attribute(path, name).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setAttribute)]
    pub fn set_attribute(&mut self, path: &str, name: &str, value: &str) -> JsResult<()> {
        self.inner
            .set_attribute(path, name, value)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = removeAttribute)]
    pub fn remove_attribute(&mut self, path: &str, name: &str) -> JsResult<()> {
        self.inner.remove_attribute(path, name).map_err(js_error)
    }

    #[wasm_bindgen(js_name = addSchemaAttribute)]
    pub fn add_schema_attribute(&mut self, path: &str, name: &str) -> JsResult<String> {
        self.inner
            .add_schema_attribute(path, name)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = attributesJson)]
    pub fn attributes_json(&self, path: &str) -> JsResult<String> {
        to_json(&self.inner.attributes(path).map_err(js_error)?)
    }

    #[wasm_bindgen(js_name = attributeSlotsJson)]
    pub fn attribute_slots_json(&self, path: &str) -> JsResult<String> {
        to_json(&self.inner.attribute_slots(path).map_err(js_error)?)
    }

    #[wasm_bindgen(js_name = nodeJson)]
    pub fn node_json(&self, path: &str) -> JsResult<String> {
        to_json(&self.inner.node_info(path).map_err(js_error)?)
    }

    #[wasm_bindgen(js_name = schemaRuleJson)]
    pub fn schema_rule_json(&self, path: &str) -> JsResult<String> {
        to_json(self.inner.schema_rule(path).map_err(js_error)?)
    }

    #[wasm_bindgen(js_name = elementJson)]
    pub fn element_json(&self, path: &str) -> JsResult<String> {
        to_json(self.inner.element(path).map_err(js_error)?)
    }

    #[wasm_bindgen(js_name = allowedChildrenJson)]
    pub fn allowed_children_json(&self, parent_path: &str) -> JsResult<String> {
        to_json(&self.inner.allowed_children(parent_path).map_err(js_error)?)
    }

    #[wasm_bindgen(js_name = appendSchemaChild)]
    pub fn append_schema_child(&mut self, parent_path: &str, name: &str) -> JsResult<String> {
        self.inner
            .append_schema_child(parent_path, name)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = insertSchemaChild)]
    pub fn insert_schema_child(
        &mut self,
        parent_path: &str,
        name: &str,
        position: usize,
    ) -> JsResult<String> {
        self.inner
            .insert_schema_child(parent_path, name, position)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = removeSchemaNode)]
    pub fn remove_schema_node(&mut self, path: &str) -> JsResult<()> {
        self.inner.remove_schema_node(path).map_err(js_error)
    }

    #[wasm_bindgen(js_name = duplicateSchemaNode)]
    pub fn duplicate_schema_node(&mut self, path: &str) -> JsResult<String> {
        self.inner.duplicate_schema_node(path).map_err(js_error)
    }

    #[wasm_bindgen(js_name = moveSchemaNode)]
    pub fn move_schema_node(
        &mut self,
        path: &str,
        target_path: &str,
        after: bool,
    ) -> JsResult<String> {
        self.inner
            .move_schema_node(path, target_path, after)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = reparentSchemaNode)]
    pub fn reparent_schema_node(
        &mut self,
        path: &str,
        new_parent_path: &str,
        position: Option<usize>,
    ) -> JsResult<String> {
        self.inner
            .reparent_schema_node(path, new_parent_path, position)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = pasteSchemaNodeJson)]
    pub fn paste_schema_node_json(
        &mut self,
        parent_path: &str,
        element_json: &str,
        position: Option<usize>,
    ) -> JsResult<String> {
        let element: Element = serde_json::from_str(element_json).map_err(json_error)?;
        self.inner
            .paste_schema_node(parent_path, element, position)
            .map_err(js_error)
    }

    /// Apply one edit (JSON); returns the inverse edit as JSON.
    #[wasm_bindgen(js_name = applyJson)]
    pub fn apply_json(&mut self, edit_json: &str) -> JsResult<String> {
        let edit: Edit = serde_json::from_str(edit_json).map_err(json_error)?;
        to_json(&self.inner.apply(&edit).map_err(js_error)?)
    }

    /// Apply an edit batch (JSON) atomically; returns the inverse batch as JSON.
    #[wasm_bindgen(js_name = applyBatchJson)]
    pub fn apply_batch_json(&mut self, batch_json: &str) -> JsResult<String> {
        let batch: EditBatch = serde_json::from_str(batch_json).map_err(json_error)?;
        to_json(&self.inner.apply_batch(&batch).map_err(js_error)?)
    }

    #[wasm_bindgen(js_name = diffJson)]
    pub fn diff_json(&self, other: &WasmDocument) -> JsResult<String> {
        to_json(&self.inner.diff(&other.inner).map_err(js_error)?)
    }
}

/// The generated declaration catalog as JSON.
#[wasm_bindgen(js_name = schemaCatalogJson)]
pub fn schema_catalog_json() -> JsResult<String> {
    to_json(cached_catalog().map_err(js_error)?)
}

/// Package/engine version, useful for diagnostics in bug reports.
#[wasm_bindgen(js_name = engineVersion)]
pub fn engine_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[wasm_bindgen(js_name = errorCodes)]
pub fn error_codes() -> JsValue {
    let object = Object::new();
    for code in [
        "input_too_large",
        "max_depth_exceeded",
        "invalid_xml",
        "write_failed",
        "invalid_utf8",
        "invalid_path",
        "path_not_found",
        "schema",
        "invalid_json",
        "cardinality",
        "content_model",
        "io",
        "encoding",
    ] {
        let _ = Reflect::set(&object, &code.into(), &code.into());
    }
    object.into()
}

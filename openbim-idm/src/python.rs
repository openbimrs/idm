use crate::{
    Document, Edit, EditBatch, Element, Encoding, Error, cached_catalog, schema_text,
    verify_schema_dir,
};
use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde::Serialize;

create_exception!(
    _native,
    IdmError,
    PyValueError,
    "Base class for idmxml errors."
);
create_exception!(
    _native,
    XmlError,
    IdmError,
    "Malformed, oversized or too deeply nested XML."
);
create_exception!(
    _native,
    PathError,
    IdmError,
    "Invalid or unresolvable path/locator."
);
create_exception!(
    _native,
    SchemaError,
    IdmError,
    "Catalog or schema-directory problem."
);
create_exception!(
    _native,
    JsonError,
    IdmError,
    "Invalid JSON tree or edit document."
);
create_exception!(
    _native,
    CardinalityError,
    IdmError,
    "Schema cardinality violation."
);
create_exception!(
    _native,
    ContentModelError,
    IdmError,
    "Content model violation."
);
create_exception!(
    _native,
    EncodingError,
    IdmError,
    "Unsupported or unrepresentable encoding."
);
create_exception!(_native, IdmIoError, IdmError, "File or stream failure.");

fn py_error(error: Error) -> PyErr {
    let code = error.code();
    let message = error.to_string();
    let err = match code {
        "input_too_large" | "max_depth_exceeded" | "invalid_xml" | "write_failed"
        | "invalid_utf8" => XmlError::new_err(message),
        "invalid_path" | "path_not_found" => PathError::new_err(message),
        "schema" => SchemaError::new_err(message),
        "invalid_json" => JsonError::new_err(message),
        "cardinality" => CardinalityError::new_err(message),
        "content_model" => ContentModelError::new_err(message),
        "encoding" => EncodingError::new_err(message),
        "io" => IdmIoError::new_err(message),
        _ => IdmError::new_err(message),
    };
    Python::attach(|py| {
        let _ = err.value(py).setattr("code", code);
    });
    err
}

fn json_error(error: serde_json::Error) -> PyErr {
    py_error(Error::Json(error.to_string()))
}

fn to_json<T: Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(json_error)
}

#[pyclass(name = "Document", module = "idmxml._native", eq)]
#[derive(PartialEq)]
pub struct PyDocument {
    inner: Document,
}

#[pymethods]
impl PyDocument {
    #[staticmethod]
    fn parse(xml: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Document::parse(xml).map_err(py_error)?,
        })
    }

    /// Decode bytes (BOM/declaration aware) and return `(document, metadata_json)`.
    #[staticmethod]
    fn parse_bytes(bytes: &[u8]) -> PyResult<(Self, String)> {
        let parsed = Document::parse_bytes(bytes).map_err(py_error)?;
        let metadata = serde_json::json!({
            "encoding": parsed.encoding,
            "declared_encoding": parsed.declared_encoding,
            "had_bom": parsed.had_bom,
            "warnings": parsed.warnings,
        });
        Ok((
            Self {
                inner: parsed.document,
            },
            metadata.to_string(),
        ))
    }

    #[staticmethod]
    fn new(full_title: &str, idm_code: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Document::new_idm(full_title, idm_code).map_err(py_error)?,
        })
    }

    #[staticmethod]
    fn from_json(value: &str) -> PyResult<Self> {
        let value = serde_json::from_str(value).map_err(json_error)?;
        Ok(Self {
            inner: Document::from_value(&value).map_err(py_error)?,
        })
    }

    fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    fn __copy__(&self) -> Self {
        self.copy()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.copy()
    }

    #[getter]
    fn root_name(&self) -> &str {
        self.inner.root().local_name()
    }

    #[getter]
    fn namespace(&self) -> Option<&str> {
        self.inner.root().namespace_uri()
    }

    #[pyo3(signature = (pretty=true))]
    fn to_xml(&self, pretty: bool) -> PyResult<String> {
        self.inner.to_xml(pretty).map_err(py_error)
    }

    #[pyo3(signature = (pretty=true, encoding="utf-8"))]
    fn to_bytes<'py>(
        &self,
        py: Python<'py>,
        pretty: bool,
        encoding: &str,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let encoding = Encoding::from_label(encoding).map_err(py_error)?;
        let bytes = self
            .inner
            .to_bytes_with_encoding(pretty, encoding)
            .map_err(py_error)?;
        Ok(PyBytes::new(py, &bytes))
    }

    #[pyo3(signature = (pretty=false))]
    fn to_json(&self, pretty: bool) -> PyResult<String> {
        let value = self.inner.to_value();
        if pretty {
            serde_json::to_string_pretty(&value)
        } else {
            serde_json::to_string(&value)
        }
        .map_err(json_error)
    }

    fn validate_json(&self) -> PyResult<String> {
        to_json(&self.inner.validate())
    }

    fn count(&self, name: &str) -> usize {
        self.inner.count(name)
    }

    fn element_paths(&self, name: &str) -> Vec<String> {
        self.inner.element_paths(name)
    }

    fn find_by_guid(&self, guid: &str) -> Vec<String> {
        self.inner.find_by_guid(guid)
    }

    fn find_by_id(&self, id: &str) -> Vec<String> {
        self.inner.find_by_id(id)
    }

    fn path_of(&self, locator: &str) -> PyResult<String> {
        self.inner.path_of(locator).map_err(py_error)
    }

    fn text(&self, path: &str) -> PyResult<String> {
        self.inner.text(path).map_err(py_error)
    }

    fn set_text(&mut self, path: &str, value: &str) -> PyResult<()> {
        self.inner.set_text(path, value).map_err(py_error)
    }

    fn set_text_unchecked(&mut self, path: &str, value: &str) -> PyResult<()> {
        self.inner.set_text_unchecked(path, value).map_err(py_error)
    }

    fn attribute(&self, path: &str, name: &str) -> PyResult<String> {
        self.inner.attribute(path, name).map_err(py_error)
    }

    fn set_attribute(&mut self, path: &str, name: &str, value: &str) -> PyResult<()> {
        self.inner
            .set_attribute(path, name, value)
            .map_err(py_error)
    }

    fn remove_attribute(&mut self, path: &str, name: &str) -> PyResult<()> {
        self.inner.remove_attribute(path, name).map_err(py_error)
    }

    fn add_schema_attribute(&mut self, path: &str, name: &str) -> PyResult<String> {
        self.inner
            .add_schema_attribute(path, name)
            .map_err(py_error)
    }

    fn attributes_json(&self, path: &str) -> PyResult<String> {
        to_json(&self.inner.attributes(path).map_err(py_error)?)
    }

    fn attribute_slots_json(&self, path: &str) -> PyResult<String> {
        to_json(&self.inner.attribute_slots(path).map_err(py_error)?)
    }

    fn node_info_json(&self, path: &str) -> PyResult<String> {
        to_json(&self.inner.node_info(path).map_err(py_error)?)
    }

    fn schema_rule_json(&self, path: &str) -> PyResult<String> {
        to_json(self.inner.schema_rule(path).map_err(py_error)?)
    }

    fn append_schema_child(&mut self, parent_path: &str, name: &str) -> PyResult<String> {
        self.inner
            .append_schema_child(parent_path, name)
            .map_err(py_error)
    }

    fn insert_schema_child(
        &mut self,
        parent_path: &str,
        name: &str,
        position: usize,
    ) -> PyResult<String> {
        self.inner
            .insert_schema_child(parent_path, name, position)
            .map_err(py_error)
    }

    fn remove_schema_node(&mut self, path: &str) -> PyResult<()> {
        self.inner.remove_schema_node(path).map_err(py_error)
    }

    fn duplicate_schema_node(&mut self, path: &str) -> PyResult<String> {
        self.inner.duplicate_schema_node(path).map_err(py_error)
    }

    #[pyo3(signature = (path, new_parent_path, position=None))]
    fn reparent_schema_node(
        &mut self,
        path: &str,
        new_parent_path: &str,
        position: Option<usize>,
    ) -> PyResult<String> {
        self.inner
            .reparent_schema_node(path, new_parent_path, position)
            .map_err(py_error)
    }

    #[pyo3(signature = (parent_path, element_json, position=None))]
    fn paste_schema_node_json(
        &mut self,
        parent_path: &str,
        element_json: &str,
        position: Option<usize>,
    ) -> PyResult<String> {
        let element: Element = serde_json::from_str(element_json).map_err(json_error)?;
        self.inner
            .paste_schema_node(parent_path, element, position)
            .map_err(py_error)
    }

    fn element_json(&self, path: &str) -> PyResult<String> {
        to_json(self.inner.element(path).map_err(py_error)?)
    }

    fn move_schema_node(&mut self, path: &str, target_path: &str, after: bool) -> PyResult<String> {
        self.inner
            .move_schema_node(path, target_path, after)
            .map_err(py_error)
    }

    fn allowed_children_json(&self, parent_path: &str) -> PyResult<String> {
        to_json(&self.inner.allowed_children(parent_path).map_err(py_error)?)
    }

    /// Apply one edit (JSON); returns the inverse edit as JSON.
    fn apply_json(&mut self, edit_json: &str) -> PyResult<String> {
        let edit: Edit = serde_json::from_str(edit_json).map_err(json_error)?;
        to_json(&self.inner.apply(&edit).map_err(py_error)?)
    }

    /// Apply an edit batch (JSON) atomically; returns the inverse batch as JSON.
    fn apply_batch_json(&mut self, batch_json: &str) -> PyResult<String> {
        let batch: EditBatch = serde_json::from_str(batch_json).map_err(json_error)?;
        to_json(&self.inner.apply_batch(&batch).map_err(py_error)?)
    }

    fn diff_json(&self, other: &Self) -> PyResult<String> {
        to_json(&self.inner.diff(&other.inner).map_err(py_error)?)
    }

    fn __repr__(&self) -> String {
        format!(
            "Document(root='{}', use_cases={}, exchange_requirements={})",
            self.inner.root().local_name(),
            self.inner.count("uc"),
            self.inner.count("er")
        )
    }
}

#[pyfunction]
fn schema_catalog_json() -> PyResult<String> {
    to_json(cached_catalog().map_err(py_error)?)
}

#[pyfunction]
fn read_schema_text(schema_dir: &str, name: &str) -> PyResult<String> {
    schema_text(schema_dir, name).map_err(py_error)
}

#[pyfunction]
fn verify_schema_dir_json(schema_dir: &str) -> PyResult<String> {
    let report = verify_schema_dir(schema_dir).map_err(py_error)?;
    let mut value = serde_json::to_value(&report).map_err(json_error)?;
    value["ok"] = report.ok().into();
    Ok(value.to_string())
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyDocument>()?;
    module.add_function(wrap_pyfunction!(schema_catalog_json, module)?)?;
    module.add_function(wrap_pyfunction!(read_schema_text, module)?)?;
    module.add_function(wrap_pyfunction!(verify_schema_dir_json, module)?)?;
    let py = module.py();
    module.add("IdmError", py.get_type::<IdmError>())?;
    module.add("XmlError", py.get_type::<XmlError>())?;
    module.add("PathError", py.get_type::<PathError>())?;
    module.add("SchemaError", py.get_type::<SchemaError>())?;
    module.add("JsonError", py.get_type::<JsonError>())?;
    module.add("CardinalityError", py.get_type::<CardinalityError>())?;
    module.add("ContentModelError", py.get_type::<ContentModelError>())?;
    module.add("EncodingError", py.get_type::<EncodingError>())?;
    module.add("IdmIoError", py.get_type::<IdmIoError>())?;
    Ok(())
}

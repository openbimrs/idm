use crate::{Document, schema_catalog, schema_text};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn py_error(error: crate::Error) -> PyErr {
    PyValueError::new_err(error.to_string())
}

#[pyclass(name = "Document", module = "idmxml._native")]
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

    #[staticmethod]
    fn new(full_title: &str, idm_code: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Document::new_idm(full_title, idm_code).map_err(py_error)?,
        })
    }

    #[staticmethod]
    fn from_json(value: &str) -> PyResult<Self> {
        let value = serde_json::from_str(value)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(Self {
            inner: Document::from_value(&value).map_err(py_error)?,
        })
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

    #[pyo3(signature = (pretty=false))]
    fn to_json(&self, pretty: bool) -> PyResult<String> {
        let value = self.inner.to_value();
        if pretty {
            serde_json::to_string_pretty(&value)
        } else {
            serde_json::to_string(&value)
        }
        .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    fn validate_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.validate())
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    fn count(&self, name: &str) -> usize {
        self.inner.count(name)
    }

    fn element_paths(&self, name: &str) -> Vec<String> {
        self.inner.element_paths(name)
    }

    fn text(&self, path: &str) -> PyResult<String> {
        self.inner.text(path).map_err(py_error)
    }

    fn set_text(&mut self, path: &str, value: &str) -> PyResult<()> {
        self.inner.set_text(path, value).map_err(py_error)
    }

    fn attribute(&self, path: &str, name: &str) -> PyResult<String> {
        self.inner.attribute(path, name).map_err(py_error)
    }

    fn set_attribute(&mut self, path: &str, name: &str, value: &str) -> PyResult<()> {
        self.inner
            .set_attribute(path, name, value)
            .map_err(py_error)
    }

    fn append_schema_child(&mut self, parent_path: &str, name: &str) -> PyResult<String> {
        self.inner
            .append_schema_child(parent_path, name)
            .map_err(py_error)
    }

    fn remove_schema_node(&mut self, path: &str) -> PyResult<()> {
        self.inner.remove_schema_node(path).map_err(py_error)
    }

    fn move_schema_node(&mut self, path: &str, target_path: &str, after: bool) -> PyResult<String> {
        self.inner
            .move_schema_node(path, target_path, after)
            .map_err(py_error)
    }

    fn allowed_children_json(&self, parent_path: &str) -> PyResult<String> {
        let actions = self.inner.allowed_children(parent_path).map_err(py_error)?;
        serde_json::to_string(&actions).map_err(|error| PyValueError::new_err(error.to_string()))
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
    serde_json::to_string(&schema_catalog().map_err(py_error)?)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn read_schema_text(schema_dir: &str, name: &str) -> PyResult<String> {
    schema_text(schema_dir, name).map_err(py_error)
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyDocument>()?;
    module.add_function(wrap_pyfunction!(schema_catalog_json, module)?)?;
    module.add_function(wrap_pyfunction!(read_schema_text, module)?)?;
    Ok(())
}

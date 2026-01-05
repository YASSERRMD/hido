use pyo3::prelude::*;
use pyo3::types::{PyDict, PyBytes};
use crate::uail::{DIDManager, DIDConfig, DIDDocument};
use crate::uail::crypto::CryptoSuite;
use crate::uail::DIDKey;
use crate::icc::intent::{SemanticIntent, IntentDomain, IntentPriority};
use crate::audit::{AuditConfig, create_audit_backend, AuditBackend, AuditEntry, EntryId};
use crate::audit::backend::BackendType; // Import BackendType
use std::sync::Arc;
use tokio::runtime::Runtime;

// Helper runtime
fn runtime() -> Runtime {
    Runtime::new().unwrap()
}

// --- DID Bindings ---

#[pyclass(name = "DIDDocument")]
pub struct PyDIDDocument {
    inner: DIDDocument,
}

#[pymethods]
impl PyDIDDocument {
    #[getter]
    fn get_id(&self) -> String {
        self.inner.id.clone()
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
}

#[pyclass(name = "DIDManager")]
pub struct PyDIDManager {
    inner: DIDManager,
}

#[pymethods]
impl PyDIDManager {
    #[new]
    fn new() -> Self {
        PyDIDManager {
            inner: DIDManager::new(DIDConfig::default()),
        }
    }

    fn generate(&mut self) -> PyResult<String> {
        let did = runtime().block_on(self.inner.generate())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(did.id)
    }

    fn resolve(&self, did: String) -> PyResult<PyDIDDocument> {
        let doc = runtime().block_on(self.inner.resolve(&did))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyDIDDocument { inner: doc })
    }

    fn sign(&self, py: Python, did: String, message: Vec<u8>) -> PyResult<PyObject> {
        let signature = self.inner.sign(&did, &message)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyBytes::new(py, &signature).into())
    }

    fn verify(&self, did: String, message: Vec<u8>, signature: Vec<u8>) -> bool {
        self.inner.verify(&did, &message, &signature).is_ok()
    }
}

// --- Intent Bindings ---

#[pyclass(name = "Intent")]
#[derive(Clone)]
pub struct PyIntent {
    inner: SemanticIntent,
}

#[pymethods]
impl PyIntent {
    #[new]
    fn new(action: String, domain: Option<String>) -> Self {
        let crypto = CryptoSuite::new();
        let sender = DIDKey::new(&crypto);
        
        let domain_enum = match domain {
            Some(d) => IntentDomain::Custom(d),
            None => IntentDomain::Data,
        };

        PyIntent {
            inner: SemanticIntent::new(&sender, domain_enum, &action),
        }
    }

    #[getter]
    fn get_id(&self) -> String {
        self.inner.id.clone()
    }
    
    #[getter]
    fn get_action(&self) -> String {
        self.inner.action.clone()
    }

    fn set_target(&mut self, target: String) -> Self {
        self.inner.target = Some(target);
        self.clone()
    }

    fn set_priority(&mut self, priority: u8) -> Self {
        self.inner.priority = match priority {
            0 => IntentPriority::Low,
            1 => IntentPriority::Normal,
            2 => IntentPriority::High,
            _ => IntentPriority::Critical,
        };
        self.clone()
    }

    fn add_param(&mut self, key: String, value: String) -> Self {
        // Simple string params for now to avoid extensive PyObject conversion boilerplate
        self.inner.parameters.insert(key, serde_json::Value::String(value));
        self.clone()
    }

    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
}

// --- Audit Bindings ---

#[pyclass(name = "AuditBackend")]
pub struct PyAuditBackend {
    inner: Arc<dyn AuditBackend>,
}

#[pymethods]
impl PyAuditBackend {
    #[staticmethod]
    fn create_blockchain() -> PyResult<Self> {
        let config = AuditConfig::blockchain();
        let backend = runtime().block_on(create_audit_backend(&config))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(PyAuditBackend { inner: backend })
    }

    fn record(&self, actor: String, action: String, target: String) -> PyResult<String> {
        let entry = AuditEntry::new(&actor, &action, &target);
        let id = runtime().block_on(self.inner.record(entry))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(id.as_str().to_string())
    }
    
    fn backend_type(&self) -> String {
        self.inner.backend_type().to_string()
    }
}

// --- Module ---

#[pymodule]
fn hido(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyDIDManager>()?;
    m.add_class::<PyDIDDocument>()?;
    m.add_class::<PyIntent>()?;
    m.add_class::<PyAuditBackend>()?;
    Ok(())
}

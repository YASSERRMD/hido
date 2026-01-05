use pyo3::prelude::*;
use crate::uail::{DIDManager, DIDConfig};
use crate::uail::crypto::CryptoSuite;
use crate::uail::DIDKey;
use crate::icc::intent::{SemanticIntent, IntentDomain};

/// Python wrapper for DIDManager
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
        let did = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.inner.generate())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(did.id)
    }
}

/// Python wrapper for SemanticIntent
#[pyclass(name = "Intent")]
#[derive(Clone)]
pub struct PyIntent {
    inner: SemanticIntent,
}

#[pymethods]
impl PyIntent {
    #[new]
    fn new(action: String, domain: Option<String>) -> Self {
        // For basic example, we generate a temporary sender DID
        let crypto = CryptoSuite::new();
        let sender = DIDKey::new(&crypto);
        
        let domain_enum = match domain {
            Some(d) => IntentDomain::Custom(d),
            None => IntentDomain::Data, // Default to Data domain
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
}

/// Main HIDO Python module
#[pymodule]
fn hido(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyDIDManager>()?;
    m.add_class::<PyIntent>()?;
    Ok(())
}

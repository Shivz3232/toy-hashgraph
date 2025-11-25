use pyo3::prelude::*;
use pyo3::types::PyDict;

#[pyclass]
pub struct Hashgraph {
    inner: toy_hashgraph::Hashgraph,
}

#[pymethods]
impl Hashgraph {
    #[new]
    #[pyo3(signature = (id, timestamp, private_key, public_keys))]
    #[pyo3(
        text_signature = "(id: int, timestamp: int, private_key: bytes, public_keys: dict[int, bytes])"
    )]
    fn new(
        id: u64,
        timestamp: u64,
        private_key: &[u8],
        public_keys: &Bound<'_, PyDict>,
    ) -> PyResult<Self> {
        if private_key.len() != 32 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "private_key should be a bytes object of length 32",
            ));
        }
        let mut private_key_bound = [0u8; 32];
        private_key_bound.copy_from_slice(&private_key);

        let mut public_keys_vec = Vec::new();

        for (key, value) in public_keys.iter() {
            let key_str = key.extract::<u64>()?;
            let value_bytes: &[u8] = value.extract()?;

            if value_bytes.len() != 32 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "public_keys should be a dict with bytes of length 32 as values",
                ));
            }

            let mut buffer = [0; 32];
            buffer.copy_from_slice(value_bytes);
            public_keys_vec.push((key_str, buffer));
        }

        Ok(Hashgraph {
            inner: toy_hashgraph::Hashgraph::new(id, timestamp, private_key_bound, public_keys_vec),
        })
    }

    #[pyo3(text_signature = "(transaction: bytes) -> None")]
    fn append_transaction(&mut self, transaction: &[u8]) {
        self.inner.append_transaction(transaction);
    }

    #[pyo3(text_signature = "() -> bytes")]
    fn send(&mut self) -> Vec<u8> {
        self.inner.send()
    }

    #[pyo3(text_signature = "(data: bytes, timestamp: int) -> None")]
    fn recieve(&mut self, data: &[u8], timestamp: u64) {
        self.inner.recieve(data, timestamp);
    }

    #[pyo3(text_signature = "() -> str")]
    fn as_json(&self) -> String {
        self.inner.as_json()
    }
}

#[pymodule]
#[pyo3(name = "toy_hashgraph")]
fn toy_hashgraph_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Hashgraph>()?;
    Ok(())
}

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use toy_hashgraph::event::EventTrait;

/// A Hashgraph instance representing a single peer's view of the distributed ledger.
///
/// The Hashgraph data structure is used for Byzantine fault-tolerant consensus.
/// Each peer maintains their own Hashgraph instance and synchronizes with other
/// peers by exchanging events.
#[pyclass]
pub struct Hashgraph {
    inner: toy_hashgraph::Hashgraph,
}

#[pymethods]
impl Hashgraph {
    /// Create a new Hashgraph instance for a peer.
    ///
    /// Args:
    ///     id: The unique identifier for this peer.
    ///     timestamp: The initial timestamp (typically in milliseconds).
    ///     private_key: The Ed25519 private key for this peer (32 bytes).
    ///     public_keys: A dictionary mapping peer IDs to their Ed25519 public keys.
    ///
    /// Raises:
    ///     ValueError: If private_key is not 32 bytes.
    ///     ValueError: If any public key is not 32 bytes.
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

    /// The unique identifier of this peer.
    #[getter]
    fn id(&self) -> u64 {
        self.inner.id
    }

    /// The pending transactions that will be included in the next event.
    ///
    /// Transactions are accumulated via `append_transaction()` and are
    /// cleared when a new event is created during `receive()`.
    #[getter]
    fn pending_transactions(&self) -> Vec<u8> {
        self.inner.pending_transactions.clone()
    }

    /// The Ed25519 private key used for signing events (32 bytes).
    #[getter]
    fn signer(&self) -> Vec<u8> {
        self.inner.signer.to_bytes().to_vec()
    }

    /// A dictionary mapping peer IDs to their Ed25519 public keys (32 bytes each).
    #[getter]
    fn verifiers(&self) -> HashMap<u64, Vec<u8>> {
        self.inner
            .verifiers
            .iter()
            .map(|(id, key)| (*id, key.to_bytes().to_vec()))
            .collect()
    }

    /// Get a read-only querier for the underlying graph structure.
    ///
    /// The GraphQuerier provides methods to inspect the graph, check
    /// ancestry relationships, compute rounds, and more.
    #[getter]
    fn graph(slf: Py<Self>) -> GraphQuerier {
        GraphQuerier { parent: slf }
    }

    /// Append transaction data to the pending transactions buffer.
    ///
    /// The accumulated transactions will be included in the next event
    /// created during a `receive()` call.
    #[pyo3(text_signature = "(transaction: bytes) -> None")]
    fn append_transaction(&mut self, transaction: &[u8]) {
        self.inner.append_transaction(transaction);
    }

    /// Prepare a message to send to another peer.
    ///
    /// This serializes the current graph state along with a signature
    /// for verification by the receiving peer.
    ///
    /// Returns:
    ///     A bytes object containing the signed message to send.
    #[pyo3(text_signature = "() -> bytes")]
    fn send(&mut self) -> Vec<u8> {
        self.inner.send()
    }

    /// Receive and process a message from another peer.
    ///
    /// This updates the local graph with events from the sender and
    /// creates a new event referencing both the local latest event
    /// and the sender's latest event. Pending transactions are included.
    ///
    /// Args:
    ///     data: The signed message from another peer (from their `send()`).
    ///     timestamp: The current timestamp for the new event.
    #[pyo3(text_signature = "(data: bytes, timestamp: int) -> None")]
    fn receive(&mut self, data: &[u8], timestamp: u64) {
        self.inner.receive(data, timestamp);
    }

    /// Serialize the entire Hashgraph state to JSON.
    ///
    /// Returns:
    ///     A JSON string containing the id, pending transactions, and graph.
    #[pyo3(text_signature = "() -> str")]
    fn as_json(&self) -> String {
        self.inner.as_json()
    }
}

/// A read-only interface for querying the graph structure of a Hashgraph.
///
/// This class provides methods to inspect events, check ancestry relationships,
/// compute consensus rounds, and identify witnesses. All methods are non-mutating
/// and provide a view into the current state of the graph.
///
/// Obtain a GraphQuerier via the `Hashgraph.graph` property.
#[pyclass]
pub struct GraphQuerier {
    parent: Py<Hashgraph>,
}

#[pymethods]
impl GraphQuerier {
    /// Serialize all events in the graph to JSON.
    ///
    /// Returns:
    ///     A JSON object mapping event hashes (hex) to event data.
    #[pyo3(text_signature = "() -> str")]
    fn as_json(&self, py: Python<'_>) -> String {
        self.parent.borrow(py).inner.graph.as_json()
    }

    /// Check if a count represents a supermajority of peers.
    ///
    /// A supermajority is more than 2/3 of the total peers.
    ///
    /// Args:
    ///     count: The number to check.
    ///
    /// Returns:
    ///     True if count > (2/3 * total_peers).
    #[pyo3(text_signature = "(count: int) -> bool")]
    fn is_supermajority(&self, py: Python<'_>, count: u64) -> bool {
        self.parent.borrow(py).inner.graph.is_supermajority(count)
    }

    /// Serialize all events in the graph to bytes.
    #[pyo3(text_signature = "() -> bytes")]
    fn events_as_bytes(&self, py: Python<'_>) -> Vec<u8> {
        self.parent.borrow(py).inner.graph.events_as_bytes()
    }

    /// Get the hash of the latest event created by a specific peer.
    ///
    /// The latest event is the one with no descendants in the peer's chain.
    ///
    /// Args:
    ///     peer: The peer ID to query.
    ///
    /// Returns:
    ///     The event hash (32 bytes), or None if no events exist for this peer.
    #[pyo3(text_signature = "(peer: int) -> bytes | None")]
    fn latest_event(&self, py: Python<'_>, peer: u64) -> Option<Vec<u8>> {
        self.parent
            .borrow(py)
            .inner
            .graph
            .latest_event(peer)
            .map(|event| event.hash().to_vec())
    }

    /// Get an event by its hash.
    ///
    /// Args:
    ///     event_hash: The SHA-256 hash of the event (32 bytes).
    ///
    /// Returns:
    ///     The event as a JSON string.
    #[pyo3(text_signature = "(event_hash: bytes) -> str")]
    fn get_event(&self, py: Python<'_>, event_hash: &[u8]) -> PyResult<String> {
        let hash = extract_hash(event_hash)?;
        let binding = self.parent.borrow(py);
        let event = binding.inner.graph.get_event(&hash);
        Ok(event.as_json())
    }

    /// Get the peer ID of the creator of an event.
    ///
    /// This follows the self-parent chain back to the initial event.
    ///
    /// Args:
    ///     event_hash: The SHA-256 hash of the event (32 bytes).
    ///
    /// Returns:
    ///     The peer ID of the event's creator.
    #[pyo3(text_signature = "(event_hash: bytes) -> int")]
    fn creator(&self, py: Python<'_>, event_hash: &[u8]) -> PyResult<u64> {
        let hash = extract_hash(event_hash)?;
        Ok(self.parent.borrow(py).inner.graph.creator(&hash))
    }

    /// Check if event x is an ancestor of event y (x ≤ y).
    ///
    /// An event is an ancestor of another if there is a path through
    /// parent references. Every event is an ancestor of itself.
    ///
    /// Args:
    ///     x: Hash of the potential ancestor event (32 bytes).
    ///     y: Hash of the potential descendant event (32 bytes).
    ///
    /// Returns:
    ///     True if x is an ancestor of y (including x == y).
    #[pyo3(text_signature = "(x: bytes, y: bytes) -> bool")]
    fn is_ancestor(&self, py: Python<'_>, x: &[u8], y: &[u8]) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self
            .parent
            .borrow(py)
            .inner
            .graph
            .is_ancestor(&x_hash, &y_hash))
    }

    /// Check if event x is a strict ancestor of event y (x < y).
    ///
    /// Same as is_ancestor but excludes the case where x == y.
    #[pyo3(text_signature = "(x: bytes, y: bytes) -> bool")]
    fn is_strict_ancestor(&self, py: Python<'_>, x: &[u8], y: &[u8]) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self
            .parent
            .borrow(py)
            .inner
            .graph
            .is_strict_ancestor(&x_hash, &y_hash))
    }

    /// Check if event x is a self-ancestor of event y (x ⊑ y).
    ///
    /// A self-ancestor relationship follows only the self-parent chain,
    /// meaning both events were created by the same peer.
    #[pyo3(text_signature = "(x: bytes, y: bytes) -> bool")]
    fn is_self_ancestor(&self, py: Python<'_>, x: &[u8], y: &[u8]) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self
            .parent
            .borrow(py)
            .inner
            .graph
            .is_self_ancestor(&x_hash, &y_hash))
    }

    /// Check if event x is a strict self-ancestor of event y (x ⊏ y).
    ///
    /// Same as is_self_ancestor but excludes the case where x == y.
    #[pyo3(text_signature = "(x: bytes, y: bytes) -> bool")]
    fn is_strict_self_ancestor(&self, py: Python<'_>, x: &[u8], y: &[u8]) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self
            .parent
            .borrow(py)
            .inner
            .graph
            .is_strict_self_ancestor(&x_hash, &y_hash))
    }

    /// Check if two events represent a fork (Byzantine behavior).
    ///
    /// A fork occurs when two events from the same creator are not
    /// in a self-ancestor relationship with each other.
    #[pyo3(text_signature = "(x: bytes, y: bytes) -> bool")]
    fn is_fork(&self, py: Python<'_>, x: &[u8], y: &[u8]) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self.parent.borrow(py).inner.graph.is_fork(&x_hash, &y_hash))
    }

    /// Check if an event can see evidence of dishonesty by a peer.
    ///
    /// An event can see dishonesty if it has visibility to a fork
    /// created by the specified peer.
    #[pyo3(text_signature = "(event_hash: bytes, peer: int) -> bool")]
    fn can_see_dishonesty(&self, py: Python<'_>, event_hash: &[u8], peer: u64) -> PyResult<bool> {
        let hash = extract_hash(event_hash)?;
        Ok(self
            .parent
            .borrow(py)
            .inner
            .graph
            .can_see_dishonesty(&hash, peer))
    }

    /// Check if event y sees event x (x ⊴ y).
    ///
    /// Event y sees event x if x is an ancestor of y and y cannot
    /// see any dishonesty by the creator of x.
    #[pyo3(text_signature = "(x: bytes, y: bytes) -> bool")]
    fn sees(&self, py: Python<'_>, x: &[u8], y: &[u8]) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self.parent.borrow(py).inner.graph.sees(&x_hash, &y_hash))
    }

    /// Check if event y strongly sees event x (x ≪ y).
    ///
    /// Event y strongly sees event x if there is a supermajority of
    /// peers whose events are ancestors of y and see x.
    #[pyo3(text_signature = "(x: bytes, y: bytes) -> bool")]
    fn strongly_sees(&self, py: Python<'_>, x: &[u8], y: &[u8]) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self
            .parent
            .borrow(py)
            .inner
            .graph
            .strongly_sees(&x_hash, &y_hash))
    }

    /// Compute the round number of an event.
    ///
    /// Initial events are in round 0. Other events advance to round r+1
    /// if they can strongly see a supermajority of round r witnesses.
    #[pyo3(text_signature = "(event_hash: bytes) -> int")]
    fn round(&self, py: Python<'_>, event_hash: &[u8]) -> PyResult<u64> {
        let hash = extract_hash(event_hash)?;
        Ok(self.parent.borrow(py).inner.graph.round(&hash))
    }

    /// Get all witness events for a specific round.
    ///
    /// A witness is the first event by a peer in a given round.
    /// Initial events are witnesses for round 0.
    ///
    /// Returns:
    ///     A list of event hashes (32 bytes each).
    #[pyo3(text_signature = "(round: int) -> list[bytes]")]
    fn witnesses(&self, py: Python<'_>, round: u64) -> Vec<Vec<u8>> {
        self.parent
            .borrow(py)
            .inner
            .graph
            .witnesses(round)
            .into_iter()
            .map(|hash| hash.to_vec())
            .collect()
    }
}

fn extract_hash(bytes: &[u8]) -> PyResult<[u8; 32]> {
    if bytes.len() != 32 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "hash should be a bytes object of length 32",
        ));
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(bytes);
    Ok(hash)
}

#[pymodule]
#[pyo3(name = "toy_hashgraph")]
fn toy_hashgraph_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Hashgraph>()?;
    m.add_class::<GraphQuerier>()?;
    Ok(())
}

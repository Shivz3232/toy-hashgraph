use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

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
    ///
    /// Note: This returns a clone of the graph at the current point in time.
    /// Changes to the Hashgraph after calling this will not be reflected
    /// in the returned GraphQuerier.
    #[getter]
    fn graph(&self) -> GraphQuerier {
        GraphQuerier {
            inner: self.inner.graph.clone(),
        }
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

    /// Create a deep copy of this Hashgraph.
    ///
    /// Returns:
    ///     A new Hashgraph instance with the same state.
    #[pyo3(text_signature = "() -> Hashgraph")]
    fn clone(&self) -> Self {
        Hashgraph {
            inner: self.inner.clone(),
        }
    }
}

/// An interface for querying the graph structure of a Hashgraph.
///
/// This class provides methods to inspect events, check ancestry relationships,
/// compute consensus rounds, and identify witnesses. Methods that perform
/// computations will cache their results for improved performance.
///
/// Obtain a GraphQuerier via the `Hashgraph.graph` property, or construct one
/// from JSON using `GraphQuerier.from_json()`.
#[pyclass]
pub struct GraphQuerier {
    inner: toy_hashgraph::graph::Graph,
}

#[pymethods]
impl GraphQuerier {
    /// Create a GraphQuerier from a JSON string.
    ///
    /// Args:
    ///     json: A JSON string representing the graph (as produced by `as_json()`).
    ///
    /// Returns:
    ///     A new GraphQuerier instance.
    #[staticmethod]
    #[pyo3(text_signature = "(json: str) -> GraphQuerier")]
    fn from_json(json: &str) -> Self {
        GraphQuerier {
            inner: toy_hashgraph::graph::Graph::from_json(json),
        }
    }

    /// Serialize the graph to JSON.
    ///
    /// Returns:
    ///     A JSON object with the following structure:
    ///     - `total_peers`: The total number of peers in the network.
    ///     - `events`: An object mapping event hashes (hex) to event data.
    #[pyo3(text_signature = "() -> str")]
    fn as_json(&self) -> String {
        self.inner.as_json()
    }

    /// The total number of peers in the network.
    #[getter]
    fn total_peers(&self) -> usize {
        self.inner.total_peers
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
    fn is_supermajority(&self, count: usize) -> bool {
        self.inner.is_supermajority(count)
    }

    /// Serialize all events in the graph to bytes.
    #[pyo3(text_signature = "() -> bytes")]
    fn events_as_bytes(&self) -> Vec<u8> {
        self.inner.events_as_bytes()
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
    fn latest_event(&mut self, peer: u64) -> Option<Vec<u8>> {
        self.inner.latest_event(peer).map(|hash| hash.to_vec())
    }

    /// Get an event by its hash.
    ///
    /// Args:
    ///     event_hash: The SHA-256 hash of the event (32 bytes or 64-char hex string).
    ///
    /// Returns:
    ///     The event as a JSON string.
    #[pyo3(text_signature = "(event_hash: bytes | str) -> str")]
    fn get_event(&self, event_hash: &Bound<'_, PyAny>) -> PyResult<String> {
        let hash = extract_hash(event_hash)?;
        let event = self.inner.get_event(&hash);
        Ok(serde_json::to_string(event).expect("failed to serialize event"))
    }

    /// Get the peer ID of the creator of an event.
    ///
    /// This follows the self-parent chain back to the initial event.
    ///
    /// Args:
    ///     event_hash: The SHA-256 hash of the event (32 bytes or 64-char hex string).
    ///
    /// Returns:
    ///     The peer ID of the event's creator.
    #[pyo3(text_signature = "(event_hash: bytes | str) -> int")]
    fn creator(&mut self, event_hash: &Bound<'_, PyAny>) -> PyResult<u64> {
        let hash = extract_hash(event_hash)?;
        Ok(self.inner.creator(&hash))
    }

    /// Check if event x is an ancestor of event y (x ≤ y).
    ///
    /// An event is an ancestor of another if there is a path through
    /// parent references. Every event is an ancestor of itself.
    ///
    /// Args:
    ///     x: Hash of the potential ancestor event (32 bytes or 64-char hex string).
    ///     y: Hash of the potential descendant event (32 bytes or 64-char hex string).
    ///
    /// Returns:
    ///     True if x is an ancestor of y (including x == y).
    #[pyo3(text_signature = "(x: bytes | str, y: bytes | str) -> bool")]
    fn is_ancestor(&mut self, x: &Bound<'_, PyAny>, y: &Bound<'_, PyAny>) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self.inner.is_ancestor(&x_hash, &y_hash))
    }

    /// Check if event x is a strict ancestor of event y (x < y).
    ///
    /// Same as is_ancestor but excludes the case where x == y.
    #[pyo3(text_signature = "(x: bytes | str, y: bytes | str) -> bool")]
    fn is_strict_ancestor(&mut self, x: &Bound<'_, PyAny>, y: &Bound<'_, PyAny>) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self.inner.is_strict_ancestor(&x_hash, &y_hash))
    }

    /// Check if event x is a self-ancestor of event y (x ⊑ y).
    ///
    /// A self-ancestor relationship follows only the self-parent chain,
    /// meaning both events were created by the same peer.
    #[pyo3(text_signature = "(x: bytes | str, y: bytes | str) -> bool")]
    fn is_self_ancestor(&mut self, x: &Bound<'_, PyAny>, y: &Bound<'_, PyAny>) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self.inner.is_self_ancestor(&x_hash, &y_hash))
    }

    /// Check if event x is a strict self-ancestor of event y (x ⊏ y).
    ///
    /// Same as is_self_ancestor but excludes the case where x == y.
    #[pyo3(text_signature = "(x: bytes | str, y: bytes | str) -> bool")]
    fn is_strict_self_ancestor(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self.inner.is_strict_self_ancestor(&x_hash, &y_hash))
    }

    /// Check if two events represent a fork (Byzantine behavior).
    ///
    /// A fork occurs when two events from the same creator are not
    /// in a self-ancestor relationship with each other.
    #[pyo3(text_signature = "(x: bytes | str, y: bytes | str) -> bool")]
    fn is_fork(&mut self, x: &Bound<'_, PyAny>, y: &Bound<'_, PyAny>) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self.inner.is_fork(&x_hash, &y_hash))
    }

    /// Check if an event can see evidence of dishonesty by a peer.
    ///
    /// An event can see dishonesty if it has visibility to a fork
    /// created by the specified peer.
    #[pyo3(text_signature = "(event_hash: bytes | str, peer: int) -> bool")]
    fn can_see_dishonesty(&mut self, event_hash: &Bound<'_, PyAny>, peer: u64) -> PyResult<bool> {
        let hash = extract_hash(event_hash)?;
        Ok(self.inner.can_see_dishonesty(&hash, peer))
    }

    /// Check if event y sees event x (x ⊴ y).
    ///
    /// Event y sees event x if x is an ancestor of y and y cannot
    /// see any dishonesty by the creator of x.
    #[pyo3(text_signature = "(x: bytes | str, y: bytes | str) -> bool")]
    fn sees(&mut self, x: &Bound<'_, PyAny>, y: &Bound<'_, PyAny>) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self.inner.sees(&x_hash, &y_hash))
    }

    /// Check if event y strongly sees event x (x ≪ y).
    ///
    /// Event y strongly sees event x if there is a supermajority of
    /// peers whose events are ancestors of y and see x.
    #[pyo3(text_signature = "(x: bytes | str, y: bytes | str) -> bool")]
    fn strongly_sees(&mut self, x: &Bound<'_, PyAny>, y: &Bound<'_, PyAny>) -> PyResult<bool> {
        let x_hash = extract_hash(x)?;
        let y_hash = extract_hash(y)?;
        Ok(self.inner.strongly_sees(&x_hash, &y_hash))
    }

    /// Compute the round number of an event.
    ///
    /// Initial events are in round 0. Other events advance to round r+1
    /// if they can strongly see a supermajority of round r witnesses.
    #[pyo3(text_signature = "(event_hash: bytes | str) -> int")]
    fn round(&mut self, event_hash: &Bound<'_, PyAny>) -> PyResult<u64> {
        let hash = extract_hash(event_hash)?;
        Ok(self.inner.round(&hash))
    }

    /// Get all witness events for a specific round.
    ///
    /// A witness is the first event by a peer in a given round.
    /// Initial events are witnesses for round 0.
    ///
    /// Returns:
    ///     A list of event hashes (32 bytes each).
    #[pyo3(text_signature = "(round: int) -> list[bytes]")]
    fn witnesses(&mut self, round: u64) -> Vec<Vec<u8>> {
        self.inner
            .witnesses(round)
            .into_iter()
            .map(|hash| hash.to_vec())
            .collect()
    }

    /// Get all famous witness events for a specific round.
    ///
    /// A famous witness is a witness that has been decided as famous
    /// through the voting process.
    ///
    /// Returns:
    ///     A list of event hashes (32 bytes each).
    #[pyo3(text_signature = "(round: int) -> list[bytes]")]
    fn famous_witnesses(&mut self, round: u64) -> Vec<Vec<u8>> {
        self.inner
            .famous_witnesses(round)
            .into_iter()
            .map(|hash| hash.to_vec())
            .collect()
    }

    /// Get the unique famous witnesses for a specific round.
    ///
    /// When a peer has multiple famous witnesses (due to forks),
    /// only the minimum hash is kept per peer.
    ///
    /// Returns:
    ///     A list of event hashes (32 bytes each).
    #[pyo3(text_signature = "(round: int) -> list[bytes]")]
    fn unique_famous_witnesses(&mut self, round: u64) -> Vec<Vec<u8>> {
        self.inner
            .unique_famous_witnesses(round)
            .into_iter()
            .map(|hash| hash.to_vec())
            .collect()
    }

    /// Get the round in which an event was received (achieved consensus).
    ///
    /// An event is received in round r if all unique famous witnesses
    /// of round r are descendants of the event.
    ///
    /// Returns:
    ///     The round number, or None if consensus has not been reached.
    #[pyo3(text_signature = "(event_hash: bytes | str) -> int | None")]
    fn round_received(&mut self, event_hash: &Bound<'_, PyAny>) -> PyResult<Option<u64>> {
        let hash = extract_hash(event_hash)?;
        Ok(self.inner.round_received(&hash))
    }

    /// Get the consensus timestamp for an event.
    ///
    /// The consensus timestamp is the median of the timestamps when
    /// each unique famous witness first saw the event.
    ///
    /// Returns:
    ///     The consensus timestamp, or None if consensus has not been reached.
    #[pyo3(text_signature = "(event_hash: bytes | str) -> int | None")]
    fn consensus_timestamp(&mut self, event_hash: &Bound<'_, PyAny>) -> PyResult<Option<u64>> {
        let hash = extract_hash(event_hash)?;
        Ok(self.inner.consensus_timestamp(&hash))
    }

    /// Compare two events by their consensus ordering.
    ///
    /// Events are ordered by: round received, then consensus timestamp,
    /// then by their hash as a tiebreaker.
    ///
    /// Returns:
    ///     -1 if a < b, 0 if a == b, 1 if a > b, or None if either
    ///     event has not yet reached consensus.
    #[pyo3(text_signature = "(a: bytes | str, b: bytes | str) -> int | None")]
    fn consensus_ordering(
        &mut self,
        a: &Bound<'_, PyAny>,
        b: &Bound<'_, PyAny>,
    ) -> PyResult<Option<i8>> {
        let a_hash = extract_hash(a)?;
        let b_hash = extract_hash(b)?;
        Ok(self
            .inner
            .consensus_ordering(a_hash, b_hash)
            .map(|ordering| match ordering {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            }))
    }
}

fn extract_hash(value: &Bound<'_, PyAny>) -> PyResult<[u8; 32]> {
    // Try to extract as bytes first
    if let Ok(bytes) = value.extract::<Vec<u8>>() {
        if bytes.len() != 32 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "hash should be a bytes object of length 32",
            ));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        return Ok(hash);
    }

    // Try to extract as hex string
    if let Ok(hex_str) = value.extract::<String>() {
        // Strip optional "0x" prefix
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(&hex_str);

        if hex_str.len() != 64 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "hash hex string should be 64 characters (32 bytes)",
            ));
        }

        let mut hash = [0u8; 32];
        for (i, chunk) in hex_str.as_bytes().chunks(2).enumerate() {
            let hex_byte = std::str::from_utf8(chunk).map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid hex string")
            })?;
            hash[i] = u8::from_str_radix(hex_byte, 16).map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("invalid hex string")
            })?;
        }
        return Ok(hash);
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "hash should be bytes (32 bytes) or a hex string (64 characters)",
    ))
}

#[pymodule]
#[pyo3(name = "toy_hashgraph")]
fn toy_hashgraph_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Hashgraph>()?;
    m.add_class::<GraphQuerier>()?;
    Ok(())
}

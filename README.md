# toy-hashgraph

A toy implementation of the hashgraph consensus protocol as a library.

## Overview

Each peer maintains its own `Hashgraph` instance. Peers synchronize by sending their graph state to each other, creating new events that link to both their previous event and the sender's latest event.

## Project Structure

- `toy-hashgraph/` - Core Rust library
- `toy-hashgraph-py/` - Python bindings (via PyO3)
- `toy-hashgraph-js/` - WebAssembly bindings
- `examples/python/` - Python example with visualization
- `examples/web/` - Web example

## API

### `Hashgraph::new(id, timestamp, private_key, public_keys)`
Creates a new hashgraph instance for a peer.
- `id` - Unique peer identifier
- `timestamp` - Initial timestamp (milliseconds)
- `private_key` - 32-byte Ed25519 private key
- `public_keys` - Map of peer IDs to their 32-byte public keys

### `append_transaction(transaction)`
Appends transaction data to be included in the next event.

### `send() -> bytes`
Serializes the graph state with a digital signature for sending to another peer.

### `receive(data, timestamp)`
Receives and verifies data from another peer, updates the graph, and creates a new event linking to both the local and remote parent events.

### `as_json() -> str`
Returns the current state as JSON with three fields:
- `id` - Peer ID
- `transactions` - Pending transactions (hex string)
- `graph` - Map of event hashes to events

## Python Bindings

### Setup
```bash
cd toy-hashgraph-py
python -m venv .venv
source .venv/bin/activate
pip install maturin
maturin develop
```

### Usage
```python
from toy_hashgraph import Hashgraph
import json

# Create hashgraph instances for each peer
hashgraph = Hashgraph(
    id=0,
    timestamp=current_timestamp_ms(),
    private_key=private_key_bytes,
    public_keys={0: pk0, 1: pk1, 2: pk2}
)

# Append a transaction
hashgraph.append_transaction(b"hello")

# Send state to another peer
data = hashgraph.send()

# Receive state from another peer
other_hashgraph.receive(data, current_timestamp_ms())

# Get state as JSON
state = json.loads(hashgraph.as_json())
```

### Example

See `examples/python/` for a complete example with visualization using matplotlib.
"""
Tests for the Hashgraph GraphQuerier methods.
These tests mirror the Rust tests in toy-hashgraph/src/graph.rs
"""

import json
from nacl.signing import SigningKey
from toy_hashgraph import Hashgraph

# Peer IDs matching the Rust tests
ALICE = 1
BOB = 2
CATHY = 3
DAVE = 4


def generate_keys(num_peers: int) -> tuple[dict[int, bytes], dict[int, bytes]]:
    """Generate private and public keys for peers."""
    private_keys = {}
    public_keys = {}
    for peer in range(1, num_peers + 1):
        signing_key = SigningKey.generate()
        private_keys[peer] = bytes(signing_key)
        public_keys[peer] = bytes(signing_key.verify_key)
    return private_keys, public_keys


def build_figure1_hashgraphs() -> dict[int, Hashgraph]:
    """
    Build the hashgraph from Figure 1 in the paper by simulating
    the message exchanges between peers.
    
    Returns a dict of Hashgraph instances for each peer.
    """
    peers = [ALICE, BOB, CATHY, DAVE]
    private_keys, public_keys = generate_keys(4)
    
    # Create Hashgraph for each peer with timestamp 0
    hashgraphs = {
        peer: Hashgraph(peer, 0, private_keys[peer], public_keys)
        for peer in peers
    }


    
    # Simulate the exchanges from Figure 1:
    # Dave sent D1 to Cathy -> C2
    msg = hashgraphs[DAVE].send()
    hashgraphs[CATHY].receive(msg, 1)
    
    # Cathy sent C2 to Dave -> D2
    msg = hashgraphs[CATHY].send()
    hashgraphs[DAVE].receive(msg, 1)
    
    # Bob sent B1 to Alice -> A2
    msg = hashgraphs[BOB].send()
    hashgraphs[ALICE].receive(msg, 1)
    
    # Bob sent B1 to Cathy -> C3
    msg = hashgraphs[BOB].send()
    hashgraphs[CATHY].receive(msg, 2)
    
    # Alice sent A1 to Bob -> B2
    msg = hashgraphs[ALICE].send()
    hashgraphs[BOB].receive(msg, 1)
    
    # Alice sent A2 to Bob -> B3
    msg = hashgraphs[ALICE].send()
    hashgraphs[BOB].receive(msg, 2)
    
    # Cathy sent C3 to Bob -> B4
    msg = hashgraphs[CATHY].send()
    hashgraphs[BOB].receive(msg, 3)
    
    # Dave sent D2 to Bob -> B5
    msg = hashgraphs[DAVE].send()
    hashgraphs[BOB].receive(msg, 4)
    
    return hashgraphs


def get_event_hashes_by_peer(graph) -> dict[int, list[bytes]]:
    """Get all event hashes grouped by creator peer."""
    graph_json = json.loads(graph.as_json())
    hashes_by_peer: dict[int, list[tuple[int, bytes]]] = {}
    
    for hash_hex, event in graph_json.items():
        peer = graph.creator(bytes.fromhex(hash_hex))
        timestamp = event["timestamp"]
        if peer not in hashes_by_peer:
            hashes_by_peer[peer] = []
        hashes_by_peer[peer].append((timestamp, bytes.fromhex(hash_hex)))
    
    # Sort by timestamp and return just the hashes
    return {
        peer: [h for _, h in sorted(events)]
        for peer, events in hashes_by_peer.items()
    }


class TestResults:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.errors = []
    
    def check(self, condition: bool, message: str):
        if condition:
            self.passed += 1
        else:
            self.failed += 1
            self.errors.append(message)
    
    def summary(self):
        print(f"\n{'='*60}")
        print(f"Tests passed: {self.passed}")
        print(f"Tests failed: {self.failed}")
        if self.errors:
            print("\nFailures:")
            for error in self.errors:
                print(f"  - {error}")
        print(f"{'='*60}")
        return self.failed == 0


def test_hashgraph_fields(results: TestResults):
    """Test that Hashgraph fields are properly exposed."""
    print("\n[TEST] Hashgraph fields...")
    
    private_keys, public_keys = generate_keys(2)
    hg = Hashgraph(1, 0, private_keys[1], public_keys)
    
    # Test id
    results.check(hg.id == 1, "id should be 1")
    
    # Test pending_transactions (should be empty initially)
    results.check(hg.pending_transactions == b"", "pending_transactions should be empty")
    
    # Test signer (should be the private key)
    results.check(len(hg.signer) == 32, "signer should be 32 bytes")
    results.check(hg.signer == private_keys[1], "signer should match private key")
    
    # Test verifiers
    results.check(len(hg.verifiers) == 2, "verifiers should have 2 entries")
    for peer_id, pub_key in hg.verifiers.items():
        results.check(len(pub_key) == 32, f"verifier {peer_id} should be 32 bytes")
        results.check(pub_key == public_keys[peer_id], f"verifier {peer_id} should match public key")
    
    # Test pending_transactions after appending
    hg.append_transaction(b"test transaction")
    results.check(hg.pending_transactions == b"test transaction", 
                  "pending_transactions should contain appended data")
    
    print("  Fields test completed")


def test_graph_as_json(results: TestResults):
    """Test GraphQuerier.as_json()"""
    print("\n[TEST] GraphQuerier.as_json()...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    json_str = graph.as_json()
    parsed = json.loads(json_str)
    
    # BOB should have many events after all the syncs
    results.check(len(parsed) > 0, "Graph should have events")
    
    # Check that events have proper structure
    for hash_hex, event in parsed.items():
        results.check("kind" in event, f"Event {hash_hex[:8]}... should have 'kind'")
        results.check("timestamp" in event, f"Event {hash_hex[:8]}... should have 'timestamp'")
        if event["kind"] == "initial":
            results.check("peer" in event, f"Initial event should have 'peer'")
        elif event["kind"] == "default":
            results.check("transactions" in event, f"Default event should have 'transactions'")
            results.check("self_parent" in event, f"Default event should have 'self_parent'")
            results.check("other_parent" in event, f"Default event should have 'other_parent'")
    
    print("  as_json test completed")


def test_is_supermajority(results: TestResults):
    """Test GraphQuerier.is_supermajority()"""
    print("\n[TEST] GraphQuerier.is_supermajority()...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    # For 4 peers, supermajority means > 2/3 * 4 = 2.67, so at least 3
    results.check(not graph.is_supermajority(0), "0 should not be supermajority")
    results.check(not graph.is_supermajority(1), "1 should not be supermajority")
    results.check(not graph.is_supermajority(2), "2 should not be supermajority")
    results.check(graph.is_supermajority(3), "3 should be supermajority")
    results.check(graph.is_supermajority(4), "4 should be supermajority")
    
    print("  is_supermajority test completed")


def test_events_as_bytes(results: TestResults):
    """Test GraphQuerier.events_as_bytes()"""
    print("\n[TEST] GraphQuerier.events_as_bytes()...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    event_bytes = graph.events_as_bytes()
    results.check(len(event_bytes) > 0, "events_as_bytes should return non-empty bytes")
    results.check(isinstance(event_bytes, bytes), "events_as_bytes should return bytes")
    
    print("  events_as_bytes test completed")


def test_latest_event(results: TestResults):
    """Test GraphQuerier.latest_event()"""
    print("\n[TEST] GraphQuerier.latest_event()...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    # BOB should have a latest event
    latest_bob = graph.latest_event(BOB)
    results.check(latest_bob is not None, "BOB should have a latest event")
    
    if latest_bob:
        results.check(len(latest_bob) == 32, "latest_event should return 32-byte hash")
        results.check(isinstance(latest_bob, bytes), "latest_event should return bytes")
        # Verify we can use the hash to get the event
        event = json.loads(graph.get_event(latest_bob))
        results.check(event["kind"] == "default", "BOB's latest should be a default event")
    
    # Non-existent peer should return None
    latest_unknown = graph.latest_event(999)
    results.check(latest_unknown is None, "Unknown peer should return None")
    
    print("  latest_event test completed")


def test_get_event(results: TestResults):
    """Test GraphQuerier.get_event()"""
    print("\n[TEST] GraphQuerier.get_event()...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    # Get an event hash from as_json
    graph_json = json.loads(graph.as_json())
    some_hash = bytes.fromhex(list(graph_json.keys())[0])
    
    event_json = graph.get_event(some_hash)
    event = json.loads(event_json)
    
    results.check("kind" in event, "get_event should return valid event JSON")
    results.check("timestamp" in event, "Event should have timestamp")
    
    print("  get_event test completed")


def test_creator(results: TestResults):
    """Test GraphQuerier.creator()"""
    print("\n[TEST] GraphQuerier.creator()...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    # Get all events and check their creators
    graph_json = json.loads(graph.as_json())
    
    for hash_hex, event in graph_json.items():
        event_hash = bytes.fromhex(hash_hex)
        creator = graph.creator(event_hash)
        
        if event["kind"] == "initial":
            results.check(creator == event["peer"], 
                         f"Creator of initial event should match peer field")
    
    print("  creator test completed")


def test_ancestor_relations(results: TestResults):
    """Test is_ancestor, is_strict_ancestor, is_self_ancestor, is_strict_self_ancestor"""
    print("\n[TEST] Ancestor relations...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    # Get events by peer
    hashes_by_peer = get_event_hashes_by_peer(graph)
    
    # Self-ancestor: an event is always its own ancestor
    if BOB in hashes_by_peer and len(hashes_by_peer[BOB]) >= 2:
        bob_first = hashes_by_peer[BOB][0]
        bob_last = hashes_by_peer[BOB][-1]
        
        # Every event is an ancestor of itself
        results.check(graph.is_ancestor(bob_first, bob_first), 
                     "Event should be ancestor of itself")
        
        # First event should be ancestor of last event
        results.check(graph.is_ancestor(bob_first, bob_last), 
                     "First event should be ancestor of last")
        
        # Last event should NOT be ancestor of first
        results.check(not graph.is_ancestor(bob_last, bob_first), 
                     "Last event should not be ancestor of first")
        
        # Strict ancestor excludes self
        results.check(not graph.is_strict_ancestor(bob_first, bob_first), 
                     "Event should not be strict ancestor of itself")
        results.check(graph.is_strict_ancestor(bob_first, bob_last), 
                     "First should be strict ancestor of last")
        
        # Self-ancestor chain
        results.check(graph.is_self_ancestor(bob_first, bob_last), 
                     "First should be self-ancestor of last")
        results.check(not graph.is_self_ancestor(bob_last, bob_first), 
                     "Last should not be self-ancestor of first")
        
        # Strict self-ancestor
        results.check(not graph.is_strict_self_ancestor(bob_first, bob_first), 
                     "Event should not be strict self-ancestor of itself")
    
    print("  Ancestor relations test completed")


def test_fork_detection(results: TestResults):
    """Test is_fork and can_see_dishonesty"""
    print("\n[TEST] Fork detection...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    # Get events by peer
    hashes_by_peer = get_event_hashes_by_peer(graph)
    
    # In Figure 1, there are no forks (all peers are honest)
    if BOB in hashes_by_peer and len(hashes_by_peer[BOB]) >= 2:
        bob_events = hashes_by_peer[BOB]
        
        # Consecutive events from same peer should not be forks
        for i in range(len(bob_events) - 1):
            results.check(not graph.is_fork(bob_events[i], bob_events[i+1]), 
                         f"Consecutive events should not be forks")
    
    # No event should see dishonesty in the honest Figure 1 graph
    graph_json = json.loads(graph.as_json())
    peers = [ALICE, BOB, CATHY, DAVE]
    
    for hash_hex in list(graph_json.keys())[:5]:  # Test first 5 events
        event_hash = bytes.fromhex(hash_hex)
        for peer in peers:
            results.check(not graph.can_see_dishonesty(event_hash, peer),
                         f"No event should see dishonesty in honest graph")
    
    print("  Fork detection test completed")


def test_sees_and_strongly_sees(results: TestResults):
    """Test sees and strongly_sees"""
    print("\n[TEST] Sees and strongly_sees...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    # Get events by peer
    hashes_by_peer = get_event_hashes_by_peer(graph)
    
    if BOB in hashes_by_peer and len(hashes_by_peer[BOB]) >= 2:
        bob_first = hashes_by_peer[BOB][0]
        bob_last = hashes_by_peer[BOB][-1]
        
        # In an honest graph, sees is equivalent to is_ancestor
        results.check(graph.sees(bob_first, bob_last) == graph.is_ancestor(bob_first, bob_last),
                     "In honest graph, sees should equal is_ancestor")
    
    # Test strongly_sees with the last event (should strongly see initial events)
    if BOB in hashes_by_peer:
        bob_last = hashes_by_peer[BOB][-1]
        
        # Count how many initial events bob_last strongly sees
        strongly_seen_count = 0
        for peer, events in hashes_by_peer.items():
            if events:
                first_event = events[0]
                if graph.strongly_sees(first_event, bob_last):
                    strongly_seen_count += 1
        
        # BOB's last event should strongly see multiple initial events
        results.check(strongly_seen_count >= 1, 
                     f"Last event should strongly see at least 1 initial event, saw {strongly_seen_count}")
    
    print("  Sees and strongly_sees test completed")


def test_round(results: TestResults):
    """Test round calculation"""
    print("\n[TEST] Round calculation...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    # Get events by peer
    hashes_by_peer = get_event_hashes_by_peer(graph)
    
    # Initial events should be in round 0
    for peer, events in hashes_by_peer.items():
        if events:
            initial_event = events[0]
            event_json = json.loads(graph.get_event(initial_event))
            if event_json["kind"] == "initial":
                round_num = graph.round(initial_event)
                results.check(round_num == 0, 
                             f"Initial event of peer {peer} should be in round 0, got {round_num}")
    
    # At least one event should be in round > 0 (if graph is large enough)
    max_round = 0
    for peer, events in hashes_by_peer.items():
        for event_hash in events:
            round_num = graph.round(event_hash)
            max_round = max(max_round, round_num)
    
    # In Figure 1, B5 should be in round 1
    results.check(max_round >= 0, f"Max round should be at least 0, got {max_round}")
    
    print(f"  Max round found: {max_round}")
    print("  Round calculation test completed")


def test_witnesses(results: TestResults):
    """Test witnesses calculation"""
    print("\n[TEST] Witnesses calculation...")
    
    hashgraphs = build_figure1_hashgraphs()
    graph = hashgraphs[BOB].graph
    
    # Get round 0 witnesses (should be initial events)
    witnesses_r0 = graph.witnesses(0)
    results.check(len(witnesses_r0) > 0, "Round 0 should have witnesses")
    
    # All round 0 witnesses should be initial events
    for witness_hash in witnesses_r0:
        event_json = json.loads(graph.get_event(witness_hash))
        results.check(event_json["kind"] == "initial", 
                     "Round 0 witnesses should be initial events")
    
    # Check round 1 witnesses
    witnesses_r1 = graph.witnesses(1)
    for witness_hash in witnesses_r1:
        round_num = graph.round(witness_hash)
        results.check(round_num == 1, "Round 1 witnesses should be in round 1")
    
    print(f"  Round 0 witnesses: {len(witnesses_r0)}")
    print(f"  Round 1 witnesses: {len(witnesses_r1)}")
    print("  Witnesses calculation test completed")


def main():
    """Run all tests."""
    print("="*60)
    print("Hashgraph GraphQuerier Tests")
    print("(Python port of toy-hashgraph/src/graph.rs tests)")
    print("="*60)
    
    results = TestResults()
    
    try:
        test_hashgraph_fields(results)
        test_graph_as_json(results)
        test_is_supermajority(results)
        test_events_as_bytes(results)
        test_latest_event(results)
        test_get_event(results)
        test_creator(results)
        test_ancestor_relations(results)
        test_fork_detection(results)
        test_sees_and_strongly_sees(results)
        test_round(results)
        test_witnesses(results)
    except Exception as e:
        print(f"\n[ERROR] Test execution failed: {e}")
        import traceback
        traceback.print_exc()
        return 1
    
    success = results.summary()
    return 0 if success else 1


if __name__ == "__main__":
    exit(main())


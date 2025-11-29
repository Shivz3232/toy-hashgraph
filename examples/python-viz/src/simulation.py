"""Hashgraph simulation logic."""

import random
import time
from toy_hashgraph import Hashgraph
import config


def current_timestamp_ms() -> int:
    """Get current timestamp in milliseconds."""
    return int(time.time() * 1000)


def run_simulation(hashgraphs: dict, peers: list):
    """
    Run the hashgraph simulation.
    
    Args:
        hashgraphs: Dictionary mapping peer_id to Hashgraph instance
        peers: List of peer IDs
        
    Returns:
        List of simulation events for ground truth visualization
    """
    # Set random seed for reproducibility
    random.seed(config.RANDOM_SEED)
    
    print(f"Starting simulation with {len(peers)} peers for {config.SIMULATION_DURATION_SECONDS} seconds...")
    print(f"- Gossiping between 2 random peers every {config.GOSSIP_INTERVAL_SECONDS} seconds")
    print(f"- Randomly appending transactions every {config.TRANSACTION_INTERVAL_SECONDS} seconds")
    print(f"- Random seed: {config.RANDOM_SEED}")
    print()

    # Use simulated time for deterministic results
    simulated_time = 0.0
    base_timestamp_ms = current_timestamp_ms()  # Base timestamp for the hashgraph
    last_gossip_time = 0.0
    last_transaction_time = 0.0
    transaction_counter = 0
    
    # Track ground truth events
    simulation_events = []

    while simulated_time < config.SIMULATION_DURATION_SECONDS:
        # Every transaction interval, randomly decide whether to append a transaction
        if simulated_time - last_transaction_time >= config.TRANSACTION_INTERVAL_SECONDS:
            last_transaction_time = simulated_time
            if random.choice([True, False]):
                peer = random.choice(peers)
                transaction_counter += 1
                tx_data = f"tx_{transaction_counter}".encode()
                hashgraphs[peer].append_transaction(tx_data)
                print(f"[{simulated_time:.3f}s] Peer {peer} appended transaction: {tx_data.decode()}")
                
                simulation_events.append({
                    'type': 'transaction',
                    'peer': peer,
                    'transaction': tx_data.decode(),
                    'time': simulated_time
                })
        
        # Every gossip interval, choose 2 random peers to gossip
        if simulated_time - last_gossip_time >= config.GOSSIP_INTERVAL_SECONDS:
            last_gossip_time = simulated_time
            sender, receiver = random.sample(peers, 2)
            
            # Sender creates a message
            data = hashgraphs[sender].send()
            print(f"[{simulated_time:.3f}s] Peer {sender} -> Peer {receiver}: gossiping {len(data)} bytes")
            
            # Receiver processes the message with deterministic timestamp
            event_timestamp_ms = base_timestamp_ms + int(simulated_time * 1000)
            hashgraphs[receiver].receive(data, event_timestamp_ms)
            
            simulation_events.append({
                'type': 'gossip',
                'sender': sender,
                'receiver': receiver,
                'time': simulated_time
            })
        
        # Advance simulated time
        simulated_time += config.LOOP_SLEEP_SECONDS

    print()
    print("Simulation complete!")
    
    return simulation_events


"""Main entry point for the hashgraph simulation."""

from toy_hashgraph import Hashgraph
import config
import keys
import simulation
import visualization


def main():
    """Main function to run the hashgraph simulation."""
    # Generate keys for all peers
    peers = list(range(config.NUM_PEERS))
    private_keys, public_keys = keys.generate_keys(config.NUM_PEERS)
    
    # Use the same initial timestamp for all peers so initial events have consistent ordering
    initial_timestamp = simulation.current_timestamp_ms()
    
    # Create a Hashgraph instance for each peer
    hashgraphs = {
        peer: Hashgraph(peer, initial_timestamp, private_keys[peer], public_keys)
        for peer in peers
    }
    
    # Run the simulation and get ground truth events
    simulation_events = simulation.run_simulation(hashgraphs, peers)
    
    # Visualize the results with ground truth
    visualization.visualize_hashgraphs(hashgraphs, peers, simulation_events)


if __name__ == "__main__":
    main()


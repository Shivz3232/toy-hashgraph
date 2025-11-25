"""Configuration constants for the hashgraph simulation."""

# Simulation parameters
NUM_PEERS = 4
SIMULATION_DURATION_SECONDS = 1
GOSSIP_INTERVAL_SECONDS = 0.1
TRANSACTION_INTERVAL_SECONDS = 0.03
LOOP_SLEEP_SECONDS = 0.005
RANDOM_SEED = 0  # For reproducible simulation

# Visualization
OUTPUT_DIR = "images"
PEER_VIEWS_FILENAME = "images/hashgraph_peer_views.png"
TIMELINE_FILENAME = "images/hashgraph_timeline.png"
MERGED_GRAPH_FILENAME = "images/hashgraph_merged.png"
DPI = 150


"""Hashgraph visualization utilities."""

import json
import matplotlib.pyplot as plt
import config


def get_event_peer(graph: dict, event_hash: str) -> int:
    """
    Trace back through self_parent to find the peer that owns this event.
    
    Args:
        graph: The hashgraph graph dictionary
        event_hash: Hash of the event to trace
        
    Returns:
        Peer ID that owns the event
    """
    current_hash = event_hash
    while True:
        event = graph[current_hash]
        if event["kind"] == "initial":
            return event["peer"]
        else:
            current_hash = event["self_parent"]


def plot_hashgraph(state: dict, title: str, ax):
    """
    Plot a hashgraph on the given axes.
    
    Args:
        state: Hashgraph state dictionary
        title: Title for the plot
        ax: Matplotlib axes to plot on
    """
    graph = state["graph"]
    
    # Get all unique peers by looking at initial events
    peer_ids = sorted(set(
        event["peer"] for event in graph.values() if event["kind"] == "initial"
    ))
    peer_names = [f"Peer {p}" for p in peer_ids]
    peer_x = {peer: i * 2 for i, peer in enumerate(peer_ids)}
    
    # Sort events by timestamp, then by peer ID for consistent ordering
    events_sorted = sorted(
        [(h, e) for h, e in graph.items()],
        key=lambda x: (x[1]["timestamp"], get_event_peer(graph, x[0]))
    )
    
    # Assign y positions based on timestamp order
    event_positions = {}
    for y_pos, (event_hash, event) in enumerate(events_sorted):
        peer = get_event_peer(graph, event_hash)
        x = peer_x[peer]
        event_positions[event_hash] = (x, y_pos)
    
    # Draw peer lanes (dashed vertical lines)
    max_y = len(events_sorted)
    for peer, x in peer_x.items():
        ax.axvline(x, color='gray', linestyle='--', alpha=0.3, zorder=0)
    
    # Draw edges first (so they're behind nodes)
    for event_hash, event in graph.items():
        if event["kind"] == "default":
            x1, y1 = event_positions[event_hash]
            
            # Self parent edge
            if event["self_parent"] in event_positions:
                x2, y2 = event_positions[event["self_parent"]]
                ax.plot([x1, x2], [y1, y2], 'k-', linewidth=1, zorder=1)
            
            # Other parent edge
            if event["other_parent"] in event_positions:
                x2, y2 = event_positions[event["other_parent"]]
                ax.plot([x1, x2], [y1, y2], 'k-', linewidth=1, zorder=1)
    
    # Draw events as circles
    for event_hash, (x, y) in event_positions.items():
        event = graph[event_hash]
        color = 'lightblue' if event["kind"] == "initial" else 'white'
        circle = plt.Circle((x, y), 0.3, fill=True, facecolor=color, 
                            edgecolor='black', linewidth=1.5, zorder=2)
        ax.add_patch(circle)
        
        # Label with short hash
        short_hash = event_hash[:4]
        ax.text(x, y, short_hash, ha='center', va='center', fontsize=7, zorder=4)
    
    # Add peer labels at top
    for peer, x in peer_x.items():
        ax.text(x, max_y + 0.5, f"Peer {peer}", ha='center', va='bottom', fontsize=10, fontweight='bold')
    
    ax.set_xlim(-1, max(peer_x.values()) + 1)
    ax.set_ylim(-1, max_y + 1.5)
    ax.set_aspect('equal')
    ax.axis('off')
    ax.set_title(title, fontsize=12, fontweight='bold', pad=10)


def plot_ground_truth(simulation_events: list, peers: list, ax):
    """
    Plot the ground truth simulation timeline.
    
    Args:
        simulation_events: List of simulation events
        peers: List of peer IDs
        ax: Matplotlib axes to plot on
    """
    # Spacing between peers
    peer_x = {peer: i * 1.5 for i, peer in enumerate(peers)}
    
    # Calculate time range
    max_time = max(e['time'] for e in simulation_events) if simulation_events else 1
    
    # Group transactions that are very close in time on the same peer
    transaction_positions = {}
    spacing_threshold = 0.02  # 20ms
    
    for event in simulation_events:
        if event['type'] == 'transaction':
            peer = event['peer']
            time = event['time']
            
            # Check if there's a nearby transaction on the same peer
            adjusted_time = time
            if peer in transaction_positions:
                for existing_time in transaction_positions[peer]:
                    if abs(time - existing_time) < spacing_threshold:
                        # Offset this transaction slightly
                        adjusted_time = existing_time + spacing_threshold
                        break
            
            if peer not in transaction_positions:
                transaction_positions[peer] = []
            transaction_positions[peer].append(adjusted_time)
            event['adjusted_time'] = adjusted_time
    
    # Draw peer lanes
    for peer, x in peer_x.items():
        ax.axvline(x, color='gray', linestyle='--', alpha=0.3, linewidth=1, zorder=0)
    
    # Plot gossip arrows first (so they're behind transactions)
    for event in simulation_events:
        if event['type'] == 'gossip':
            sender = event['sender']
            receiver = event['receiver']
            x1, x2 = peer_x[sender], peer_x[receiver]
            y = event['time']
            
            # Draw gossip as an arrow
            ax.annotate('', xy=(x2, y), xytext=(x1, y),
                       arrowprops=dict(arrowstyle='->', lw=1.5, color='royalblue', 
                                     alpha=0.5, shrinkA=0, shrinkB=0),
                       zorder=1)
    
    # Plot transactions on top
    for event in simulation_events:
        if event['type'] == 'transaction':
            peer = event['peer']
            x = peer_x[peer]
            y = event.get('adjusted_time', event['time'])
            
            # Draw transaction as a scatter point (guaranteed to be circular)
            ax.scatter(x, y, s=300, c='orange', edgecolors='black', 
                      linewidths=1.5, zorder=3, marker='o')
            
            # Extract transaction number
            tx_num = event['transaction'].replace('tx_', '')
            
            # Label with transaction number inside the circle
            ax.text(x, y, tx_num, ha='center', va='center', 
                   fontsize=7, fontweight='bold', zorder=4)
    
    # Add peer labels at bottom
    for peer, x in peer_x.items():
        ax.text(x, max_time + 0.08, f"Peer {peer}", ha='center', va='bottom', 
               fontsize=11, fontweight='bold')
    
    # Set axis properties
    ax.set_xlim(-0.5, max(peer_x.values()) + 0.5)
    ax.set_ylim(-0.1, max_time + 0.2)
    ax.set_ylabel('Time (seconds)', fontsize=11)
    ax.set_title('Ground Truth: Simulation Timeline', fontsize=13, fontweight='bold', pad=15)
    ax.grid(True, axis='y', alpha=0.2, linestyle='-', linewidth=0.5)
    ax.set_xticks([])  # Remove x-axis ticks
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    ax.spines['bottom'].set_visible(False)


def visualize_hashgraphs(hashgraphs: dict, peers: list, simulation_events: list = None):
    """
    Create and save visualization of all peer hashgraphs.
    
    Args:
        hashgraphs: Dictionary mapping peer_id to Hashgraph instance
        peers: List of peer IDs
        simulation_events: Optional list of simulation events for ground truth plot
    """
    # Save timeline separately if available
    if simulation_events:
        fig_timeline = plt.figure(figsize=(8, 6))
        ax_timeline = fig_timeline.add_subplot(111)
        plot_ground_truth(simulation_events, peers, ax_timeline)
        plt.tight_layout()
        plt.savefig(config.TIMELINE_FILENAME, dpi=config.DPI, bbox_inches='tight')
        print(f"Saved timeline to {config.TIMELINE_FILENAME}")
        plt.close(fig_timeline)
    
    # Create a 2x2 grid for peer views
    num_peers = len(peers)
    rows = 2
    cols = 2
    
    fig, axes = plt.subplots(rows, cols, figsize=(12, 12))
    axes = axes.flatten()  # Flatten to 1D array for easy indexing

    # Plot each peer's view
    for i, peer in enumerate(peers):
        if i < len(axes):
            state = json.loads(hashgraphs[peer].as_json())
            plot_hashgraph(state, f"Peer {peer}'s View ({len(state['graph'])} events)", axes[i])
    
    # Hide any unused subplots
    for i in range(num_peers, len(axes)):
        axes[i].axis('off')

    plt.tight_layout()
    plt.savefig(config.PEER_VIEWS_FILENAME, dpi=config.DPI, bbox_inches='tight')
    print(f"Saved peer views to {config.PEER_VIEWS_FILENAME}")
    plt.close(fig)


"""Key generation and management utilities."""

from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization


def generate_keys(num_peers: int):
    """
    Generate Ed25519 key pairs for each peer.
    
    Args:
        num_peers: Number of peers to generate keys for
        
    Returns:
        tuple: (private_keys, public_keys) dictionaries mapping peer_id to bytes
    """
    peers = list(range(num_peers))
    keys = {
        peer: ed25519.Ed25519PrivateKey.generate()
        for peer in peers
    }

    private_keys = {
        peer: key.private_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PrivateFormat.Raw,
            encryption_algorithm=serialization.NoEncryption()
        )
        for peer, key in keys.items()
    }
    
    public_keys = {
        peer: key.public_key().public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw
        )
        for peer, key in keys.items()
    }

    # Validate key sizes
    assert all(len(v) == 32 for v in private_keys.values())
    assert all(len(v) == 32 for v in public_keys.values())
    
    return private_keys, public_keys

